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

pub fn parse(source: &str) -> Option<Tree> {
    PARSER.with(|cell| cell.borrow_mut().parse(source, None))
}

pub fn parse_incremental(source: &str, old: Option<&Tree>) -> Option<Tree> {
    PARSER.with(|cell| cell.borrow_mut().parse(source, old))
}
