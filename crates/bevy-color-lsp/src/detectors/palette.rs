use crate::detectors::ColorMatch;
use crate::palette::lookup_palette;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

const QUERY: &str = r#"
(scoped_identifier
  path: (scoped_identifier
    path: (_)
    name: (identifier) @module)
  name: (identifier) @name) @full
"#;

const MODULES: &[&str] = &["css", "tailwind", "basic"];

pub fn detect(tree: &Tree, source: &str, out: &mut Vec<ColorMatch>) {
    let language = tree_sitter_rust::LANGUAGE.into();
    let query = Query::new(&language, QUERY).expect("compile palette query");
    let mut cursor = QueryCursor::new();
    let bytes = source.as_bytes();

    let mut matches = cursor.matches(&query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        let mut module = "";
        let mut name = "";
        let mut full_start = 0;
        let mut full_end = 0;
        for cap in m.captures {
            let cap_name = &query.capture_names()[cap.index as usize];
            let text = cap.node.utf8_text(bytes).unwrap_or("");
            match *cap_name {
                "module" => module = text,
                "name" => name = text,
                "full" => {
                    full_start = cap.node.start_byte();
                    full_end = cap.node.end_byte();
                }
                _ => {}
            }
        }
        if !MODULES.contains(&module) {
            continue;
        }
        if let Some(color) = lookup_palette(module, name) {
            out.push(ColorMatch {
                start_byte: full_start,
                end_byte: full_end,
                color,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn detect_str(src: &str) -> Vec<ColorMatch> {
        let tree = parse(src).unwrap();
        let mut out = Vec::new();
        detect(&tree, src, &mut out);
        out
    }

    #[test]
    fn tailwind_short_path() {
        let m = detect_str("palettes::tailwind::BLUE_500");
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn css_long_path() {
        let m = detect_str("bevy_color::palettes::css::TOMATO");
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn basic_red() {
        let m = detect_str("palettes::basic::RED");
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn ignores_other_module() {
        let m = detect_str("std::collections::HashMap");
        assert!(m.is_empty());
    }
}
