# DevX & Maintainability Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise lint strictness, DevX, error handling, and docs across the
`zed-bevy-color` workspace without changing runtime behavior or
regressing benchmarks.

**Architecture:** Four independent, sequentially mergeable PR-sized
streams: (1) lint config, (2) DevX tooling, (3) code hardening, (4)
docs. Fast CI stays fast; strict checks run in parallel `lint.yml`.

**Tech Stack:** Rust stable (MSRV 1.87), `tower-lsp`, `tree-sitter`,
`parking_lot`, `thiserror`, `tracing`, `cargo-deny`, `lefthook`, `just`.

**Source of truth:** `docs/superpowers/specs/2026-04-18-devx-maintainability-design.md`.

---

## Repository Context (for the implementer)

You are working in a Rust Cargo workspace:

- `crates/bevy-color-lsp/` — lib + `bevy-color-lsp` binary. Tower-LSP
  server, tree-sitter-rust parser, color detectors, palette tables.
- `crates/zed-extension/` — `cdylib` compiled to `wasm32-wasip2`. Thin
  shim that locates the LSP binary for Zed.

Fast CI (`.github/workflows/ci.yml`) already runs `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace` on Linux/macOS/Windows. A separate job builds
the extension for `wasm32-wasip2`.

The shell is **Nushell** on the user's machine. Command examples here
use POSIX syntax that works in both bash and Nushell for single-line
invocations. For multi-line pipelines, test interactively.

## File Structure — What Gets Created or Modified

### New files

- `rustfmt.toml` — workspace formatting rules (some nightly-only).
- `clippy.toml` — MSRV + complexity thresholds.
- `deny.toml` — cargo-deny config.
- `Justfile` — task recipes.
- `lefthook.yml` — git hook config.
- `.github/workflows/lint.yml` — slow/strict checks parallel to `ci.yml`.
- `crates/bevy-color-lsp/src/num.rs` — auditable numeric cast helpers.
- `crates/bevy-color-lsp/src/error.rs` — `thiserror` error enum.

### Modified files

- `Cargo.toml` — workspace `[lints]` + `rust-version`.
- `crates/bevy-color-lsp/Cargo.toml` — new deps + `[lints] workspace = true`.
- `crates/zed-extension/Cargo.toml` — `[lints] workspace = true`.
- `crates/bevy-color-lsp/src/lib.rs` — register new modules, add crate docs.
- `crates/bevy-color-lsp/src/main.rs` — tracing init.
- `crates/bevy-color-lsp/src/server.rs` — `#[instrument]`, `num::*` call sites.
- `crates/bevy-color-lsp/src/document.rs` — parking_lot, `num::*`, signed arithmetic.
- `crates/bevy-color-lsp/src/color.rs` — `f32::from`, `num::*`.
- `crates/bevy-color-lsp/src/detectors/{bevy_ctor,bevy_hex,bevy_const,palette}.rs`
  — `num::u32_to_usize`, `num::f32_to_u8_clamped`, expect_used justifications.
- `crates/bevy-color-lsp/src/parser.rs` — expect_used justification.
- `crates/zed-extension/src/lib.rs` — crate docs + `#![warn(missing_docs)]`.
- `crates/bevy-color-lsp/tests/lsp_integration.rs` — top-level test lint allow.
- `crates/bevy-color-lsp/benches/pipeline.rs` — top-level bench lint allow.
- `flake.nix` — add `lefthook`, `cargo-deny`, `typos`, `taplo`, shellHook.
- `README.md` — new logging env var + `cargo doc` hint.
- `CONTRIBUTING.md` — Justfile table, lefthook step, lint strictness note, MSRV.

### Untouched

`release-plz.toml`, `.github/workflows/release.yml`,
`.github/workflows/release-plz.yml`, `.github/workflows/bench.yml` —
zero change.

---

## Stream 1 — Lint Config

**Single commit at end of stream.** Produces: warnings surface in fast
CI blocking new prod unwraps/expects/panics, opinionated pedantic/nursery
checks opt-in only.

### Task 1.1: Add workspace lint table + rust-version

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Edit `Cargo.toml`**

Replace the entire file with:

```toml
[workspace]
members = ["crates/bevy-color-lsp", "crates/zed-extension"]
default-members = ["crates/bevy-color-lsp"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.87"
license = "MIT"
repository = "https://github.com/DavidDudson/zed-bevy-colors"
authors = ["David Dudson <davidjohndudson@gmail.com>"]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
unreachable_pub = "warn"
trivial_casts = "warn"
trivial_numeric_casts = "warn"
rust_2018_idioms = { level = "warn", priority = -1 }

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
# Pedantic/nursery/cargo stay opt-in so the fast CI path (`ci.yml`,
# which runs clippy with `-D warnings`) is not broken by opinionated
# nudges. `lint.yml` opts in explicitly via `-W clippy::pedantic` etc.
pedantic = { level = "allow", priority = -1 }
nursery = { level = "allow", priority = -1 }
cargo = { level = "allow", priority = -1 }
# Warn-level in workspace + `-D warnings` in fast CI blocks new prod
# unwraps/expects/panics. Tests/benches opt out via file-level allow.
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "warn"
unimplemented = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

- [ ] **Step 2: Commit nothing yet — continue**

### Task 1.2: Propagate workspace lints into each crate

**Files:**
- Modify: `crates/bevy-color-lsp/Cargo.toml`
- Modify: `crates/zed-extension/Cargo.toml`

- [ ] **Step 1: Append `[lints]` to `crates/bevy-color-lsp/Cargo.toml`**

Add at the end of the file (after the `[[bench]]` block):

```toml
[lints]
workspace = true
```

- [ ] **Step 2: Append `[lints]` to `crates/zed-extension/Cargo.toml`**

Add at the end of the file:

```toml
[lints]
workspace = true
```

- [ ] **Step 3: Verify Cargo accepts the changes**

Run:
```sh
cargo metadata --format-version 1 --no-deps >/dev/null
```
Expected: exits 0 with no output. If it fails, re-check TOML syntax.

### Task 1.3: Create `rustfmt.toml`

**Files:**
- Create: `rustfmt.toml`

- [ ] **Step 1: Write the file**

```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
newline_style = "Unix"
```

Note: `imports_granularity` and `group_imports` are nightly-only. Stable
`rustfmt` ignores them (harmless). `lint.yml` runs them under nightly
with `continue-on-error: true`.

- [ ] **Step 2: Verify stable rustfmt does not reject the file**

Run:
```sh
cargo fmt --all -- --check
```
Expected: either passes or reports diffs on existing files (no config-
parse error). If config-parse errors appear, re-check TOML.

### Task 1.4: Create `clippy.toml`

**Files:**
- Create: `clippy.toml`

- [ ] **Step 1: Write the file**

```toml
msrv = "1.87"
cognitive-complexity-threshold = 20
too-many-lines-threshold = 120
type-complexity-threshold = 250
```

- [ ] **Step 2: Verify clippy accepts it**

Run:
```sh
cargo clippy --workspace --all-targets -- -D warnings
```
Do not proceed to Task 1.5 until the diagnostics are understood. Some
warnings (e.g., unwrap_used in DocumentStore) are expected and will be
silenced in Task 1.5 + removed in Stream 3.

### Task 1.5: Add temporary allows on prod Mutex unwraps

**Files:**
- Modify: `crates/bevy-color-lsp/src/document.rs`

These five unwraps all appear inside `impl DocumentStore`. Stream 3
replaces `std::sync::Mutex` with `parking_lot::Mutex`, removing the
allows. For now they need temporary `#[allow]` to pass fast CI.

- [ ] **Step 1: Add function-level allow to each affected method**

