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
| `packaging/linux` | systemd unit, agent autostart entry |
| `scripts/dev-install.sh` | Install a local build as a system service |

## Building and installing (Linux)

```bash
cargo build --release
sudo scripts/dev-install.sh
al-dir-cli status
```

Remove again with `sudo scripts/dev-install.sh --uninstall` (settings stay in `/var/lib/al-dir`).

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
