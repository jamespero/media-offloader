# Media Offloader

An open-source camera-card offload app for Windows + macOS: copy a card to a primary drive with checksum verification, then make a verified backup on a second drive. One screen, a status readable from across the set, and safety rules that are never bent.

Maintainer: James Pero. How to contribute (branches, Conventional Commits, PR checks, versioning): [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Names
- Product: **Media Offloader**. Crates: `offloader-core` (engine), `offloader-cli` (binary `offload`), `media-offloader-app` (Tauri shell). Bundle identifier `com.jamespero.mediaoffloader`.
- This is an original project. Don't reference, imitate the branding of, or describe it as a clone of any other product.

## Locked decisions
- **Stack:** Tauri 2 + Rust. Copy/hash engine in a UI-agnostic Rust crate with a CLI for testing; web UI (Svelte 5, TypeScript, plain CSS variables).
- **Platforms:** Windows + macOS. Unsigned builds for now.
- **License:** MIT OR Apache-2.0. Bundled fonts are SIL OFL.
- **Verification:** xxHash64 by default; MD5 optional (computed in the same read). Verify by re-reading the destination **with the OS cache bypassed** (`F_NOCACHE` on macOS, `FILE_FLAG_NO_BUFFERING` on Windows).
- **Copy model:** sequential by default (copy → verify → card is free → backup reads from the verified copy). Fan-out (one card read, write all destinations) is a setting.
- **Sidecars:** ASC MHL v2 manifest + optional `.md5` list next to the footage. Plain-text log per transfer always.
- **Folder sources:** behave like cards, but no auto-select and no eject.

## Safety rules (never break these)
1. Never overwrite anything. If the destination already exists, write into `copy-conflicted`, `copy-conflicted-1`, …
2. Never modify or delete anything on the source. No "format card" feature.
3. Write to `<name>.partial`, fsync, then rename. A file only counts as done after its verify pass matches.
4. Check free space on every destination before starting.
5. Preserve the card's folder structure exactly; copy every file (hidden OS junk optional, off by default).

## Design rules
Full details and tokens: `docs/design-system.md`.
- **Color means status; controls stay neutral.** Violet `#8E7CFF` = working, mint `#4BE3A3` = done & safe, red `#FF5C5C` = problem. Buttons, chips, toggles, menus are neutral grays; the one primary action (Start) is white.
- Dark only. Fonts: Outfit (headings, lowercase), Inter (UI), JetBrains Mono (paths/filenames) — bundled locally (SIL OFL), no CDN.
- All artwork (icons, app icon) is drawn for this project.

## Working here
- Every change goes through a pull request; `main` is protected. PR titles follow Conventional Commits (they become the squash commit).
- Tests run against generated fake card folders; never eject or dismount real drives in tests.
- Before pushing: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and in `app/`: `npm run build`.