Modify the `impl DocumentStore` block (around lines 294–323). Change
each method signature to have an `#[allow]` directly above it:

```rust
impl DocumentStore {
    // TODO(Stream 3): remove once std::Mutex is swapped for parking_lot::Mutex.
    #[allow(clippy::unwrap_used)]
    pub fn open(&self, uri: Url, text: String) {
        self.docs.lock().unwrap().insert(uri, Document::new(text));
    }

    #[allow(clippy::unwrap_used)]
    pub fn replace(&self, uri: &Url, text: String) {
        if let Some(doc) = self.docs.lock().unwrap().get_mut(uri) {
            doc.set_text(text);
        }
    }

    #[allow(clippy::unwrap_used)]
    pub fn apply_change(&self, uri: &Url, range: Option<Range>, text: &str) {
        if let Some(doc) = self.docs.lock().unwrap().get_mut(uri) {
            doc.apply_change(range, text);
        }
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&self, uri: &Url) {
        self.docs.lock().unwrap().remove(uri);
    }

    #[allow(clippy::unwrap_used)]
    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> {
        self.docs
            .lock()
            .unwrap()
            .get_mut(uri)
            .map(|d| d.colors())
            .unwrap_or_default()
    }
}
```

### Task 1.6: Add temporary allows on remaining prod expect_used sites

**Files:**
- Modify: `crates/bevy-color-lsp/src/parser.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_hex.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_const.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/palette.rs`

All five `expect_used` sites are static initializations of tree-sitter
queries / parser language. They become permanent with rustdoc
justification in Stream 3 Task 3.13. For now wrap each with `#[allow]`
to unblock fast CI.

- [ ] **Step 1: `parser.rs` — wrap `make_parser` body**

Replace the `make_parser` function:

```rust
fn make_parser() -> Parser {
    let mut parser = Parser::new();
    #[allow(clippy::expect_used)]
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("load tree-sitter-rust grammar");
    parser
}
```

- [ ] **Step 2: For each detector static `QUERY`, add attribute**

`crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`:

```rust
#[allow(clippy::expect_used)]
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC).expect("compile bevy_ctor query")
});
```

Repeat for `bevy_hex.rs`, `bevy_const.rs`, `detectors/palette.rs` —
same pattern, different `expect` message.

### Task 1.7: Add top-level allow attributes to tests/benches

**Files:**
- Modify: `crates/bevy-color-lsp/tests/lsp_integration.rs`
- Modify: `crates/bevy-color-lsp/benches/pipeline.rs`

Unit tests inside `mod tests` are handled by `#[cfg(test)]` gating
plus Cargo's behavior — workspace `warn`-level lints in test code can
be overridden at the module boundary.

- [ ] **Step 1: Add file-top allow to `tests/lsp_integration.rs`**

Prepend (before any other content):

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]
```

- [ ] **Step 2: Add file-top allow to `benches/pipeline.rs`**

Same prepend.

- [ ] **Step 3: Add module-level allow to every `#[cfg(test)] mod tests` block**

For each of these files, find the `#[cfg(test)] mod tests {` line and
change it to:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
```

Files to touch:
- `crates/bevy-color-lsp/src/color.rs`
- `crates/bevy-color-lsp/src/palette.rs`
- `crates/bevy-color-lsp/src/document.rs`
- `crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`
- `crates/bevy-color-lsp/src/detectors/bevy_hex.rs`
- `crates/bevy-color-lsp/src/detectors/bevy_const.rs`
- `crates/bevy-color-lsp/src/detectors/palette.rs`

### Task 1.8: Verify fast-CI clippy still passes

**Files:** none changed in this task.

- [ ] **Step 1: Run clippy exactly as CI does**

```sh
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: `Finished` without errors. If warnings appear, scrutinize —
they are likely new signals worth fixing, not suppressing.

- [ ] **Step 2: Run fmt check**

```sh
cargo fmt --all -- --check
```
Expected: no diff. If diffs exist, run `cargo fmt --all` and re-check.

- [ ] **Step 3: Run tests**

```sh
cargo test --workspace
```
Expected: all tests pass.

### Task 1.9: Commit Stream 1

- [ ] **Step 1: Stage + commit**

```sh
git add Cargo.toml rustfmt.toml clippy.toml \
  crates/bevy-color-lsp/Cargo.toml \
  crates/zed-extension/Cargo.toml \
  crates/bevy-color-lsp/src/ \
  crates/bevy-color-lsp/tests/lsp_integration.rs \
  crates/bevy-color-lsp/benches/pipeline.rs
git commit -m "chore: workspace lint config + rust-version 1.87

Adds [workspace.lints] (unsafe_code=forbid, clippy::all=deny,
unwrap/expect/panic=warn; pedantic/nursery/cargo opt-in only),
rustfmt.toml, clippy.toml. Temporary allow attributes on 5 prod Mutex
unwraps and 5 static-init expects — Stream 3 removes them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Stream 2 — DevX Tooling

### Task 2.1: Create `Justfile`

**Files:**
- Create: `Justfile`

- [ ] **Step 1: Write the file**

```makefile
default: check

# Fast local check — mirrors ci.yml
check: fmt-check clippy test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets -- \
        -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo

test:
    cargo test --workspace

bench:
    cargo bench -p bevy-color-lsp

wasm:
    cargo build -p zed-bevy-color-extension --release --target wasm32-wasip2

deny:
    cargo deny check

docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Full pre-push gate — mirrors lint.yml + ci.yml
ci: fmt-check clippy test deny docs wasm

watch:
    cargo watch -x 'clippy --workspace --all-targets'
```

- [ ] **Step 2: Verify recipes parse**

```sh
just --list
```
Expected: tabular output listing every recipe above. If `just` is
missing, enter `nix develop` first.

### Task 2.2: Create `lefthook.yml`

**Files:**
- Create: `lefthook.yml`

- [ ] **Step 1: Write the file**

```yaml
pre-commit:
  parallel: true
  commands:
    fmt:
      glob: "*.rs"
      run: cargo fmt --all -- --check
    clippy:
      glob: "*.rs"
      run: cargo clippy --workspace --all-targets -- -D warnings
    typos:
      run: typos

commit-msg:
  commands:
    conventional:
      run: |
        grep -qE '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?!?: .+' {1} \
          || (echo "commit msg must follow Conventional Commits" && exit 1)
```

- [ ] **Step 2: Install the hooks locally**

```sh
lefthook install
```
Expected: `sync hooks: ✓ (pre-commit, commit-msg)`.

- [ ] **Step 3: Sanity check — force a lint error**

Temporarily introduce a trivial format drift in any `.rs` file, attempt
`git commit --allow-empty -m "test: lefthook"`, confirm hook blocks,
revert the drift.

### Task 2.3: Create `deny.toml`

**Files:**
- Create: `deny.toml`

- [ ] **Step 1: Write the file**

```toml
[advisories]
version = 2
yanked = "deny"
ignore = []

[licenses]
version = 2
allow = [
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-DFS-2016",
  "Unicode-3.0",
  "Zlib",
  "CC0-1.0",
  "MPL-2.0"
]
confidence-threshold = 0.9

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

- [ ] **Step 2: Run cargo-deny locally**

```sh
cargo deny check
```
Expected: `advisories ok`, `licenses ok`, `bans warn` for multiple
versions (acceptable), `sources ok`. Any `licenses` error requires
adding the exact SPDX id to the `allow` list and explaining in the
commit message.

### Task 2.4: Update `flake.nix`

**Files:**
- Modify: `flake.nix`

- [ ] **Step 1: Add new devshell packages and shellHook**

Replace the `devShells.default` block with:

