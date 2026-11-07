# Media Offloader

Copy camera cards safely. Media Offloader is a free, open-source desktop app for Windows and macOS that copies a camera card to your drives and proves every copy is good before it tells you the card is safe to reuse.

Pick the card, pick a destination and an optional backup drive, and press Start. Each file is copied, checked against a checksum read back from the disk itself (not from the OS cache), and only then marked done. When the copy verifies, the card is free; the backup carries on from the verified copy.

It is built for shooters, camera assistants and DITs who want a single screen with a status you can read from across the set: **violet** means working, **mint** means done and safe, **red** means a problem.

## What it does

- **Checksum-verified copies.** xxHash64 by default, with optional MD5 computed in the same read. The verify pass re-reads the destination with the OS cache bypassed, so it checks the disk and not RAM.
- **Never overwrites, never touches the card.** Repeat offloads go into `copy-conflicted`, `copy-conflicted-1`, …; nothing on the source is ever modified or deleted, and there is no "format card" button.
- **Exact copies.** The card's folder structure is preserved file for file, so camera formats with deep folder trees still open.
- **Copy plus backup.** A primary copy and a second drive, run one after the other (the card is free as soon as the copy verifies) or from a single card read to every drive at once (fan-out).
- **Sidecars for later.** An [ASC MHL v2](https://github.com/ascmitc/mhl) manifest and an optional `.md5` list next to the footage, plus a plain-text log of every transfer. Re-verify any folder against its MHL or `.md5` list at any time.
- **Resume after a crash or unplug.** Every offload keeps a journal; continue where it stopped, or retry only the files that failed.
- **Folder naming.** Build the destination path from tokens such as Import #, Source Name, Date (Shot), Date (Now), Shoot Day #, Reel / Roll #, Project Name and Custom Text, and save patterns as presets.
- **History.** Recent offloads stay as tabs you can reopen, with their logs.
- **Finishing touches.** Thumbnails from the OS thumbnailers, a completion sound and notification, keep-awake during transfers, and eject when verified.

## Status

Early, and not yet packaged. The engine, CLI and app work on Windows; on macOS the tests pass and the app launches, but eject, keep-awake, notifications and QuickLook thumbnails haven't been tried with a real card yet. Still to come: a Camera Model naming token, real volume watching (sources are polled every 2 s today) and signed installers.

## Layout

| Path | What |
|---|---|
| `crates/core` | `offloader-core`: the copy/verify engine. UI-agnostic; the CLI and the app both drive it. |
| `crates/cli` | `offload`: command-line front end, for testing against real cards and for scripting. |
| `app` | The desktop app: Tauri shell (`app/src-tauri`) and UI (`app/src`). |

## Setting up a Mac

```bash
xcode-select --install
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install Node 20+ (e.g. from nodejs.org or `brew install node`) and follow the build steps below. On first run macOS asks for permission to access removable volumes and to show notifications; allow both. The macOS code paths (cache bypass with `F_NOCACHE`, `renamex_np`, `diskutil eject`, `caffeinate`, QuickLook thumbnails via `qlmanage`) build and the test suite passes on a Mac.

## Build and test

Needs Rust 1.85+ (stable).

```bash
cargo test --workspace
```

```bash
cargo build --release -p offloader-cli
```

The tests generate fake cards (DCIM trees, sizes around the 8 MiB chunk boundary, Unicode and odd names, a path over 260 characters, empty folders, OS junk) and inject faults: corruption at the destination, a file changing mid-copy, card read errors, unplugging the card, cancelling, a full disk.

## CLI

```bash
offload plan E:\ D:\Jobs\Day1 --backup F:\Jobs\Day1
```

```bash
offload copy E:\ D:\Jobs\Day1 --backup F:\Jobs\Day1 --md5
```

```bash
offload hash D:\Jobs\Day1\DCIM\100NCZ_R\DSC_0001.NEF --md5
```

- `plan` shows what would happen (resolved folders, `copy-conflicted`, free space, skipped and renamed files) without writing anything.
- `copy` copies, verifies with cache-bypassed reads, and backs up from the verified copy (`--fan-out` writes every drive from one card read instead). Ctrl-C stops cleanly: verified files stay, the file in progress is removed.
- `verify` re-checks a folder against its ASC MHL history or `.md5` list; `history` and `resume` list and continue past offloads.
- Logs go to `%LOCALAPPDATA%\Media Offloader\Logs` (Windows) or `~/Library/Logs/Media Offloader` (macOS); `--log-dir` / `--no-log` override.
- `--json` prints engine events as JSON lines (the same events the app consumes).
- Exit codes: 0 all verified, 1 some files failed, 2 couldn't start, 3 stopped (drive disconnected or full), 130 cancelled.

## App

The desktop app lives in `app/` (Tauri 2, Vite + TypeScript). Needs Node 20+ and the Rust toolchain; on Windows, WebView2 (built into Windows 11). From `app/`:

```bash
npm install
```

```bash
npm run tauri dev
```

```bash
npm run tauri build
```

`npm run dev` on its own serves the UI at http://localhost:1420 with a simulated engine, so the design can be checked in any browser (`?fail` shows the error state, `?slow` slows it down). That preview is dev-only and never part of a build.

## Contributing

Bug reports and pull requests are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first: it covers setup, the safety rules the code never breaks, Conventional Commits, and how releases are versioned.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option. Bundled fonts (Outfit, Inter, JetBrains Mono) are under the SIL Open Font License 1.1; see [`app/src/fonts/OFL.txt`](app/src/fonts/OFL.txt).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
