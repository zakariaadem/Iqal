# Al-Dir

A native, open-source website and app blocker for Linux Mint and Windows 11.

> Status: early development. See [docs/PLAN.md](docs/PLAN.md) for the design and milestones.

## Layout

| Path | What |
|---|---|
| `crates/al-dir-core` | Platform-independent rule engine: rules, matching, schedules, locks, import/export |
| `crates/al-dir-ipc` | Messages between the service and its clients |
| `crates/al-dir-daemon` | The background service (runs as root under systemd) |
| `crates/al-dir-cli` | Command-line tool (`al-dir-cli --help`) |
| `crates/al-dir-agent` | Per-session agent: desktop notifications when something is blocked |
| `crates/al-dir-gtk` | The graphical app (`al-dir`), GTK 4 |
| `crates/al-dir-i18n` | Translations (Fluent); texts in `locales/` |
| `crates/al-dir-host` | Native messaging host between the browser extension and the service |
| `crates/al-dir-pack` | Dev tool: packages the extension (`.xpi`, signed `.crx`) |
| `extension` | Browser extension (Firefox and Chromium family) |
| `lists` | Curated starter lists for the pre-made categories |
| `locales` | Interface translations (11 languages) |
| `packaging/linux` | systemd unit, agent autostart entry |
| `scripts/dev-install.sh` | Install a local build as a system service |

## Building the package (Linux Mint 22 / Ubuntu 24.04)

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libshumate-dev
cargo build --release
cargo run -p al-dir-pack            # browser extension
packaging/linux/build-deb.sh        # -> dist/al-dir_<version>_amd64.deb
sudo apt install ./dist/al-dir_*_amd64.deb
```

Removing the package is refused while any block is locked. `apt purge al-dir` also deletes the settings in `/var/lib/al-dir`.

Firefox only accepts a Mozilla-signed extension: put it at `dist/extension/al-dir-signed.xpi` before building.

### Development install (without a package)

```bash
sudo scripts/dev-install.sh             # installs to /usr/local
sudo scripts/dev-install.sh --uninstall
```

Do not mix this with the package.

## Development without root

```bash
export AL_DIR_SOCKET=/tmp/al-dir-dev.sock
target/release/al-dir-daemon --socket $AL_DIR_SOCKET --state-dir /tmp/al-dir-dev &
target/release/al-dir-cli status
```

Tests and lookup benchmark:

```bash
cargo test
cargo run --release --example bench -p al-dir-core
```

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
