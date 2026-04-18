use tree_sitter::{Parser, Tree};

pub fn rust_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("load tree-sitter-rust grammar");
    parser
}

pub fn parse(source: &str) -> Option<Tree> {
    rust_parser().parse(source, None)
}
