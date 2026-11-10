# Contributing to Media Offloader

Thanks for helping out. Media Offloader moves people's footage, so the bar is simple: a change must never make a copy less safe. This guide covers how to set up, the rules the code follows, and how changes get in.

## Setting up

You need:

- Rust stable, 1.90 or newer ([rustup](https://rustup.rs)), with `rustfmt` and `clippy`.
- Node 24 or newer, for the desktop app's UI and its tests.
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

The app's UI has its own tests, run from `app/` with `npm test` ([Vitest](https://vitest.dev) and [Testing Library](https://testing-library.com/docs/svelte-testing-library/intro), in a simulated browser):

- **Component and module tests** sit next to what they test, as `Icon.test.ts` beside `Icon.svelte` or `presets.test.ts` beside `presets.ts`.
- **Whole-app tests** in `src/test/` start the real app with the same simulated backend as the browser preview (`src/preview.ts`), click through it the way a person would, and check what reaches the backend. `startApp()` in `src/test/app.ts` gives each test a fresh app, fake timers so a simulated offload finishes instantly, and a record of every backend call.
- Find things the way a person would: by role and visible name (`getByRole("button", { name: "Start" })`), not by class names.
- A UI change comes with a test, and a bug fix with a test that fails without the fix.

## Rules the code follows

These are not up for debate in a PR. If you think one is wrong, open an issue first.

1. Never overwrite anything. An existing destination folder means the copy goes into `copy-conflicted`, `copy-conflicted-1`, …
2. Never modify or delete anything on the source. There is no "format card" feature and there won't be.
3. Write to `<name>.partial`, fsync, then rename. A file only counts as done once its verify pass matches.
4. Check free space on every destination before starting.
5. Preserve the card's folder structure exactly and copy every file (hidden OS junk is optional and off by default).
6. Verify reads bypass the OS cache (`F_NOCACHE` on macOS, `FILE_FLAG_NO_BUFFERING` on Windows), so a match proves what is on the disk.

For the UI (the [design system](docs/design-system.md) has the details, including which values to use):

- Color means status; controls stay neutral. Violet `#8E7CFF` is working, mint `#4BE3A3` is done and safe, red `#FF5C5C` is a problem. Buttons, chips, toggles and menus are neutral grays; the one primary action (Start) is white.
- Dark only. Outfit for headings (lowercase), Inter for UI text, JetBrains Mono for paths and file names, all bundled locally. No CDNs.
- All artwork is drawn for this project. Don't add third-party logos, icons or screenshots.

## UI components

The UI is built with [Svelte 5](https://svelte.dev). Each component has its own folder, with its CSS in a separate file that's scoped to it:

```text
app/src/components/Toast/
  Toast.svelte             markup and logic; ends with <style src="./Toast.css"></style>
  Toast.css                the component's styles
  toastState.svelte.ts     state shared with code outside the component, if it needs any
```

Components read the app state from `S` in `state.svelte.ts` and update themselves when it changes. Each button handles its own click with `onclick`, calling a function from the module that owns that piece of the app (`source.ts`, `destinations.ts`, `history.ts` and so on); a menu item runs its action and then closes the menu. Keyboard shortcuts are in `shortcuts.ts`. Settings in `S.settings` are saved automatically whenever they change, so code just changes them, and a toggle binds to its setting: `<Toggle bind:on={S.settings.sound} label="…" />`.

Global CSS is limited to `styles/global.css` (fonts and resets) and `styles/tokens.css` (design tokens). Everything else belongs to a component, including shared controls such as `Button`, `Icon`, `IconButton`, `MenuItem`, `Segmented`, `StatusPill` and `TextInput`. Icons are drawn with `<Icon name="check" size={13} />`; a new icon goes in `icons.ts` as the shapes it's made of, on the same 24×24 grid. Use those rather than restyling their class names from outside. To style markup a component doesn't render itself, such as a parent's content or an icon, use `:global()` under one of its own elements.

Name a state module differently from its component. `Toast.svelte` and `toast.svelte.ts` collide on the case-insensitive filesystems Windows and macOS use by default.

**Two TypeScript versions.** `svelte-check` doesn't support TypeScript 7 yet, so `app/package.json` installs both: `typescript7` checks `.ts` files, and `typescript` (version 6) is used only by `svelte-check`. Both install a `tsc` command, so the `typecheck` script calls version 7 by path. #43 removes this once `svelte-check` catches up, and until then Dependabot ignores major updates to `typescript`.

`npm run typecheck` runs both checkers and treats warnings as errors, including accessibility warnings and unused CSS selectors. `npm run build` and `tauri build` run it first.

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

   ```bash
   npm test
   ```

3. Open a pull request against `main` and fill in the template.

Every pull request is checked when it's opened and again on every push:

| Check | What it does |
|---|---|
| **PR title** | The title follows Conventional Commits (below). Editing the title re-runs it. |
| **Build and test (Windows / macOS)** | Formatting, clippy with warnings as errors, the full test suite, and the app's typecheck, build and UI tests. |
| **Version consistency** | Every version number in the repo is valid semver and they all match. |

All checks must pass, and the branch must be up to date with `main`, before a pull request can be merged. Pull requests are squash-merged, so the title becomes the commit on `main`.

Workflows pin every action to a full commit SHA, with the version in a comment (`uses: owner/action@<sha> # v1.2.3`). Pin any action you add the same way; Dependabot keeps the pins current.

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
- While the version is below 1.0, `feat` and breaking changes bump the minor version and `fix` and `build` bump the patch version. `build` commits, such as dependency updates, appear in the changelog under Dependencies, so they open a release pull request too; it only ships if a maintainer merges it. A maintainer can override the next version with a `Release-As: x.y.z` line in a commit message.

## License

By contributing, you agree that your contributions are dual licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at the user's option, without any additional terms or conditions.
