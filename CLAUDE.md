# zed-bevy-color

## Contributor docs

`CONTRIBUTING.md` must stay accurate. Update it in the same PR whenever
any of these change:

- Crates added/removed from the workspace.
- Build commands, targets, toolchain components.
- Test or bench invocation.
- Nix flake inputs, dev-shell packages, or `rust-toolchain.toml`.
- Release automation (`release-plz.toml`, `.github/workflows/release-plz.yml`,
  `.github/workflows/release.yml`).
- Zed extension metadata (`crates/zed-extension/extension.toml`) or
  registry submission flow.
- Commit conventions or branching strategy.
- Required repo secrets or settings.

If you touch any of the above without updating `CONTRIBUTING.md`, the
change is incomplete.

## Releases

Automated via release-plz. Never hand-bump `workspace.package.version`
in `Cargo.toml` or edit `CHANGELOG.md` entries for released versions —
both are managed by release-plz from conventional commits.
