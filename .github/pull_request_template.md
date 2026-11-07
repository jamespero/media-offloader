## What and why

<!-- What does this change, and why? Link the issue if there is one. -->

## How it was tested

<!-- Tests added or changed, and anything you checked by hand (on which OS). -->

## Checklist

- [ ] The title follows Conventional Commits (`feat(core): …`, `fix(app): …`)
- [ ] `cargo fmt`, `cargo clippy -- -D warnings` and `cargo test --workspace` pass locally
- [ ] `npm run build` passes in `app/` (if the app changed)
- [ ] No safety rule is weakened (see CONTRIBUTING.md)
- [ ] Tests use generated fake cards only; nothing ejects or dismounts a real drive
- [ ] No version numbers were changed (maintainers handle releases)
