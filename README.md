# Al-Dir

A native, open-source website and app blocker for Linux Mint and Windows 11.

> Status: early development. See [docs/PLAN.md](docs/PLAN.md) for the design and milestones.

## Layout

| Path | What |
|---|---|
| `crates/al-dir-core` | Platform-independent rule engine: rules, matching, schedules, locks, import/export |

## Building

```bash
cargo build --release
cargo test
```

Lookup benchmark:

```bash
cargo run --release --example bench -p al-dir-core
```

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
