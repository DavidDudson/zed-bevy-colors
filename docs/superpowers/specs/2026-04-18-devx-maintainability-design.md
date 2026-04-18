# DevX & Maintainability Hardening — Design

**Date:** 2026-04-18
**Status:** Approved for implementation planning
**Scope:** `zed-bevy-color` workspace (bevy-color-lsp + zed-extension)

## Goal

Raise DevX, maintainability, and strictness without regressing build/test
time on the fast CI path. Remove avoidable runtime panics from production
code paths (prod only — tests keep idiomatic `unwrap()`). Add strict lint
configuration enforced locally and in CI. Introduce a proper error type,
structured logging, crate-level docs, and a task runner.

## Non-Goals

- Removing `unwrap()` / `expect()` / `panic!` from `#[cfg(test)]` code or
  `benches/`. These are idiomatic and removing them is pure churn.
- Any API / semver-breaking change to published artifacts beyond MSRV bump.
- Observability beyond stderr `tracing` output. No OTel, no metrics.
- Publishing `bevy-color-lsp` to crates.io.

## Constraints

- Workspace stays on stable Rust.
- LSP binary + WASM extension — only 64-bit `usize` platforms supported at
  runtime for LSP; WASM extension uses only `zed_extension_api`, no num work.
- Must not lengthen `ci.yml` fast path (fmt/clippy/test/wasm) more than
  marginally. New strict checks live in a parallel `lint.yml`.
- MSRV bumps from implicit-stable to explicit **1.87** (for
  `usize::cast_signed` / `checked_add_signed` stable usage).
- Zero new `unsafe` code. `unsafe_code = "forbid"` workspace-wide.

## Architecture

Four parallel work streams, each independently mergeable as one PR:

1. **Lint config** — workspace `[lints]`, `rustfmt.toml`, `clippy.toml`,
   per-crate test overrides.
2. **DevX tooling** — `Justfile`, `lefthook.yml`, `deny.toml`, flake
   devshell updates, new `lint.yml` CI workflow.
3. **Code hardening** — Mutex swap to `parking_lot`, numeric-cast module,
   `thiserror` error enum, `tracing` integration, static-init expect
   justifications.
4. **Docs** — crate-level `//!`, `///` on all `pub` items in
   `bevy-color-lsp`, `#![deny(missing_docs)]` gate, CONTRIBUTING updates.

Order of execution does not matter across streams; inside each stream,
order is the natural dependency order.

## Stream 1 — Lint Config

### Root `Cargo.toml` additions

```toml
[workspace.package]
rust-version = "1.87"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
rust_2018_idioms = { level = "warn", priority = -1 }
unreachable_pub = "warn"
trivial_casts = "warn"
trivial_numeric_casts = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
# Pedantic/nursery/cargo stay opt-in so the fast CI path (`ci.yml`, which
# runs clippy with `-D warnings`) is not broken by opinionated nudges.
# `lint.yml` opts in via explicit `-W clippy::pedantic` etc.
pedantic = { level = "allow", priority = -1 }
nursery = { level = "allow", priority = -1 }
cargo = { level = "allow", priority = -1 }
# Warn-level in workspace + `-D warnings` in fast CI = blocks new prod
# unwraps/expects/panics. Tests/benches opt out via `#![allow]`.
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "warn"
unimplemented = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
```

Each crate `Cargo.toml` gains `[lints] workspace = true`.

### `rustfmt.toml` (root)

```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
newline_style = "Unix"
```

`imports_granularity` and `group_imports` are nightly-only. Fast CI uses
stable `cargo fmt --check`; `lint.yml` adds a `continue-on-error: true`
nightly-fmt job.

### `clippy.toml` (root)

```toml
msrv = "1.87"
cognitive-complexity-threshold = 20
too-many-lines-threshold = 120
type-complexity-threshold = 250
```

### Test/bench allow overrides

Integration test (`tests/lsp_integration.rs`) and bench (`benches/pipeline.rs`)
files gain top-level:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::pedantic, clippy::nursery)]
```

Unit tests within source files use `#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]`
on each `mod tests`.

## Stream 2 — DevX Tooling

