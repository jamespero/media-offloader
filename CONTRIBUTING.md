# Contributing to Media Offloader

Thanks for helping out. Media Offloader moves people's footage, so the bar is simple: a change must never make a copy less safe. This guide covers how to set up, the rules the code follows, and how changes get in.

## Setting up

You need:

- Rust stable, 1.85 or newer ([rustup](https://rustup.rs)), with `rustfmt` and `clippy`.
- Node 20 or newer, for the desktop app's UI.
- Windows: WebView2 (built into Windows 11) and the MSVC build tools.
- macOS: the Xcode command-line tools (`xcode-select --install`).

Then:

```bash
cargo test --workspace
```

The desktop app lives in `app/`; run `npm install` there once, then `npm run tauri dev`.

## Testing

- Tests run against **generated fake cards** (temporary folders that look like camera cards). Never point a test at a real card or drive.
- **Never eject or dismount a real drive in a test**, not even to check that a refusal path works.
- New engine behavior comes with a test. Fault cases (corruption, a file changing mid-copy, read errors, unplugging, a full disk) are tested by injecting the fault, not by hoping for it.
- Changes that only matter on one platform still have to build and pass on both; CI runs Windows and macOS.

## Rules the code follows

These are not up for debate in a PR. If you think one is wrong, open an issue first.

1. Never overwrite anything. An existing destination folder means the copy goes into `copy-conflicted`, `copy-conflicted-1`, …
2. Never modify or delete anything on the source. There is no "format card" feature and there won't be.
3. Write to `<name>.partial`, fsync, then rename. A file only counts as done once its verify pass matches.
4. Check free space on every destination before starting.
5. Preserve the card's folder structure exactly and copy every file (hidden OS junk is optional and off by default).
6. Verify reads bypass the OS cache (`F_NOCACHE` on macOS, `FILE_FLAG_NO_BUFFERING` on Windows), so a match proves what is on the disk.

For the UI:

- Color means status; controls stay neutral. Violet `#8E7CFF` is working, mint `#4BE3A3` is done and safe, red `#FF5C5C` is a problem. Buttons, chips, toggles and menus are neutral grays; the one primary action (Start) is white.
- Dark only. Outfit for headings (lowercase), Inter for UI text, JetBrains Mono for paths and file names, all bundled locally. No CDNs.
- All artwork is drawn for this project. Don't add third-party logos, icons or screenshots.

## Making a change

1. Fork the repo (or branch, if you're a maintainer) and make your change on a branch.
2. Run the same checks CI runs:

   ```bash
   cargo fmt --all --check
   ```

   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

   ```bash
   cargo test --workspace
   ```

   and, if you touched the app, from `app/`:

   ```bash
   npm run build
   ```

3. Open a pull request against `main` and fill in the template.

Every pull request is checked when it's opened and again on every push:

| Check | What it does |
|---|---|
| **PR title** | The title follows Conventional Commits (below). Editing the title re-runs it. |
| **Build and test (Windows / macOS)** | Formatting, clippy with warnings as errors, the full test suite, and the app's typecheck and build. |
| **Version consistency** | Every version number in the repo is valid semver and they all match. |

All checks must pass, and the branch must be up to date with `main`, before a pull request can be merged. Pull requests are squash-merged, so the title becomes the commit on `main`.

## Conventional Commits

Pull request titles (and ideally your commits) follow [Conventional Commits](https://www.conventionalcommits.org):

```
<type>(<optional scope>): <short summary in the imperative>
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `style`, `chore`, `revert`.

Scopes: `core` (the engine), `cli` (the `offload` command), `app` (the desktop app), `ci`, `deps`. Leave the scope off when a change spans several.

Examples:

- `feat(core): resume an interrupted offload from its journal`
- `fix(app): keep the source list stable while a card mounts`
- `docs: explain the copy-conflicted folders`

A breaking change gets a `!` after the type or scope (`feat(cli)!: rename --dest to --to`) and a `BREAKING CHANGE:` paragraph in the description.

## Versioning and releases

Media Offloader uses [Semantic Versioning](https://semver.org). The version lives in the Cargo workspace (`Cargo.toml`), `app/package.json` and `app/src-tauri/tauri.conf.json`, and is repeated in `Cargo.lock` and `app/package-lock.json`; CI checks that they always match.

- **Don't bump versions in your pull request.** Maintainers decide when to release and what the version is.
- Releases are prepared automatically from the commit history: a release pull request collects the changes since the last release, bumps every version and updates `CHANGELOG.md`. Merging it tags the release, and the Release workflow then builds the Windows and macOS installers and attaches them to it. If a build fails, re-run the workflow by hand with the tag.
- While the version is below 1.0, `feat` and breaking changes bump the minor version and `fix` bumps the patch version. A maintainer can override the next version with a `Release-As: x.y.z` line in a commit message.

## License

By contributing, you agree that your contributions are dual licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at the user's option, without any additional terms or conditions.