```nix
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rust
            cargo-criterion
            cargo-watch
            cargo-deny
            just
            lefthook
            release-plz
            git-cliff
            typos
            taplo
          ];

          RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";

          shellHook = ''
            if [ -d .git ] && [ ! -f .git/hooks/pre-commit ]; then
              ${pkgs.lefthook}/bin/lefthook install >/dev/null
            fi
          '';
        };
```

- [ ] **Step 2: Re-enter devshell and verify**

```sh
nix develop --command bash -c 'which cargo-deny lefthook typos taplo just'
```
Expected: 5 absolute paths. If missing, `nix flake update` then retry.

### Task 2.5: Create `.github/workflows/lint.yml`

**Files:**
- Create: `.github/workflows/lint.yml`

- [ ] **Step 1: Write the file**

```yaml
name: lint

on:
  pull_request:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  cargo-deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2

  pedantic:
    runs-on: ubuntu-latest
    # Stream 4 flips this to `false` after pedantic-clean codebase.
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: |
          cargo clippy --workspace --all-targets -- \
            -D warnings \
            -W clippy::pedantic \
            -W clippy::nursery \
            -W clippy::cargo

  docs:
    runs-on: ubuntu-latest
    env:
      RUSTDOCFLAGS: "-D warnings"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --workspace --no-deps

  typos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: crate-ci/typos@master

  nightly-fmt:
    runs-on: ubuntu-latest
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt
      - run: cargo +nightly fmt --all -- --check
```

### Task 2.6: Update `CONTRIBUTING.md` for DevX additions

**Files:**
- Modify: `CONTRIBUTING.md`

- [ ] **Step 1: Add a new "Task runner" subsection under "Dev loop"**

Find the `## Dev loop` heading. Just after it, insert:

```markdown
### Task runner

Common commands are defined in `Justfile`:

| Recipe | Action |
|--------|--------|
| `just` / `just check` | fmt-check + clippy + test (local fast path) |
| `just fmt` | format all |
| `just clippy` | default clippy, same as CI |
| `just clippy-strict` | add pedantic + nursery + cargo groups |
| `just test` | `cargo test --workspace` |
| `just bench` | criterion benches |
| `just wasm` | build Zed extension for `wasm32-wasip2` |
| `just deny` | `cargo deny check` |
| `just docs` | build rustdoc with `-D warnings` |
| `just ci` | full gate (mirrors ci.yml + lint.yml) |
| `just watch` | rerun clippy on file change |
```

- [ ] **Step 2: Add a "Pre-commit hooks" subsection after "Task runner"**

```markdown
### Pre-commit hooks

Configured via `lefthook.yml` — runs `cargo fmt --check`,
`cargo clippy -D warnings`, and `typos` on staged Rust/text files, plus
validates Conventional Commits format on `commit-msg`.

First-time install (nix users: auto-runs from flake `shellHook`):

```sh
lefthook install
```

Skip once with `--no-verify` only for emergencies; fix the root cause
and re-commit rather than routinely bypassing.
```

- [ ] **Step 3: Add a "Lint strictness" subsection after "Pre-commit hooks"**

```markdown
### Lint strictness

`Cargo.toml` declares workspace `[lints]`:

- `clippy::all = deny` — correctness/suspicious/style lints block.
- `clippy::{pedantic,nursery,cargo} = allow` at workspace level; opted
  into by the `lint.yml` CI job only.
- `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` = warn,
  promoted to deny in fast CI via `-D warnings`.

New warnings must be fixed or justified with a narrowly scoped
`#[allow(clippy::LINT)] // reason…`. `unsafe_code = forbid` —
`unsafe` blocks are not permitted.

MSRV is **1.87** (declared in `Cargo.toml` and `clippy.toml`). Bumping
it is a breaking change for downstream packagers; discuss in a PR
first.

Supply-chain audit via `cargo deny check` (`deny.toml`) runs in
`lint.yml`.
```

- [ ] **Step 4: Update the "Keeping this file current" list**

Find the "Keeping this file current" section. Add two bullets to its
list:

```markdown
- `rustfmt.toml`, `clippy.toml`, `deny.toml`, `Justfile`,
  `lefthook.yml`, `.github/workflows/lint.yml`.
- Workspace lint config (`[workspace.lints]` in `Cargo.toml`) or MSRV.
```

### Task 2.7: Verify everything green, commit Stream 2

- [ ] **Step 1: Run the full gate**

```sh
just ci
```
Expected: all recipes pass. If the `docs` recipe fails with missing
docs warnings, that is Stream 4's job — temporarily skip with
`just fmt-check clippy test deny wasm` and proceed.

- [ ] **Step 2: Commit**

```sh
git add Justfile lefthook.yml deny.toml flake.nix \
  .github/workflows/lint.yml CONTRIBUTING.md
git commit -m "chore: add devx tooling (Justfile, lefthook, cargo-deny, lint.yml)

Justfile task runner, lefthook pre-commit hooks, cargo-deny supply-chain
audit, parallel lint.yml workflow (cargo-deny, pedantic clippy, docs,
typos, nightly-fmt). Nix devshell gains lefthook/cargo-deny/typos/taplo
+ auto-installs hooks on shell entry.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Stream 3 — Code Hardening

### Task 3.1: Add new dependencies

**Files:**
- Modify: `crates/bevy-color-lsp/Cargo.toml`

- [ ] **Step 1: Add four deps to `[dependencies]`**

Locate the `[dependencies]` block. Add:

```toml
parking_lot = "0.12"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Verify resolution**

```sh
cargo metadata --format-version 1 --no-deps >/dev/null
cargo fetch
```
Expected: both succeed.

### Task 3.2: Verify toolchain meets new MSRV

**Files:**
- Modify: `rust-toolchain.toml` (only if CI-stable is < 1.87 at merge time)

- [ ] **Step 1: Check stable version**

```sh
cargo --version
```
If the displayed toolchain is ≥ 1.87 — no action. Current stable as of
plan date (2026-04-18) is well past 1.87; no change expected. If an
older toolchain is pinned, change `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.87"
components = ["rustfmt", "clippy"]
targets = ["wasm32-wasip2"]
```

Prefer `channel = "stable"` unless there is a reason to pin.

### Task 3.3: TDD — write failing tests for `num.rs`

**Files:**
- Create: `crates/bevy-color-lsp/src/num.rs` (initially empty module stub)
- Modify: `crates/bevy-color-lsp/src/lib.rs`

- [ ] **Step 1: Create `num.rs` with only a module comment**

```rust
//! Narrow, auditable numeric conversions.
```

- [ ] **Step 2: Register module in `lib.rs`**

Add `pub mod num;` alongside the other `pub mod` lines:

```rust
pub mod color;
pub mod detectors;
pub mod document;
pub mod num;
pub mod palette;
pub mod parser;
pub mod server;
```

- [ ] **Step 3: Append failing tests to `num.rs`**

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn u32_to_usize_roundtrip() {
        assert_eq!(u32_to_usize(0), 0);
        assert_eq!(u32_to_usize(u32::MAX), u32::MAX as usize);
    }

    #[test]
    fn usize_to_u32_saturates() {
        assert_eq!(usize_to_u32_sat(0), 0);
        assert_eq!(usize_to_u32_sat(12345), 12345);
        // On 64-bit usize > u32::MAX saturates.
        assert_eq!(usize_to_u32_sat(u32::MAX as usize + 1), u32::MAX);
        assert_eq!(usize_to_u32_sat(usize::MAX), u32::MAX);
    }

    #[test]
    fn f32_to_u8_clamps_low_high() {
        assert_eq!(f32_to_u8_clamped(-1.0), 0);
        assert_eq!(f32_to_u8_clamped(0.0), 0);
        assert_eq!(f32_to_u8_clamped(127.5), 128);
        assert_eq!(f32_to_u8_clamped(255.0), 255);
        assert_eq!(f32_to_u8_clamped(1_000_000.0), 255);
    }

    #[test]
    fn f32_to_u8_handles_nan_and_inf() {
        // clamp(NaN) is implementation-defined in IEEE-754 but f32::clamp
        // picks the NaN path = passthrough. round() of NaN is NaN, and
        // NaN as u8 is 0 — we rely on this being saturating-to-0.
        // Goal: never panic.
        let _ = f32_to_u8_clamped(f32::NAN);
        assert_eq!(f32_to_u8_clamped(f32::INFINITY), 255);
        assert_eq!(f32_to_u8_clamped(f32::NEG_INFINITY), 0);
    }

    #[test]
    fn f32_to_u32_floor_clamped_bounds() {
        assert_eq!(f32_to_u32_floor_clamped(-1.0, 6), 0);
        assert_eq!(f32_to_u32_floor_clamped(0.0, 6), 0);
        assert_eq!(f32_to_u32_floor_clamped(5.9, 6), 5);
        assert_eq!(f32_to_u32_floor_clamped(6.0, 6), 6);
        assert_eq!(f32_to_u32_floor_clamped(100.0, 6), 6);
    }
}
```

