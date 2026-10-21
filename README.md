# Iqal Blocker

A native, open-source website and app blocker for Linux Mint and Windows 11.

> Status: early development. See [docs/PLAN.md](docs/PLAN.md) for the design and milestones.

## Download

- **Windows 11:** `iqal-setup-<version>.exe` from the [releases page](https://github.com/zakariaadem/Iqal/releases). Code signing for the Windows programs: free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/) (see the [code signing policy](docs/code-signing-policy.md)). Version 0.1.0 is not signed yet.
- **Linux Mint / Ubuntu:** build the package from source, as described below.

## Layout

| Path | What |
|---|---|
| `crates/iqal-core` | Platform-independent rule engine: rules, matching, schedules, locks, import/export |
| `crates/iqal-ipc` | Messages between the service and its clients |
| `crates/iqal-daemon` | The background service (runs as root under systemd) |
| `crates/iqal-cli` | Command-line tool (`iqal-cli --help`) |
| `crates/iqal-agent` | Per-session agent: desktop notifications when something is blocked |
| `crates/iqal-gtk` | The graphical app (`iqal`), GTK 4 |
| `crates/iqal-i18n` | Translations (Fluent); texts in `locales/` |
| `crates/iqal-host` | Native messaging host between the browser extension and the service |
| `crates/iqal-pack` | Dev tool: packages the extension (`.xpi`, signed `.crx`) |
| `extension` | Browser extension (Firefox and Chromium family) |
| `lists` | Curated starter lists for the pre-made categories |
| `locales` | Interface translations (11 languages) |
| `packaging/linux` | systemd unit, agent autostart entry |
| `packaging/windows` | Builds the Windows setup (`build-setup.ps1`) |
| `crates/iqal-win32` | The graphical app on Windows |
| `crates/iqal-setup` | The Windows setup and uninstaller |
| `scripts/dev-install.sh` | Install a local build as a system service |

## Building the package (Linux Mint 22 / Ubuntu 24.04)

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libshumate-dev
cargo build --release
cargo run -p iqal-pack            # browser extension
packaging/linux/build-deb.sh        # -> dist/iqal_<version>_amd64.deb
sudo apt install ./dist/iqal_*_amd64.deb
```

Removing the package is refused while any block is locked. `apt purge iqal` also deletes the settings in `/var/lib/iqal`.

Firefox only accepts a Mozilla-signed extension: put it at `dist/extension/iqal-signed.xpi` before building.

### Development install (without a package)

```bash
sudo scripts/dev-install.sh             # installs to /usr/local
sudo scripts/dev-install.sh --uninstall
```

Do not mix this with the package.

## Development without root

```bash
export IQAL_SOCKET=/tmp/iqal-dev.sock
target/release/iqal-daemon --socket $IQAL_SOCKET --state-dir /tmp/iqal-dev &
target/release/iqal-cli status
```

Tests and lookup benchmark:

```bash
cargo test
cargo run --release --example bench -p iqal-core
```

## Building the setup (Windows 11)

With Rust and Visual Studio's C++ build tools installed:

```powershell
powershell -File packaging\windows\build-setup.ps1   # -> dist\iqal-setup-<version>.exe
```

The setup installs Iqal into `Program Files\Iqal` (or updates it), and `iqal-setup.exe --uninstall` removes it again; Settings > Apps does the same. Both are refused while any block is locked. GitHub Actions builds and tests every change ([`.github/workflows/windows.yml`](.github/workflows/windows.yml)) and signs releases: see the [code signing policy](docs/code-signing-policy.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
