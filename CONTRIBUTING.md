# Contributing

Thanks for hacking on `zed-bevy-color`. This doc covers the dev loop,
commit conventions, release automation, and how to publish new versions
to the Zed extensions registry.

## Repo layout

```
crates/
├── bevy-color-lsp/      # tower-lsp server, tree-sitter-rust parser
│   ├── src/             # server, detector, color math
│   ├── tests/           # integration tests over the LSP transport
│   └── benches/         # criterion benches (`cargo bench`)
└── zed-extension/       # WASM wrapper; resolves binary path for Zed
```

Workspace version lives in `Cargo.toml` (`workspace.package.version`);
both crates inherit it. Do not bump by hand — release-plz does it (see
[Releases](#releases)).

## Dev environment

### With nix + direnv (recommended)

```sh
direnv allow   # first time only; afterwards cd auto-activates the shell
```

This provides: Rust stable (`rustfmt`, `clippy`, `wasm32-wasip2`
target), `cargo-criterion`, `cargo-watch`, `just`, `release-plz`,
`git-cliff`.

Or enter the shell manually:

```sh
nix develop
```

The toolchain is pinned via `rust-toolchain.toml` and the flake reads
it — change the file once, both non-nix and nix users track it.

### Without nix

Install `rustup`, then `cd` into the repo. `rust-toolchain.toml` will
auto-install the correct channel, components, and `wasm32-wasip2`
target on first `cargo` invocation.

## Dev loop

Build the LSP server:

```sh
cargo build -p bevy-color-lsp
```

Run tests:

```sh
cargo test --workspace
```

Run benches:

```sh
cargo bench -p bevy-color-lsp
```

Build the Zed extension (WASM):

```sh
cargo build -p zed-bevy-color-extension --release --target wasm32-wasip2
```

Load the extension into Zed for local testing: `zed: install dev
extension` → select `crates/zed-extension/`. Ensure `bevy-color-lsp` is
on `$PATH` (`cargo install --path crates/bevy-color-lsp`) or the
extension will try to download a release binary.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: …` — new functionality (minor bump)
- `fix: …` — bug fix (patch bump)
- `perf: …` — perf improvement (patch bump)
- `docs: …`, `test: …`, `chore: …`, `refactor: …` — no version bump
- `feat!: …` or a `BREAKING CHANGE:` footer — major bump

release-plz reads these to decide version bumps and generate the
changelog. Non-conformant messages won't break the build but will be
dropped from the changelog.

## Releases

Fully automated via [release-plz](https://release-plz.dev):

1. Merge conventional commits into `main`.
2. The `release-plz` workflow opens (or updates) a `chore: release
   vX.Y.Z` PR bumping the workspace version and `CHANGELOG.md`.
3. Merge that PR. release-plz tags `vX.Y.Z` and creates a GitHub
   release.
4. The tag push triggers `release.yml`, which builds the
   `bevy-color-lsp` binary for all supported targets and attaches them
   to the release.
5. The Zed extension resolves to that release's binary on first use.

### Required secret

`RELEASE_PLZ_TOKEN` — a PAT with `contents: write` +
`pull-requests: write`. Required because the default `GITHUB_TOKEN`
cannot trigger `release.yml` from tags it created.

## Publishing to the Zed extensions registry

See [Zed docs: developing extensions][zed-ext-docs]. Summary:

### First-time publish

1. Finish and merge a versioned release (see above) so there's a
   tagged `vX.Y.Z` for the extension to reference.
2. Fork [`zed-industries/extensions`][zed-ext-repo].
3. From the fork root:
   ```sh
   git submodule add https://github.com/DavidDudson/zed-bevy-colors.git extensions/bevy-color
   git add extensions/bevy-color
   ```
4. Add to top-level `extensions.toml`:
   ```toml
   [bevy-color]
   submodule = "extensions/bevy-color"
   version = "X.Y.Z"
   ```
   The `version` must match `version` in `crates/zed-extension/extension.toml`
   at the pinned submodule commit.
5. Run `pnpm sort-extensions` to keep the file sorted.
6. Open a PR against `zed-industries/extensions`. A Zed maintainer
   reviews and merges.

### Updating an existing entry

From the `zed-industries/extensions` fork root:

```sh
git submodule update --remote extensions/bevy-color
```

Bump the `version` in `extensions.toml` to match the new
`extension.toml` version, run `pnpm sort-extensions`, open a PR.

### Gotchas

- The submodule URL **must** be HTTPS, not SSH.
- `extension.toml` version and `extensions.toml` version must agree.
- `crates/zed-extension/` currently lives alongside the LSP in this
  repo. When Zed registry reviewers pin a submodule commit, the entire
  repo is pinned — that is fine; Zed only consults
  `crates/zed-extension/extension.toml`.

[zed-ext-docs]: https://zed.dev/docs/extensions/developing-extensions
[zed-ext-repo]: https://github.com/zed-industries/extensions

## Ecosystem discoverability

- **[Bevy Assets][bevy-assets]** — Bevy's community asset index.
  Submit via a PR to [`bevyengine/bevy-website`][bevy-website] under
  `content/assets/` (there's a `Development Tools` section that fits).
- **[Zed extensions registry][zed-reg]** — covered above.

[bevy-assets]: https://bevy.org/assets/
[bevy-website]: https://github.com/bevyengine/bevy-website
[zed-reg]: https://zed.dev/extensions

## Keeping this file current

Whenever any of the following changes, update this file in the same
PR:

- New crate added to the workspace.
- Build command, target, or required toolchain component changes.
- Test or bench invocation changes.
- Nix flake inputs, dev-shell packages, or `rust-toolchain.toml`.
- Release automation config (`release-plz.toml`,
  `.github/workflows/release-plz.yml`, `.github/workflows/release.yml`).
- Zed extension metadata (`crates/zed-extension/extension.toml`) or the
  registry submission flow.
- Commit convention or branching strategy.
- New required secret or repo setting.

If a PR touches any of the above, update `CONTRIBUTING.md` too.