- [ ] **Step 4: Run tests to confirm failure**

```sh
cargo test -p bevy-color-lsp num::
```
Expected: compile error — `u32_to_usize` / `usize_to_u32_sat` /
`f32_to_u8_clamped` / `f32_to_u32_floor_clamped` not found.

### Task 3.4: Implement `num.rs`

**Files:**
- Modify: `crates/bevy-color-lsp/src/num.rs`

- [ ] **Step 1: Replace file content**

```rust
//! Narrow, auditable numeric conversions.
//!
//! Cast-related clippy lints are allowed only within this module, with
//! localized justifications. Call sites use the named helpers.
//!
//! Target assumption: `usize::BITS >= 32`. The LSP binary ships for
//! 64-bit targets only; asserted at compile time below.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

const _: () = assert!(usize::BITS >= 32, "32-bit targets not supported");

/// Widen a `u32` to `usize`. Lossless on all supported targets.
#[inline]
#[must_use]
pub fn u32_to_usize(x: u32) -> usize {
    x as usize
}

/// Narrow a `usize` to `u32`, saturating at `u32::MAX`.
#[inline]
#[must_use]
pub fn usize_to_u32_sat(x: usize) -> u32 {
    if x > u32::MAX as usize {
        u32::MAX
    } else {
        x as u32
    }
}

/// Convert a normalized `f32` channel value to a `u8`, rounding to
/// nearest and clamping to `[0, 255]`. Does not panic.
///
/// `NaN` inputs yield `0` (Rust's `f32::clamp` is NaN-passthrough; the
/// subsequent `as u8` cast saturates NaN to 0 on all supported
/// platforms).
#[inline]
#[must_use]
pub fn f32_to_u8_clamped(x: f32) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

/// Convert a non-negative `f32` to `u32`, flooring and clamping to
/// `max`.
#[inline]
#[must_use]
pub fn f32_to_u32_floor_clamped(x: f32, max: u32) -> u32 {
    x.floor().clamp(0.0, max as f32) as u32
}

// …tests block stays from Task 3.3 …
```

(Keep the `#[cfg(test)] mod tests` block appended in Task 3.3.)

- [ ] **Step 2: Run tests again**

```sh
cargo test -p bevy-color-lsp num::
```
Expected: all 5 tests pass.

### Task 3.5: Swap std::Mutex for parking_lot::Mutex

**Files:**
- Modify: `crates/bevy-color-lsp/src/document.rs`

- [ ] **Step 1: Replace the `use std::sync::Mutex;` line**

Find line 4:

```rust
use std::sync::Mutex;
```

Replace with:

```rust
use parking_lot::Mutex;
```

- [ ] **Step 2: Drop `.unwrap()` from all `.lock()` calls**

`parking_lot::Mutex::lock()` returns the guard directly (no `Result`).
Edit each method and remove the `.unwrap()`. Also remove the
`#[allow(clippy::unwrap_used)]` + TODO comments added in Stream 1
Task 1.5. Final `impl DocumentStore`:

```rust
impl DocumentStore {
    pub fn open(&self, uri: Url, text: String) {
        self.docs.lock().insert(uri, Document::new(text));
    }

    pub fn replace(&self, uri: &Url, text: String) {
        if let Some(doc) = self.docs.lock().get_mut(uri) {
            doc.set_text(text);
        }
    }

    pub fn apply_change(&self, uri: &Url, range: Option<Range>, text: &str) {
        if let Some(doc) = self.docs.lock().get_mut(uri) {
            doc.apply_change(range, text);
        }
    }

    pub fn close(&self, uri: &Url) {
        self.docs.lock().remove(uri);
    }

    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> {
        self.docs
            .lock()
            .get_mut(uri)
            .map(|d| d.colors())
            .unwrap_or_default()
    }
}
```

- [ ] **Step 3: Verify build + tests**

```sh
cargo test --workspace
```
Expected: all tests still pass. `cargo clippy -- -D warnings` must
also still pass (the TODO allow attributes were removed).

### Task 3.6: TDD — add `error.rs` with failing Display test

**Files:**
- Create: `crates/bevy-color-lsp/src/error.rs`
- Modify: `crates/bevy-color-lsp/src/lib.rs`

- [ ] **Step 1: Write initial `error.rs` with only the Display test**

```rust
//! Crate error type.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn display_each_variant() {
        assert!(Error::InvalidHex("bad".into()).to_string().contains("invalid hex"));
        assert!(
            Error::PositionOutOfBounds { line: 5, character: 3 }
                .to_string()
                .contains("out of document bounds")
        );
        assert!(Error::OffsetOverflow.to_string().contains("overflow"));
        assert!(Error::GrammarLoad.to_string().contains("grammar"));
    }
}
```

- [ ] **Step 2: Register module in `lib.rs`**

Add `pub mod error;` to the pub-mod list:

```rust
pub mod color;
pub mod detectors;
pub mod document;
pub mod error;
pub mod num;
pub mod palette;
pub mod parser;
pub mod server;
```

- [ ] **Step 3: Run and confirm failure**

```sh
cargo test -p bevy-color-lsp error::
```
Expected: compile error — `Error` type not defined.

### Task 3.7: Implement `Error` enum

**Files:**
- Modify: `crates/bevy-color-lsp/src/error.rs`

- [ ] **Step 1: Prepend type definition**

Replace file content with:

```rust
//! Crate error type.

use thiserror::Error;

/// Errors surfaced by `bevy-color-lsp` internals.
#[derive(Debug, Error)]
pub enum Error {
    /// Input was not a valid hex color literal.
    #[error("invalid hex color literal: {0:?}")]
    InvalidHex(String),
    /// LSP position references a location outside the current document.
    #[error("position out of document bounds: line={line} character={character}")]
    PositionOutOfBounds {
        /// Line index (0-based, as in LSP).
        line: u32,
        /// Character index in UTF-16 code units.
        character: u32,
    },
    /// Byte-offset arithmetic overflowed while applying an incremental
    /// edit.
    #[error("byte offset arithmetic overflow")]
    OffsetOverflow,
    /// Tree-sitter failed to load its grammar.
    #[error("tree-sitter failed to load grammar")]
    GrammarLoad,
    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Crate-local `Result` alias.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn display_each_variant() {
        assert!(Error::InvalidHex("bad".into()).to_string().contains("invalid hex"));
        assert!(
            Error::PositionOutOfBounds { line: 5, character: 3 }
                .to_string()
                .contains("out of document bounds")
        );
        assert!(Error::OffsetOverflow.to_string().contains("overflow"));
        assert!(Error::GrammarLoad.to_string().contains("grammar"));
    }
}
```

