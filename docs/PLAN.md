# Iqal — Build Plan

Open-source (GPL-3.0) website and app blocker for **Linux Mint** (first) and **Windows 11** (second).
Native only: Rust core, GTK4 UI on Linux, raw Win32 UI on Windows. No web UI.

---

## 1. Architecture

```
┌──────────────┐   Unix socket    ┌─────────────────────────────┐
│ iqal (GTK) │◄────────────────►│ iqal-daemon  (root,       │
│ iqal-cli   │   (peer-cred     │   systemd, Rust)            │
└──────────────┘    checked)      │  • rule engine (core crate) │
                                  │  • locks + tamper guard     │
┌──────────────┐                  │  • fanotify exec blocker    │
│ iqal-agent │◄────────────────►│  • DNS filter + nftables    │
│ (per user,   │                  │  • location monitor         │
│  X11 titles, │                  │  • stats, list updater      │
│  block page) │                  └─────────────▲───────────────┘
└──────────────┘                                │ native messaging
                                  ┌─────────────┴───────────────┐
                                  │ Browser extension (MV3)     │
                                  │ Chrome/Chromium/Brave/Edge/ │
                                  │ Vivaldi/Opera/Firefox       │
                                  │ — thin: forwards URLs, the  │
                                  │   daemon decides            │
                                  └─────────────────────────────┘
```

The UI is only an editor. Killing it changes nothing. All enforcement is in the daemon.

### Cargo workspace

| Crate | Purpose | Platform |
|---|---|---|
| `iqal-core` | Rule model, matching engine (domain trie, Aho-Corasick for URL/keyword/title), locks, schedules, allowances, profiles, import/export format | any |
| `iqal-ipc` | Message types + framed transport for UI/CLI/agent/extension ↔ daemon | any |
| `iqal-i18n` | Fluent translations, locale detection, RTL flag | any |
| `iqal-daemon` | The privileged service (`linux/`, later `windows/` modules) | per OS |
| `iqal-agent` | Per-session agent: window titles, block overlay, notifications | per OS |
| `iqal-gtk` | Linux GUI (GTK4, libshumate map) | Linux |
| `iqal-win32` | Windows GUI (later) | Windows |
| `iqal-cli` | Command-line tool | any |
| `extension/` | WebExtension (one codebase, Chromium + Firefox manifests) | — |
| `lists/` | Pre-made lists + update manifest | — |
| `locales/` | `.ftl` translation files (en, ar, fr, es, de, tr, ur, id, ...) | — |
| `packaging/` | `.deb` for Linux Mint 22.x (later MSI for Windows) | — |

---

## 2. How each feature is enforced on Linux Mint

**Note:** Mint's default desktops (Cinnamon, MATE, Xfce) run on **X11**. Mint 22's Wayland session is experimental. So the X11 agent covers real-world Mint. The GNOME/KDE Wayland helpers are kept for later (other distros).