### `Justfile` (root)

```makefile
default: check

check: fmt-check clippy test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery

test:
    cargo test --workspace

bench:
    cargo bench -p bevy-color-lsp

wasm:
    cargo build -p zed-extension --target wasm32-wasip2 --release

deny:
    cargo deny check

docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

ci: fmt-check clippy test deny docs wasm

watch:
    cargo watch -x 'clippy --workspace --all-targets'
```

### `lefthook.yml` (root)

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

### `deny.toml` (root)

```toml
[advisories]
version = 2
yanked = "deny"
ignore = []

[licenses]
version = 2
allow = [
  "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause", "BSD-3-Clause",
  "ISC", "Unicode-DFS-2016", "Unicode-3.0",
  "Zlib", "CC0-1.0", "MPL-2.0"
]
confidence-threshold = 0.9

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

### Nix flake (`flake.nix`)

Add to devshell `buildInputs`: `lefthook`, `cargo-deny`, `typos`, `taplo`.

Add to `shellHook`:

```sh
if [ -d .git ] && [ ! -f .git/hooks/pre-commit ]; then
  lefthook install >/dev/null
fi
```

### `.github/workflows/lint.yml` (new)

```yaml
name: lint
on:
  pull_request:
  push:
    branches: [main]
jobs:
  cargo-deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
  pedantic:
    runs-on: ubuntu-latest
    # Flipped to `false` at end of Stream 4; see Rollout section.
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - run: cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo
  docs:
    runs-on: ubuntu-latest
    env:
      RUSTDOCFLAGS: "-D warnings"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
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
        with: { components: rustfmt }
      - run: cargo +nightly fmt --all -- --check
```

`ci.yml` stays as-is (fast path: stable fmt-check, clippy with
`-D warnings` default group, test, WASM build).

## Stream 3 — Code Hardening

### A. Mutex poison elimination

Replace `std::sync::Mutex` with `parking_lot::Mutex` in
`crates/bevy-color-lsp/src/document.rs`. `parking_lot::Mutex::lock()`
returns the guard directly — no `Result`, no poisoning.

Lines affected: `document.rs:296, 300, 306, 312, 318` — all drop
`.unwrap()`. Add `parking_lot = "0.12"` dep.

### B. Numeric-cast module `num.rs`

New file `crates/bevy-color-lsp/src/num.rs`:

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
pub fn u32_to_usize(x: u32) -> usize { x as usize }

/// Narrow a `usize` to `u32`, saturating at `u32::MAX`.
#[inline]
#[must_use]
pub fn usize_to_u32_sat(x: usize) -> u32 {
    if x > u32::MAX as usize { u32::MAX } else { x as u32 }
}

/// Convert a normalized `f32` channel value to a `u8`, rounding to
/// nearest and clamping to `[0, 255]`. Never panics.
#[inline]
#[must_use]
pub fn f32_to_u8_clamped(x: f32) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

/// Convert a non-negative `f32` to `u32`, flooring and clamping to `max`.
#[inline]
#[must_use]
pub fn f32_to_u32_floor_clamped(x: f32, max: u32) -> u32 {
    x.floor().clamp(0.0, max as f32) as u32
}
```

### C. Call-site rewrites (prod only)

| Location | Before | After |
|----------|--------|-------|
| `server.rs:90-92` | `(c.red * 255.0).round() as u8` | `num::f32_to_u8_clamped(c.red * 255.0)` |
| `document.rs:156-157,181` | `p.line as usize` / `p.character as usize` | `num::u32_to_usize(...)` |
| `document.rs:199,242,279` | `ch.len_utf16() as u32` | `num::usize_to_u32_sat(ch.len_utf16())` |
| `document.rs:45` | `new_end_byte as isize - old_end_byte as isize` | `new_end_byte.cast_signed() - old_end_byte.cast_signed()` (stable 1.87) |
| `document.rs:124-125` | `(m.start_byte as isize + delta) as usize` | `m.start_byte.checked_add_signed(delta).ok_or(Error::OffsetOverflow)?` |
| `color.rs:20-23` | `r as f32 / 255.0` | `f32::from(r) / 255.0` |
| `color.rs:124` | `hp as u32` | `num::f32_to_u32_floor_clamped(hp, 6)` |
| `detectors/*:cap.index as usize` | `cap.index as usize` | `num::u32_to_usize(cap.index)` |
| `detectors/bevy_ctor.rs:156-164` | `get(N)? as u8` | `num::f32_to_u8_clamped(get(N)?)` |