- [ ] **Step 2: Verify test passes**

```sh
cargo test -p bevy-color-lsp error::
```
Expected: `display_each_variant ... ok`.

### Task 3.8: Rewrite lossless `u8 as f32` → `f32::from`

**Files:**
- Modify: `crates/bevy-color-lsp/src/color.rs`

- [ ] **Step 1: Edit `Rgba::from_u8`**

Lines 18–25. Replace with:

```rust
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        )
    }
```

- [ ] **Step 2: Tests still green**

```sh
cargo test -p bevy-color-lsp color::
```

### Task 3.9: Rewrite `color.rs` hp-branch cast

**Files:**
- Modify: `crates/bevy-color-lsp/src/color.rs`

- [ ] **Step 1: Import `num::f32_to_u32_floor_clamped`**

At top of file, add:

```rust
use crate::num::f32_to_u32_floor_clamped;
```

- [ ] **Step 2: Replace `match hp as u32 {` (line 124)**

Change:

```rust
    let (r1, g1, b1) = match hp as u32 {
```

To:

```rust
    let (r1, g1, b1) = match f32_to_u32_floor_clamped(hp, 6) {
```

- [ ] **Step 3: Run color tests**

```sh
cargo test -p bevy-color-lsp color::
```
Expected: `hsv_green` still passes (hp for h=120 s=v=1 sits at 2, which
still floors to 2 under the helper).

### Task 3.10: Rewrite `server.rs` float→u8 casts

**Files:**
- Modify: `crates/bevy-color-lsp/src/server.rs`

- [ ] **Step 1: Import helper**

Add to existing `use` block at top:

```rust
use crate::num::f32_to_u8_clamped;
```

(Adjust alongside the existing `use crate::document::DocumentStore;`.)

- [ ] **Step 2: Replace three cast lines inside `color_presentation`**

Lines 90–92. Change:

```rust
        let r = (c.red * 255.0).round() as u8;
        let g = (c.green * 255.0).round() as u8;
        let b = (c.blue * 255.0).round() as u8;
```

To:

```rust
        let r = f32_to_u8_clamped(c.red * 255.0);
        let g = f32_to_u8_clamped(c.green * 255.0);
        let b = f32_to_u8_clamped(c.blue * 255.0);
```

- [ ] **Step 3: Tests green**

```sh
cargo test -p bevy-color-lsp
```

### Task 3.11: Rewrite `document.rs` numeric casts + signed arithmetic

**Files:**
- Modify: `crates/bevy-color-lsp/src/document.rs`

- [ ] **Step 1: Import helpers + error type**

Add near the top of the file (after existing `use` lines):

```rust
use crate::error::{Error, Result};
use crate::num::{u32_to_usize, usize_to_u32_sat};
```

- [ ] **Step 2: Rewrite signed delta + incremental update to bubble errors**

The existing `apply_change` computes `delta` then optimistically applies
it. Rewrite so overflow returns via `Error::OffsetOverflow`. Replace the
`apply_change` method (lines 37–78) with:

```rust
    pub fn apply_change(&mut self, range: Option<Range>, new_text: &str) {
        let Some(range) = range else {
            self.set_text(new_text.to_string());
            return;
        };
        let start_byte = position_to_byte(&self.text, &self.line_starts, range.start);
        let old_end_byte = position_to_byte(&self.text, &self.line_starts, range.end);
        let new_end_byte = start_byte + new_text.len();
        let delta: isize = new_end_byte.cast_signed() - old_end_byte.cast_signed();

        let start_position = lsp_to_point(range.start);
        let old_end_position = lsp_to_point(range.end);

        self.text.replace_range(start_byte..old_end_byte, new_text);
        self.line_starts = compute_line_starts(&self.text);
        let new_end_position = byte_to_point(&self.text, new_end_byte);

        if let Some(tree) = self.tree.as_mut() {
            tree.edit(&InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            });
        }

        let old_cache = self.cache.take();
        self.tree = parse_incremental(&self.text, self.tree.as_ref());
        if let (Some(tree), Some(old)) = (self.tree.as_ref(), old_cache) {
            match incremental_color_update(
                &self.text,
                tree,
                old,
                start_byte,
                old_end_byte,
                new_end_byte,
                delta,
            ) {
                Ok(updated) => self.cache = Some(updated),
                Err(err) => {
                    tracing::warn!(error = %err, "incremental update failed; cache invalidated");
                    self.cache = None;
                }
            }
        }
    }
```

- [ ] **Step 3: Make `incremental_color_update` fallible**

Replace the function (lines 107–142) with:

```rust
fn incremental_color_update(
    text: &str,
    tree: &Tree,
    old: Vec<(Range, ColorMatch)>,
    edit_start: usize,
    edit_old_end: usize,
    edit_new_end: usize,
    delta: isize,
) -> Result<Vec<(Range, ColorMatch)>> {
    let rescan_start = edit_start.saturating_sub(RESCAN_CONTEXT);
    let rescan_end = (edit_new_end + RESCAN_CONTEXT).min(text.len());

    let mut kept: Vec<ColorMatch> = Vec::with_capacity(old.len());
    for (_, m) in old {
        if m.end_byte <= edit_start {
            kept.push(m);
        } else if m.start_byte >= edit_old_end {
            let new_start = m
                .start_byte
                .checked_add_signed(delta)
                .ok_or(Error::OffsetOverflow)?;
            let new_end = m
                .end_byte
                .checked_add_signed(delta)
                .ok_or(Error::OffsetOverflow)?;
            kept.push(ColorMatch {
                start_byte: new_start,
                end_byte: new_end,
                color: m.color,
            });
        }
    }
    kept.retain(|m| m.end_byte <= rescan_start || m.start_byte >= rescan_end);

    let mut new_matches = detect_in_range(tree, text, Some(rescan_start..rescan_end));
    kept.append(&mut new_matches);
    kept.sort_by_key(|m| (m.start_byte, m.end_byte));
    kept.dedup_by_key(|m| (m.start_byte, m.end_byte));

    let ranges = byte_ranges_to_lsp(text, &kept);
    Ok(kept.into_iter().zip(ranges).map(|(m, r)| (r, m)).collect())
}
```

- [ ] **Step 4: Replace `p.line as usize` / `p.character as usize`**

`lsp_to_point` (lines 154–159):

```rust
fn lsp_to_point(p: Position) -> Point {
    Point {
        row: u32_to_usize(p.line),
        column: u32_to_usize(p.character),
    }
}
```

`position_to_byte` line 181:

```rust
    let line = u32_to_usize(pos.line);
```

- [ ] **Step 5: Replace `ch.len_utf16() as u32` sites**

Three locations: line 199 (in `position_to_byte`), line 242 (in
`byte_ranges_to_lsp`), line 279 (in `byte_to_position`).

Each becomes:

```rust
        col_utf16 += usize_to_u32_sat(ch.len_utf16());
```

- [ ] **Step 6: Verify**

```sh
cargo test --workspace
```
Expected: all existing tests still pass, including `position_*`,
`incremental_*`, `store_*`.

### Task 3.12: Rewrite detector cast sites

**Files:**
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_hex.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_const.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/palette.rs`

- [ ] **Step 1: Replace `cap.index as usize` in all four files**

Find each line that reads:

```rust
            let cap_name = &QUERY.capture_names()[cap.index as usize];
```

Add `use crate::num::u32_to_usize;` near the top of each file and
change to:

```rust
            let cap_name = &QUERY.capture_names()[u32_to_usize(cap.index)];
