//! Bevy Color LSP — a `textDocument/documentColor` language-server for
//! Bevy `Color` literals.
//!
//! The server speaks LSP over stdin/stdout via [`tower_lsp`].
//! Tree-sitter is used for syntactic detection without full type resolution.
//!
//! # Crate layout
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`color`] | `Rgba` type + color-space conversion helpers (built on [`palette`]) |
//! | [`named_colors`] | Named color tables (CSS, Tailwind, basic) |
//! | [`parser`] | Tree-sitter parser wrappers |
//! | [`detectors`] | Pattern matchers that produce [`detectors::ColorMatch`] values |
//! | [`document`] | Per-document text + incremental color cache |
//! | [`server`] | Tower-LSP `Backend` + `run` entry point |
//! | [`error`] | Crate error/result types |
//! | [`num`] | Auditable numeric cast helpers |
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::missing_crate_level_docs)]
// Pedantic/nursery lints opted in by `lint.yml` only; individual allows below
// are for lints that are genuinely inapplicable to this codebase.
//
// `many_single_char_names`: color math conventionally uses h, s, l, r, g, b
// etc. Renaming them would hurt readability more than it helps.
#![allow(clippy::many_single_char_names)]
// `suboptimal_flops`: the `mul_add` form changes numeric behavior; keeping
// explicit order matches the reference formulas and aids review.
#![allow(clippy::suboptimal_flops)]
// `multiple_crate_versions`: bitflags 1.x vs 2.x is a transitive dep conflict
// in our dependency tree; we have no direct control over it.
#![allow(clippy::multiple_crate_versions)]
// `cargo_common_metadata`: readme/keywords/categories are managed at release
// time; the binary crate is not published to crates.io.
#![allow(clippy::cargo_common_metadata)]

pub mod color;
pub mod detectors;
pub mod document;
pub mod error;
pub mod named_colors;
pub mod num;
pub mod parser;
pub mod server;
