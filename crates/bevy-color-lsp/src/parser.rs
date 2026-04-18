//! Thread-local tree-sitter parser wrappers.
//!
//! The parser and its compiled query are initialized lazily per thread on
//! first use. A grammar-load failure (ABI mismatch) causes a panic at that
//! point — see `# Panics` on individual functions.

use std::cell::RefCell;
use tree_sitter::{Parser, Tree};

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(make_parser());
}

fn make_parser() -> Parser {
    let mut parser = Parser::new();
    // `set_language` returns Err only on an ABI mismatch between the
    // compiled tree-sitter-rust crate and the tree-sitter runtime.
    // Both are pinned in this workspace's `Cargo.lock`, so a failure
    // here is a build-time configuration bug.
    #[allow(clippy::expect_used)]
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("load tree-sitter-rust grammar");
    parser
}

/// Parse `source` from scratch, returning `None` only if the parser times out (no timeout set here).
///
/// # Panics
///
/// Panics at startup only if the `tree-sitter-rust` grammar ABI is
/// incompatible with the linked tree-sitter runtime (a build-time
/// configuration bug; both are pinned in `Cargo.lock`).
#[must_use]
pub fn parse(source: &str) -> Option<Tree> {
    PARSER.with(|cell| cell.borrow_mut().parse(source, None))
}

/// Re-parse `source` reusing the edit-annotated `old` tree for incremental parsing.
///
/// Pass `None` for `old` to force a full parse.
///
/// # Panics
///
/// Panics at startup only if the `tree-sitter-rust` grammar ABI is
/// incompatible with the linked tree-sitter runtime (a build-time
/// configuration bug; both are pinned in `Cargo.lock`).
#[must_use]
pub fn parse_incremental(source: &str, old: Option<&Tree>) -> Option<Tree> {
    PARSER.with(|cell| cell.borrow_mut().parse(source, old))
}
