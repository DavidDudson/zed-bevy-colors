use std::cell::RefCell;
use tree_sitter::{Parser, Tree};

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(make_parser());
}

fn make_parser() -> Parser {
    let mut parser = Parser::new();
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