| Feature | Mechanism |
|---|---|
| Block apps by exe | `fanotify` `FAN_OPEN_EXEC_PERM` on every mounted filesystem. The kernel asks the daemon *before* a program runs, so a denied app never starts. Matching uses path, file name, and SHA-256 (so renaming doesn't bypass). Interpreted apps (python, java, electron, AppImage) are also matched by command line. |
| Block apps by window title | The per-user agent watches `_NET_ACTIVE_WINDOW` / `_NET_WM_NAME` via X11 events (no polling). On a match it closes the window, then kills it via `_NET_WM_PID` if needed. |
| Apps whitelist / blacklist / allow-only mode | Same engine. Allow-only = deny any exec not on the list (system paths always allowed so the desktop keeps working). |
| All users / all except / only these users | The UID comes from the process (fanotify pid → `/proc`) and from the socket UID for DNS. Each rule set has a user scope. |
| Block domains (all apps) | The daemon runs a local DNS filter. nftables redirects every user's port-53 traffic, **per UID**, to the right filter instance. Blocked names get answers pointing at the local block page. |
| Stop DNS bypasses | nftables blocks known DoH/DoT endpoints (IP set + port 853). Browser policies turn off built-in DoH (`DnsOverHttpsMode=off`, Firefox `DNSOverHTTPS.Enabled=false, Locked`). Optional: block VPN/proxy apps. |
| URL contains / search term contains | The browser extension sends each navigation + search query to the daemon over native messaging and gets back allow/block. |
| Extension can't be removed | Force-installed through managed policies (`/etc/opt/chrome/policies/managed`, `/etc/chromium/...`, `/etc/brave/...`, Firefox `policies.json`), and allowed in incognito. Any browser running **without** a live extension connection gets closed (heartbeat check). |
| Forced SafeSearch | DNS answers CNAME to `forcesafesearch.google.com`, `strict.bing.com`, `safe.duckduckgo.com`, `restrict.youtube.com`, plus the extension appends safe parameters. |
| Website exceptions | Allow rules beat block rules at the same or more specific level (e.g. block `youtube.com`, allow `youtube.com/watch?v=…` lecture). |
| Pre-made lists | Social media, adult, gambling, search engines, news, (+ games, shopping, video). Bundled, signed (minisign), and updated automatically from upstream open-source lists. |
| Import from text file | One domain/URL/keyword per line. `#` comments. hosts-file and Adblock syntax recognised. |
| Export / import blocks | `.iqal` file (versioned JSON, optionally signed). Importing can **never** weaken a locked block. |

### Locks

| Lock | Behaviour |
|---|---|
| Time | Locked for a duration / until a date-time. |
| Date range & schedule | Weekly grid (per weekday, per hour range) and date ranges. |
| Password | Password stored as Argon2id hash. Option: "friend-held" = generated, shown once, never stored in plain text. |
| Random text | Type N random characters (no paste) to unlock. |
| Delayed unlock | Unlock takes effect only after a chosen delay. |
| Restart | Can only be removed right after a reboot. |
| **Location** | User draws a circle/polygon on a map (libshumate, OpenStreetMap). Locked while **inside** the area. |

**Clock tampering:** lock deadlines are tracked with `CLOCK_BOOTTIME` plus periodic NTP verification. Moving the system clock forward doesn't end a lock.

**Location lock details**
- **Learned Wi-Fi networks (main source).** "I am at this place now" (`iqal-cli learn-location`, later a GUI button) remembers every network in range, neighbours' included. It's also learned automatically while a precise position says you're there. Learning only ever adds networks.
  - Any remembered network in range → **inside**.
  - Wi-Fi working, at least 2 networks in range, none remembered → **away**.
- **OS location service (GeoClue2).** The service registers its own GeoClue agent (root has none) and points GeoClue's Wi-Fi lookup at BeaconDB, the open successor of Mozilla's retired service. Only positions better than 1 km count; IP-based guesses (fakeable with a VPN) are ignored. BeaconDB's coverage is still sparse (it had no data for the test PC in Norway), so this is a secondary source.
- **Fail-closed:** no evidence (Wi-Fi off, too few networks, no position) → treated as **inside**. Turning things off therefore never unlocks anything. A desktop without Wi-Fi can only use GeoClue.
- While a location lock holds, polkit denies non-root users switching Wi-Fi or networking off.
- Privacy: nothing is looked up while no location lock exists. Remembered network IDs stay on the machine; only GeoClue's lookup sends network IDs to BeaconDB.

### Extras (all in v1)
Allowance (X min/day per site/app, across browsers) · delayed unlock · random-text lock · allow-only mode · forced SafeSearch · profiles (each with own schedule + lock) · local-only statistics · auto-updating lists · CLI.

### Tamper resistance (Linux)
- systemd unit with `Restart=always` and a watchdog. The agent re-launches itself via XDG autostart + a systemd user unit.
- Config in `/var/lib/iqal` (root 0600). The UI only edits through the daemon, which refuses to weaken locked rules.
- The daemon re-asserts its nftables rules, policy files and polkit rules if they're removed.
- Package removal blocked while a lock is active (dpkg pre-remove hook refuses).
- **Honest limit:** a user with `sudo` can still defeat any Linux blocker. Optional "hardened lock" mode: while a lock is active, the daemon removes the locked user's sudo rights (with an emergency recovery path documented), turning "trivial" into "requires booting another OS".

---

## 3. Internationalisation
- Fluent (`fluent-rs`) shared by both OSes. Correct Arabic plural rules.
- RTL: GTK `Widget::set_default_direction(Rtl)` for ar/fa/ur/he. Win32 later uses `WS_EX_LAYOUTRTL`.
- Starting set: English, Arabic, French, Spanish, German, Turkish, Urdu, Indonesian, Portuguese, Russian, Chinese (Simplified). Machine-assisted first drafts, open for community fixes.
- The browser extension's block page uses the same strings (generated from the `.ftl` files).

---

## 4. Milestones (Linux first)

| # | Milestone | Done when |
|---|---|---|
| M0 | Workspace, `iqal-core` (rules, matcher, schedules, locks logic, import/export), unit tests, CI | `cargo test` green; matcher benchmarks |
| M1 | Daemon skeleton, IPC, CLI, config storage, systemd unit | `iqal-cli` can add/list rules on Mint |
| M2 | App blocking: fanotify exec, user scope, allow-only | Blocked app can't launch |
| M3 | X11 agent: window-title blocking, block overlay | Matching window closes instantly |
| M4 | DNS filter + per-UID nftables + DoH lockdown + SafeSearch | Blocked domains fail in every app |
| M5 | Browser extension + native messaging + force-install policies + "browser without extension" enforcement | URL/keyword blocking works in Firefox + Chromium family |
| M6 | Locks: time, schedule, password, random text, delayed, restart, location (GeoClue + Wi-Fi + polkit) + clock-tamper protection | Locked rules can't be weakened |
| M7 | GTK4 UI incl. map picker, i18n, RTL | Full feature set usable without CLI |
| M8 | Profiles, allowances, statistics, pre-made lists + updater, text import | — |
| M9 | `.deb` package for Mint 22.x, install/uninstall protection | Clean install on fresh Mint VM |
| — | **Windows 11 port** (see below) | — |

### Windows 11 milestones

Built on the Windows PC (Rust + Visual Studio C++ tools), tested in a Windows 11 virtual machine. Programs are built with the C runtime linked in (`.cargo/config.toml`), so they need no Visual C++ Redistributable.

| # | Milestone | Done when |
|---|---|---|
| W1 | Service (`iqal-daemon` as a Windows service, LocalSystem, auto-start, restart on failure), named-pipe IPC (`\\.\pipe\iqal`, caller identified from the pipe client's token; Administrators members count as admins), protected settings folder `C:\ProgramData\Iqal`, CLI | `iqal-cli` can add/list/lock rules in the VM; locks survive service restarts ✅ |
| W2 | App blocking: process starts from an ETW real-time session (Kernel-Process) plus a 2 s sweep; blocked processes are ended (~0.1 s after start). Window titles: the service starts a watcher (`iqal-daemon --watch-session`, LocalSystem, in a kill-on-close job) on every signed-in desktop; it reports windows over the pipe and closes those named (WM_CLOSE, then ending the program after 1.5 s, never for essential programs). Allow-only restricts shortcut apps, Store apps, games and programs outside Windows/Program Files; shells and Windows' own parts are exempt | Blocked app closes the moment it opens ✅ |
| W3 | Website blocking. Done: DNS filter on 127.0.0.1:53 (decisions shared with Linux in `dnsfilter.rs`), all lookups sent there by an NRPT rule for "." (removed when the service stops; adapters untouched), rules of the active session's user, cache flushed on changes, SafeSearch, DoH hosts blocked; browser policies in the registry (DoH off, Chromium's built-in DNS client off, no private/guest windows while website rules exist). Extension (shared `extension.rs`): native host manifests next to the program, registered in HKLM for Firefox and the Chromium family; Firefox force-installs the signed XPI via the ExtensionSettings policy (private windows included); the browser is recognised from the process tree; unsupported browsers are closed while website rules apply; supported browsers without a connected extension are closed after 30 s once the extension was seen working in them. Chrome, Brave and Vivaldi force-install the Chrome Web Store version (id lifonkjgpgnkaomfcgcagijniopcecmb), Edge the Edge Add-ons version (id nbmdjdnceeaalacfkbbnlnmbobfdffec), via ExtensionInstallForcelist (entries of others kept); both submitted for review 2026-10-20. Open: firewall rules against programs asking outside DNS servers directly; testing Chrome/Edge once the stores approve | Blocked sites fail in every browser and app |
| W4 | Locks on Windows. Restart lock via Windows' boot counter (BootId); clock changes ignored (network time + GetTickCount64). Place lock: learned Wi-Fi networks (logic shared with Linux in `places.rs`) from the WLAN API; Windows lists nearby networks only while its Location services switch is on, so learning a place asks for it (locks still fail closed without it). While any block is locked the service does not accept Stop and its permissions deny administrators stop, reconfigure, delete and permission changes; normal permissions return when the last lock ends. Honest limit: an administrator can still end the process (Windows restarts it in 1 s) or take ownership of the service | Locked blocks cannot be weakened ✅ |
| W5 | Win32 GUI (`iqal-win32`, shared app logic in `iqal-app`), i18n, RTL (`WS_EX_LAYOUTRTL`), per-monitor DPI. Owner's decisions: on Windows everyone may change blocks that are not locked (administrators alone see others' activity and stored lock passwords); the place lock is chosen by name, without a map. W5a main window ✅, W5b block editor (Websites, Apps, People, Times, save bar) ✅, W5c Lock tab, lock and unlock dialogs, place lock by name (`GeoArea::Named`) ✅, W5d New block, menu (import, export, recent activity, settings, about) ✅, W5e Statistics window (cards, blocks per day, ranked lists with "Show all") ✅, W5f app picker (Start menu shortcuts with icons), Windows app examples, translated clock source, named-place lock text ✅ | Full feature set usable without the CLI |
| W6 | Session agent: block notices, notifications. Done: Iqal icon in the notification area whose balloons Windows shows as notifications (named "Iqal" through the app identity Iqal.Iqal, registered with the icon by `iqal-daemon --install`), one agent per session, started at sign-in (HKLM Run), clicking the icon or a notification opens Iqal ✅ | Same notices as on Mint |
| W7 | Installer, uninstall protection, code signing (Smart App Control blocks unsigned programs). Owner's choices: Iqal's own setup program (not .msi), free signing through SignPath Foundation. W7a done: `iqal-setup.exe` (built by packagingwindowsbuild-setup.ps1) carries all programs, installs or updates into Program FilesIqal, adds the Start menu shortcut and the Settings > Apps entry, and removes everything but the blocks (kept in ProgramData). Updating and removing are refused while a block is locked, and Uninstall is greyed out in Settings meanwhile. Every program has Iqal's icon and a version description (iqal-winres). Next: W7b automatic builds on GitHub and SignPath | Clean install on a fresh Windows 11 VM |


---

## 5. Things that need the project owner
- **Linux Mint test machine/VM** (Mint 22.x Cinnamon). Blocking can't be tested on Windows.
- **Extension publishing:** Firefox requires every extension to be signed by Mozilla (free AMO developer account, can be "unlisted"). Chrome-family force-install works best with a Chrome Web Store listing (one-time $5 developer fee). The owner creates these accounts.
- **Upstream list licences:** each bundled list must be GPL-3.0-compatible (e.g. StevenBlack MIT, HaGeZi GPL-3.0).
