# Iqal

A native, open-source website and app blocker for Linux Mint and Windows 11.

> Status: early development. See [docs/PLAN.md](docs/PLAN.md) for the design and milestones.

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

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
