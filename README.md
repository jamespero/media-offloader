# Media Offloader

Copy camera cards safely. Media Offloader is a free, open-source desktop app for Windows and macOS that copies a camera card to your drives and proves every copy is good before it tells you the card is safe to reuse.

Pick the card, pick a destination and an optional backup drive, and press Start. Each file is copied, checked against a checksum read back from the disk itself (not from the OS cache), and only then marked done. When the copy verifies, the card is free; the backup carries on from the verified copy.

It is built for shooters, camera assistants and DITs who want a single screen with a status you can read from across the set: **violet** means working, **mint** means done and safe, **red** means a problem.

## Status

Early development; nothing is packaged yet.

## Layout

| Path | What |
|---|---|
| `crates/core` | `offloader-core`: the copy/verify engine. UI-agnostic; the CLI and the app both drive it. |
| `crates/cli` | `offload`: command-line front end, for testing against real cards and for scripting. |
| `app` | The desktop app: Tauri shell (`app/src-tauri`) and UI (`app/src`). |

## Build and test

Needs Rust 1.85+ (stable) and, for the app, Node 20+.

```bash
cargo test --workspace
```

## Contributing

Bug reports and pull requests are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first: it covers setup, the safety rules the code never breaks, Conventional Commits, and how releases are versioned.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