### D. Error type `error.rs`

New file `crates/bevy-color-lsp/src/error.rs`:

```rust
use thiserror::Error;

/// Errors surfaced by `bevy-color-lsp` internals.
#[derive(Debug, Error)]
pub enum Error {
    /// Input was not a valid hex color literal.
    #[error("invalid hex color literal: {0:?}")]
    InvalidHex(String),
    /// LSP position references a location outside the current document.
    #[error("position out of document bounds: line={line} character={character}")]
    PositionOutOfBounds { line: u32, character: u32 },
    /// Byte offset arithmetic overflowed when applying an incremental edit.
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
```

`parse_hex` and friends that currently return `Option<Color>` keep
`Option` where absence is a normal outcome. Functions where "failed vs
not found" is meaningful (document byte-offset math, position
translation) return `Result<_, Error>`.

LSP boundary in `server.rs` maps `Error` to
`tower_lsp::jsonrpc::Error::internal_error()` with a logged span:

```rust
fn to_lsp(err: Error) -> tower_lsp::jsonrpc::Error {
    tracing::warn!(error = %err, "internal error");
    tower_lsp::jsonrpc::Error::internal_error()
}
```

### E. `tracing` integration

`main.rs`:

```rust
use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("BEVY_COLOR_LSP_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime must build at startup");
    rt.block_on(run());
}
```

Server methods get `#[tracing::instrument(skip(self), err)]` on
`document_color`, `color_presentation`, `did_change`, `did_open`,
`did_close`. Silent drops on `Option::None` where a warning would aid
debugging become `tracing::debug!`.

### F. Static-init `expect()` retention

Five static-init expects (`parser.rs:12`, `detectors/*/QUERY`) remain.
Each is wrapped with:

```rust
#[allow(clippy::expect_used)]
// Query source is const; `Query::new` only fails on malformed source,
// which is a build-time authoring bug surfaced by the test suite.
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC)
        .expect("compile bevy_hex query")
});
```

A `# Panics` rustdoc note is added to any function that reaches these
statics on first call.

### G. Dependency additions (`bevy-color-lsp/Cargo.toml`)

```toml
parking_lot = "0.12"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Stream 4 — Docs

### Crate-level `//!`

`crates/bevy-color-lsp/src/lib.rs`:

```rust
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::missing_crate_level_docs)]

//! # bevy-color-lsp
//!
//! LSP server providing color decoration and editing for `bevy::color`
//! values in Rust source. Implements `textDocument/documentColor` and
//! `textDocument/colorPresentation`.
//!
//! ## Modules
//! - [`color`] — hex/RGB/HSL color types + parsing.
//! - [`palette`] — CSS named + Tailwind palette lookup.
//! - [`document`] — incremental tree-sitter document store.
//! - [`detectors`] — per-pattern extractors.
//! - [`server`] — `tower-lsp` request handlers.
//! - [`error`] — crate error type.
//! - [`num`] — auditable numeric conversions.
//!
//! ## Logging
//! Set `BEVY_COLOR_LSP_LOG=debug` for detector trace output on stderr.
```

`crates/zed-extension/src/lib.rs`:

```rust
#![warn(missing_docs)]

//! # zed-bevy-color extension
//!
//! Zed extension shim that locates the `bevy-color-lsp` binary and
//! starts it for Rust files. Compiled to `wasm32-wasip2`.
```

### `pub` item coverage

Every `pub fn`, `pub struct`, `pub enum`, `pub const` in
`bevy-color-lsp` gets `///`. Targets:

- `color`: `Color`, `parse_hex`, `hsl_to_rgb`, accessors.
- `palette`: `css_named`, `tailwind_named` — note case-insensitivity.
- `document`: `Document`, `DocumentStore`, `position_to_byte`,
  `byte_to_position` — call out UTF-16 offset semantics.