```

Note `bevy_ctor.rs` uses `name` instead of `cap_name` — keep the
existing local variable name; only replace the cast.

- [ ] **Step 2: Rewrite u8 casts in `bevy_ctor.rs` `build_color`**

Add at top of `bevy_ctor.rs`:

```rust
use crate::num::{f32_to_u8_clamped, u32_to_usize};
```

Find the two `Rgba::from_u8(...)` blocks (lines 155–166). Replace:

```rust
        ("Color", "srgb_u8") | ("Srgba", "rgb_u8") => Some(Rgba::from_u8(
            f32_to_u8_clamped(get(0)?),
            f32_to_u8_clamped(get(1)?),
            f32_to_u8_clamped(get(2)?),
            255,
        )),
        ("Color", "srgba_u8") | ("Srgba", "rgba_u8") => Some(Rgba::from_u8(
            f32_to_u8_clamped(get(0)?),
            f32_to_u8_clamped(get(1)?),
            f32_to_u8_clamped(get(2)?),
            f32_to_u8_clamped(get(3).unwrap_or(255.0)),
        )),
```

- [ ] **Step 3: Confirm tests pass**

```sh
cargo test -p bevy-color-lsp detectors::
```

### Task 3.13: Document static-init expects permanently

**Files:**
- Modify: `crates/bevy-color-lsp/src/parser.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_hex.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_const.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/palette.rs`

The `#[allow(clippy::expect_used)]` added in Stream 1 Task 1.6 stays;
this task adds a rustdoc-style justification comment above each.

- [ ] **Step 1: parser.rs — annotate**

```rust
fn make_parser() -> Parser {
    let mut parser = Parser::new();
    // `set_language` returns Err only on an ABI mismatch between the
    // compiled tree-sitter-rust crate and the tree-sitter runtime.
    // Both are pinned in this workspace's `Cargo.lock`, so a failure
    // here is a build-time configuration bug.
    #[allow(clippy::expect_used)]
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("load tree-sitter-rust grammar");
    parser
}
```

- [ ] **Step 2: Each detector — annotate**

Identical pattern. Example `bevy_ctor.rs`:

```rust
// `QUERY_SRC` is a `const &str`; `Query::new` only errors on a syntax
// bug in the source, which the unit tests would catch immediately. A
// failure here is a build-time authoring bug.
#[allow(clippy::expect_used)]
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC).expect("compile bevy_ctor query")
});
```

Repeat across `bevy_hex.rs`, `bevy_const.rs`, `detectors/palette.rs`
with their respective `expect` messages.

### Task 3.14: Add `tracing` init to `main.rs`

**Files:**
- Modify: `crates/bevy-color-lsp/src/main.rs`

- [ ] **Step 1: Replace file content**

```rust
use bevy_color_lsp::server::run;
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("BEVY_COLOR_LSP_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    // Current-thread runtime keeps LSP stdio single-threaded; matches
    // the previous `#[tokio::main]` behavior.
    #[allow(clippy::expect_used)]
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime must build at startup");
    rt.block_on(run());
}
```

Note: the existing `main.rs` uses `#[tokio::main]` which expands to
`new_multi_thread()`. Switching to `new_current_thread()` is a
deliberate change — tower-lsp's stdin/stdout pair only needs a single
task and avoids thread contention. If the benchmark Task 3.16 regresses,
revert to `new_multi_thread()`.

- [ ] **Step 2: Build**

```sh
cargo build -p bevy-color-lsp
```

### Task 3.15: Instrument server handlers

**Files:**
- Modify: `crates/bevy-color-lsp/src/server.rs`

- [ ] **Step 1: Add instrument attribute on handlers**

Add `use tracing::instrument;` near the other `use` lines at the top
of the file.

Add `#[instrument(skip(self), err)]` (or `#[instrument(skip(self))]`
for non-`Result`-returning methods) to every LSP handler method:

```rust
    #[instrument(skip(self), err)]
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> { … }

    #[instrument(skip(self))]
    async fn initialized(&self, _: InitializedParams) { … }

    #[instrument(skip(self), err)]
    async fn shutdown(&self) -> Result<()> { … }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) { … }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) { … }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) { … }

    #[instrument(skip_all, fields(uri = %params.text_document.uri), err)]
    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> { … }

    #[instrument(skip_all, err)]
    async fn color_presentation(&self, params: ColorPresentationParams) -> Result<Vec<ColorPresentation>> { … }
```

- [ ] **Step 2: Verify build + tests**

```sh
cargo test --workspace
```

### Task 3.16: Bench diff against baseline

**Files:** none changed — verification only.

- [ ] **Step 1: Run benches on the pre-change commit baseline**

```sh
git stash  # keep Stream 3 changes
cargo bench -p bevy-color-lsp -- --save-baseline pre-stream3
git stash pop
```

- [ ] **Step 2: Run with changes, compare**

```sh
cargo bench -p bevy-color-lsp -- --baseline pre-stream3
```
Expected: no benchmark regresses more than 5%. `parking_lot` should be
same or faster; `num::*` helpers are `#[inline]`, zero-cost.

If a regression exceeds 5%, prime suspects:
1. `new_current_thread()` vs `new_multi_thread()` in `main.rs`.
2. `#[instrument]` overhead on hot handlers — drop `instrument` on
   `did_change` if it dominates.

### Task 3.17: Final lint + test pass before commit

- [ ] **Step 1: Full local CI**

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

- [ ] **Step 2: Commit Stream 3**

```sh
git add Cargo.lock \
  crates/bevy-color-lsp/Cargo.toml \
  crates/bevy-color-lsp/src/
git commit -m "refactor: harden prod code (parking_lot, num.rs, thiserror, tracing)

- std::Mutex -> parking_lot::Mutex: drops 5 prod unwraps.
- New num.rs module: audited helpers for u32<->usize widening,
  usize->u32 saturating, f32->u8 clamped, f32->u32 floor+clamp. All
  prod 'as' casts route through these.
- New error.rs with thiserror Error enum; incremental update returns
  Result so offset overflow is surfaced rather than wrapping.
- Signed byte-delta via usize::cast_signed + checked_add_signed (1.87+).
- tracing + tracing-subscriber: stderr logs, BEVY_COLOR_LSP_LOG filter,
  #[instrument] on every LSP handler.
- Static-init expects retained with narrow #[allow] + justification.

No behavior change to LSP protocol. Benches ± noise vs baseline.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Stream 4 — Docs

### Task 4.1: Crate-level docs for `bevy-color-lsp`

**Files:**
- Modify: `crates/bevy-color-lsp/src/lib.rs`

- [ ] **Step 1: Replace file content**

```rust
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::missing_crate_level_docs)]

//! # bevy-color-lsp
//!
//! LSP server providing color decoration and editing for
//! `bevy::color` values in Rust source. Implements
//! `textDocument/documentColor` and
//! `textDocument/colorPresentation`.
//!
//! ## Modules
//! - [`color`] — hex/RGB/HSL color types + parsing.
//! - [`palette`] — CSS named + Tailwind palette lookup.
//! - [`document`] — incremental tree-sitter document store.
//! - [`detectors`] — per-pattern color extractors.
//! - [`server`] — `tower-lsp` request handlers.
//! - [`error`] — crate error type.
//! - [`num`] — auditable numeric conversions.
//!
//! ## Logging
//! Set `BEVY_COLOR_LSP_LOG=debug` to see trace output on stderr.

pub mod color;
pub mod detectors;
pub mod document;
pub mod error;
pub mod num;
pub mod palette;
pub mod parser;
pub mod server;
```

- [ ] **Step 2: Build rustdoc**

```sh
RUSTDOCFLAGS="-D warnings" cargo doc -p bevy-color-lsp --no-deps
```
Expected: fails with `missing_docs` errors on pub items across all
modules. Those are fixed in the next tasks.

### Task 4.2: Document `color.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/color.rs`

- [ ] **Step 1: Add doc comments to each `pub` item**

```rust
/// 0–1 normalized, straight-alpha sRGB color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red channel, 0–1.
    pub r: f32,
    /// Green channel, 0–1.
    pub g: f32,
    /// Blue channel, 0–1.
    pub b: f32,
    /// Alpha (opacity), 0–1.
    pub a: f32,
}

