# zed-bevy-color

Rust workspace, two crates:
- `crates/bevy-color-lsp/` — `tower-lsp` server + `tree-sitter-rust`
  parser. Bin + lib. Speaks `textDocument/documentColor` and
  `textDocument/colorPresentation`.
- `crates/zed-extension/` — `cdylib` on `wasm32-wasip2`; resolves the
  LSP binary for Zed.

## Commands

```sh
cargo test --workspace                   # unit + integration tests
cargo bench -p bevy-color-lsp            # criterion benches
cargo clippy --workspace --all-targets -- -D warnings   # mirrors CI
cargo build -p zed-bevy-color-extension --release --target wasm32-wasip2
cargo install --path crates/bevy-color-lsp    # local install for editor tests
```

Nix: `direnv allow` (or `nix develop`) gives rust + `cargo-criterion`,
`cargo-watch`, `just`, `release-plz`, `git-cliff`.

## Gotchas

- **Tree-sitter queries are `const &str`; static init uses `expect()`
  by design** (`parser.rs`, `detectors/*/QUERY`). A failed `expect`
  means a build-time authoring bug. Do not rewrite these to unwrap/`?`
  — they live above `main` and cannot return `Result`.
- **LSP byte/position math uses UTF-16 offsets** per LSP spec, not
  UTF-8. See `document::byte_to_position` / `position_to_byte`.
- **`crates/zed-extension/` compiles only to `wasm32-wasip2`** — skip
  it from `cargo test`; it has no tests. CI builds it separately.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/) required —
release-plz reads them to decide version bumps. Invalid messages build
fine but are dropped from `CHANGELOG.md`.

## Specs & plans

`docs/superpowers/specs/` — approved designs.
`docs/superpowers/plans/` — implementation plans. Check these before
starting non-trivial work; there may be an active plan.

## Releases

Automated via release-plz. Never hand-bump `workspace.package.version`
in `Cargo.toml` or edit `CHANGELOG.md` entries for released versions —
both are managed by release-plz from conventional commits.