- `detectors`: each `detect_*` fn — input window semantics documented.
- `error`: variants + `Result` alias.
- `num`: per-fn one-liner (module doc already set).

Fns returning `Result` gain `/// # Errors` sections. Static-init
callers gain `/// # Panics — at startup only` notes to satisfy
`clippy::missing_panics_doc`.

### README + CONTRIBUTING updates

`README.md`:
- New "Logging" row in env vars table: `BEVY_COLOR_LSP_LOG`.
- `cargo doc --open` hint in quick-start.

`CONTRIBUTING.md` — append:
- `Justfile` recipe reference table.
- `lefthook install` post-clone step (auto in nix devshell).
- `cargo deny check` expectation pre-PR.
- Lint strictness note: `clippy::all` hard-denied; `pedantic`/`nursery`
  warn; new warnings must be fixed or justified with `#[allow]` +
  explanation.
- `#![deny(missing_docs)]` on lsp lib — all `pub` items documented.
- MSRV: 1.87 (bumped from implicit stable).

## Testing Strategy

- **No behavior change.** Every existing unit/integration test must
  continue passing unchanged.
- **New tests for `num.rs`:** one per helper, covering boundary inputs
  (0.0, 255.0, negative, NaN, +Inf, `u32::MAX`, `usize::MAX`).
- **New test for `Error` display:** assert each variant formats cleanly.
- **Doc tests:** any `/// # Examples` added are gated through
  `cargo test --doc`.
- **Benchmarks:** `pipeline.rs` benches re-run locally; parking_lot
  swap and `num::*` helpers must not regress more than 5% (criterion
  diff mode against pre-change baseline).

## CI Impact

- `ci.yml` (fast path): unchanged step definitions. Because workspace
  `[lints.clippy]` sets `unwrap_used`, `expect_used`, `panic`, etc. to
  `warn`, the existing `-D warnings` invocation now also blocks new
  prod unwraps/expects/panics — but pedantic/nursery/cargo remain
  `allow` so fast CI does not become noisy.
- `lint.yml` (new, parallel): cargo-deny, pedantic clippy
  (`-W clippy::pedantic -W clippy::nursery -W clippy::cargo -D warnings`),
  rustdoc (`RUSTDOCFLAGS=-D warnings`), typos, nightly-fmt
  (non-blocking).
- PR merge gate: `ci.yml` required; `lint.yml` jobs except
  `nightly-fmt` required.

## Rollout

Four PRs, merged in order but independently reviewable:

1. **Stream 1** (lint config) — `Cargo.toml` workspace `[lints]`, new
   `rustfmt.toml` / `clippy.toml`. Fast CI now blocks new prod
   `unwrap`/`expect`/`panic`. Five existing prod Mutex unwraps get
   temporary `#[allow(clippy::unwrap_used)] // TODO(Stream 3)` — removed
   in Stream 3.
2. **Stream 2** (DevX tooling) — `Justfile`, `lefthook.yml`,
   `deny.toml`, flake update. `lint.yml` workflow shipped but with the
   pedantic job set to `continue-on-error: true` initially (pedantic
   will light up until Stream 3 lands).
3. **Stream 3** (code hardening) — Mutex swap, `num.rs`, cast rewrites,
   `error.rs`, `tracing`. Removes Stream 1's TODO allows. Any
   remaining pedantic warnings are fixed or justified with localized
   `#[allow]` + comment.
4. **Stream 4** (docs) — crate `//!`, pub-item `///`,
   `#![deny(missing_docs)]`. Flip `lint.yml` pedantic job to
   blocking (`continue-on-error: false`).

## Open Questions

None. All scope decisions resolved during brainstorming:

- Prod-only unwrap/cast removal (chose A).
- DevX: Justfile + lefthook + rustfmt.toml + clippy.toml + deny.toml (a+b+c+d).
- Strict lints (option 2).
- thiserror + tracing (option iii).
- Full pub-item docs with `missing_docs = deny` on lsp lib (option y).
- Split CI: fast `ci.yml` + parallel `lint.yml` (option q).