impl Rgba {
    /// Construct from 0–1 normalized channel values.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self { … }

    /// Fully opaque white.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Fully opaque black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Fully transparent.
    pub const NONE: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Construct from `u8` channels (0–255), scaling to 0–1.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self { … }

    /// Construct from linear-RGB values, converting to sRGB (straight alpha).
    pub fn from_linear(r: f32, g: f32, b: f32, a: f32) -> Self { … }

    /// Clamp every channel to 0–1.
    pub fn clamped(self) -> Self { … }
}

/// Parse a hex color literal in `#RGB`, `#RRGGBB`, or `#RRGGBBAA`
/// form (leading `#` optional). Returns `None` if malformed.
pub fn parse_hex(s: &str) -> Option<Rgba> { … }

/// Convert HSL (degrees / 0–1 / 0–1) to [`Rgba`].
pub fn hsl_to_rgb(h: f32, s: f32, l: f32, a: f32) -> Rgba { … }

/// Convert HSV (degrees / 0–1 / 0–1) to [`Rgba`].
pub fn hsv_to_rgb(h: f32, s: f32, v: f32, a: f32) -> Rgba { … }

/// Convert HWB (degrees / 0–1 / 0–1) to [`Rgba`].
pub fn hwb_to_rgb(h: f32, w: f32, b: f32, a: f32) -> Rgba { … }

/// Convert OKLab (0–1 / ±0.4 / ±0.4) to [`Rgba`].
pub fn oklab_to_rgb(l: f32, a_chan: f32, b_chan: f32, alpha: f32) -> Rgba { … }

/// Convert OKLCh (0–1 / 0–0.4 / degrees) to [`Rgba`].
pub fn oklch_to_rgb(l: f32, c: f32, h: f32, a: f32) -> Rgba { … }
```

Only add the doc comments; do not touch bodies. Keep `…` examples as
placeholders here — in the actual file, keep bodies unchanged.

- [ ] **Step 2: Rustdoc check**

```sh
RUSTDOCFLAGS="-D warnings" cargo doc -p bevy-color-lsp --no-deps
```
Expected: fewer missing-doc errors (remaining modules still fail).

### Task 4.3: Document `palette.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/palette.rs`

- [ ] **Step 1: Add doc comments**

```rust
/// Look up a named color across CSS / basic / Tailwind palettes
/// (searched in that order). Case-insensitive.
pub fn lookup_named(name: &str) -> Option<Rgba> { … }

/// Look up a palette color by module + name. Module must be one of
/// `"css"`, `"basic"`, `"tailwind"`; name is case-insensitive.
pub fn lookup_palette(module: &str, name: &str) -> Option<Rgba> { … }
```

(The `css_named` / `basic_named` / `tailwind_named` / `tailwind_hex`
functions are already `fn` / not `pub` — they need no docs.)

### Task 4.4: Document `document.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/document.rs`

- [ ] **Step 1: Add doc comments**

```rust
/// A single text document's state: text, line-start index,
/// tree-sitter tree, and cached color matches.
pub struct Document {
    /// Raw document text.
    pub text: String,
    // private fields stay undocumented
    line_starts: Vec<usize>,
    tree: Option<Tree>,
    cache: Option<Vec<(Range, ColorMatch)>>,
}

impl Document {
    /// Create a new document from its initial text.
    #[must_use]
    pub fn new(text: String) -> Self { … }

    /// Replace the full document text and invalidate caches.
    pub fn set_text(&mut self, text: String) { … }

    /// Apply an LSP incremental change. `None` range means full
    /// replace; otherwise the range is edited in place and the
    /// syntax tree is incrementally re-parsed.
    pub fn apply_change(&mut self, range: Option<Range>, new_text: &str) { … }

    /// Return the current cached list of color matches, recomputing
    /// if the cache was invalidated.
    pub fn colors(&mut self) -> Vec<(Range, ColorMatch)> { … }
}

/// Convert an LSP [`Position`] (UTF-16) into a byte offset inside
/// `text`. Returns `text.len()` for positions past EOF.
#[must_use]
pub fn position_to_byte(text: &str, line_starts: &[usize], pos: Position) -> usize { … }

/// Convert a slice of [`ColorMatch`] byte ranges into LSP
/// [`Range`] UTF-16 ranges by a single linear scan of `text`.
#[must_use]
pub fn byte_ranges_to_lsp(text: &str, matches: &[ColorMatch]) -> Vec<Range> { … }

/// Convert a byte offset into an LSP [`Position`] (UTF-16).
#[must_use]
pub fn byte_to_position(text: &str, byte: usize) -> Position { … }

/// Concurrent store of open documents keyed by URI.
#[derive(Default)]
pub struct DocumentStore {
    docs: parking_lot::Mutex<HashMap<Url, Document>>,
}

impl DocumentStore {
    /// Insert a fresh document at `uri`, replacing any existing entry.
    pub fn open(&self, uri: Url, text: String) { … }

    /// Replace the text of the document at `uri` if present.
    pub fn replace(&self, uri: &Url, text: String) { … }

    /// Apply an LSP change to the document at `uri` if present.
    pub fn apply_change(&self, uri: &Url, range: Option<Range>, text: &str) { … }

    /// Drop the document at `uri`.
    pub fn close(&self, uri: &Url) { … }

    /// Get the colors for `uri`, or an empty `Vec` if the URI is
    /// unknown.
    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> { … }
}
```

### Task 4.5: Document `detectors/mod.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/detectors/mod.rs`

- [ ] **Step 1: Add docs**

```rust
//! Color-literal detectors — each submodule handles one pattern.

/// A single detected color with the source byte range it occupies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatch {
    /// Inclusive start byte in the source.
    pub start_byte: usize,
    /// Exclusive end byte in the source.
    pub end_byte: usize,
    /// Decoded color.
    pub color: Rgba,
}

/// Run every detector over the whole source tree.
#[must_use]
pub fn detect_all(tree: &Tree, source: &str) -> Vec<ColorMatch> { … }

/// Run every detector, restricted to `byte_range` if present.
#[must_use]
pub fn detect_in_range(
    tree: &Tree,
    source: &str,
    byte_range: Option<Range<usize>>,
) -> Vec<ColorMatch> { … }
```

### Task 4.6: Document each detector submodule

**Files:**
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_ctor.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_hex.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/bevy_const.rs`
- Modify: `crates/bevy-color-lsp/src/detectors/palette.rs`

- [ ] **Step 1: Crate-level `//!` at the top of each**

```rust
//! Detect Bevy `Color::<ctor>(...)` call expressions.
```

(one line suitable to each detector — e.g., bevy_hex: "Detect
`Color::hex(...)` / `Srgba::hex(...)` literal calls"; bevy_const:
"Detect `Color::<NAME>` / `Srgba::<NAME>` constants"; palette: "Detect
`palettes::<module>::<NAME>` references.")

- [ ] **Step 2: Doc the `pub fn detect`**

```rust
/// Run this detector against `tree`, writing matches to `out`.
/// Restricts the scan to `byte_range` if provided.
pub fn detect(
    tree: &Tree,
    source: &str,
    byte_range: Option<Range<usize>>,
    out: &mut Vec<ColorMatch>,
) { … }
```

### Task 4.7: Document `parser.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/parser.rs`

- [ ] **Step 1: Add docs**

```rust
//! Thread-local tree-sitter-rust parser.

/// Parse `source` from scratch; returns `None` on parser error.
///
/// # Panics
/// Panics at first call in a thread if tree-sitter-rust cannot be
/// loaded — this is a build-time configuration bug (ABI mismatch
/// between `tree-sitter` and `tree-sitter-rust`).
pub fn parse(source: &str) -> Option<Tree> { … }

/// Incrementally parse `source` given an optional previous tree.
/// See [`parse`] for panic semantics.
pub fn parse_incremental(source: &str, old: Option<&Tree>) -> Option<Tree> { … }
```

### Task 4.8: Document `server.rs` pub items

**Files:**
- Modify: `crates/bevy-color-lsp/src/server.rs`

- [ ] **Step 1: Add docs**

```rust
//! Tower-LSP server implementation.

/// LSP backend holding the document store and a handle to the client.
pub struct Backend { … }

/// Run the LSP server over stdio. Consumes the current task.
pub async fn run() { … }
```

### Task 4.9: Document `error.rs` pub items

Already done in Task 3.7. Verify rustdoc build clean for this module.

### Task 4.10: Document `num.rs` pub items

Already done in Task 3.4 (`///` on every helper; module `//!` set).
Verify rustdoc build clean for this module.

### Task 4.11: Add missing-panics / missing-errors docs

**Files:**
- Modify: handlers in `crates/bevy-color-lsp/src/server.rs` that return `Result`.

- [ ] **Step 1: For each `fn …() -> Result<_>`, add `# Errors`**

Because all server handlers return `tower_lsp::jsonrpc::Result` and the
doc lint expects an `# Errors` section on any pub Result-returning
function:

```rust
    /// Compute document colors for the requested document.
    ///
    /// # Errors
    /// Currently never errors — the underlying store always returns a
    /// (possibly empty) list. Kept as `Result` for protocol conformance.
    #[instrument(skip_all, fields(uri = %params.text_document.uri), err)]
    async fn document_color(&self, …) -> Result<Vec<ColorInformation>> { … }
```

Apply the same treatment to `initialize`, `shutdown`,
`color_presentation`.

### Task 4.12: Crate-level docs for zed-extension

**Files:**
- Modify: `crates/zed-extension/src/lib.rs`

- [ ] **Step 1: Prepend**

```rust
#![warn(missing_docs)]

//! # zed-bevy-color extension
//!
//! Zed extension shim that locates the `bevy-color-lsp` binary and
//! starts it for Rust files. Compiled to `wasm32-wasip2`.
```

- [ ] **Step 2: Add `///` to every `pub` item**

The only `pub` item here is `BevyColorExtension` (private `struct` —
no docs needed), and the `impl zed::Extension for BevyColorExtension`
block. All public surface is the `register_extension!` macro call.
Nothing else to document. Verify:

```sh
RUSTDOCFLAGS="-D warnings" cargo doc -p zed-bevy-color-extension --no-deps
```
Expected: no missing-docs warnings (the warn-level lint suffices; not
deny).

### Task 4.13: Verify full rustdoc green

- [ ] **Step 1: Run rustdoc with -D warnings**

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```
Expected: no warnings, builds successfully.

### Task 4.14: Update README.md and CONTRIBUTING.md

**Files:**
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`

- [ ] **Step 1: README — add logging row under "Other editors"**

Find the `### Other editors` section. After the fenced code block, add:

```markdown
Set `BEVY_COLOR_LSP_LOG` to control log verbosity (stderr), e.g.:

```sh
BEVY_COLOR_LSP_LOG=debug bevy-color-lsp
```

Defaults to `info`; uses the [`tracing_subscriber::EnvFilter`
syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).
```

- [ ] **Step 2: README — add rustdoc hint to "For contributors"**

After the quick-start block at the bottom, add:

```markdown
Browse API docs with:

```sh
cargo doc --workspace --no-deps --open
```
```

- [ ] **Step 3: CONTRIBUTING — docs gate note**

Under the new "Lint strictness" subsection added in Task 2.6, append:

```markdown
### Documentation

`crates/bevy-color-lsp/src/lib.rs` declares `#![deny(missing_docs)]` —
every public item in that crate must have a `///` doc comment. The
`lint.yml` `docs` job runs `cargo doc --no-deps` with
`RUSTDOCFLAGS=-D warnings` and blocks PRs on any missing doc or
broken intra-doc link.

The Zed extension crate only uses `#![warn(missing_docs)]` (its public
surface is the `register_extension!` macro entry point).
```

- [ ] **Step 4: CONTRIBUTING — update "Keeping this file current"**

Append to the list:

```markdown
- `docs/superpowers/` specs and plans that document ongoing initiatives.
```

### Task 4.15: Flip `lint.yml` pedantic job to blocking

**Files:**
- Modify: `.github/workflows/lint.yml`

- [ ] **Step 1: Confirm pedantic passes locally**

```sh
cargo clippy --workspace --all-targets -- \
  -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo
```
Expected: exit 0. If not, fix or add a narrow `#[allow(clippy::LINT)]
// reason…` on each remaining hit. Do NOT blanket-allow groups.

- [ ] **Step 2: Remove `continue-on-error: true` from pedantic job**

In `.github/workflows/lint.yml`, delete the two lines:

```yaml
    # Stream 4 flips this to `false` after pedantic-clean codebase.
    continue-on-error: true
```

The `pedantic` job now gates PRs alongside `cargo-deny`, `docs`, and
`typos`. `nightly-fmt` stays non-blocking.

### Task 4.16: Final verification and commit Stream 4

- [ ] **Step 1: Full local CI**

```sh
just ci
```
Expected: every recipe passes.

- [ ] **Step 2: Commit**

```sh
git add crates/bevy-color-lsp/src/ \
  crates/zed-extension/src/lib.rs \
  README.md CONTRIBUTING.md \
  .github/workflows/lint.yml
git commit -m "docs: rustdoc coverage + deny(missing_docs) on lsp lib

Crate-level //! on both libs. /// on every pub item in
bevy-color-lsp (color, palette, document, detectors, server, error,
num, parser). # Errors / # Panics sections where relevant. Zed
extension uses warn(missing_docs); lsp crate uses deny.

README/CONTRIBUTING updated: BEVY_COLOR_LSP_LOG, 'cargo doc --open',
docs gate, MSRV 1.87. lint.yml pedantic job flipped to blocking.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review Notes (done during plan authoring)

**Spec coverage:** every section of the spec maps to at least one task.

| Spec section | Tasks |
|---|---|
| Stream 1 — Lint config | 1.1 – 1.9 |
| Stream 2 — DevX tooling | 2.1 – 2.7 |
| Stream 3 — Code hardening (A–G) | 3.1 – 3.17 |
| Stream 4 — Docs | 4.1 – 4.16 |
| Testing strategy (num.rs, Error Display, docs) | 3.3, 3.6, 4.13 |
| CI impact (fast ci.yml unchanged, new lint.yml) | 2.5, 4.15 |
| Rollout (4 commits in order) | 1.9, 2.7, 3.17, 4.16 |

**Placeholder scan:** "TBD"/"TODO"/"later" appear only in two
intentional contexts — the Stream 1 TODO allow comments on Mutex
unwraps (removed in Task 3.5) and "Stream 3 removes them" commit
message. No unresolved placeholders.

**Type consistency:** `u32_to_usize`, `usize_to_u32_sat`,
`f32_to_u8_clamped`, `f32_to_u32_floor_clamped` names are identical
across the `num.rs` definition, call-site rewrites, and tests.
`Error::{InvalidHex, PositionOutOfBounds { line, character },
OffsetOverflow, GrammarLoad, Io}` variant names and fields match
between `error.rs` definition and the Display test.
