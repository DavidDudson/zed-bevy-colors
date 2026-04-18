use crate::color::Rgba;
use crate::detectors::ColorMatch;
use crate::palette::lookup_named;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

const QUERY: &str = r#"
(scoped_identifier
  path: (identifier) @type
  name: (identifier) @name) @full
"#;

pub fn detect(tree: &Tree, source: &str, out: &mut Vec<ColorMatch>) {
    let language = tree_sitter_rust::LANGUAGE.into();
    let query = Query::new(&language, QUERY).expect("compile bevy_const query");
    let mut cursor = QueryCursor::new();
    let bytes = source.as_bytes();

    let mut matches = cursor.matches(&query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        let mut ty = "";
        let mut name = "";
        let mut full_start = 0;
        let mut full_end = 0;
        for cap in m.captures {
            let cap_name = &query.capture_names()[cap.index as usize];
            let text = cap.node.utf8_text(bytes).unwrap_or("");
            match *cap_name {
                "type" => ty = text,
                "name" => name = text,
                "full" => {
                    full_start = cap.node.start_byte();
                    full_end = cap.node.end_byte();
                }
                _ => {}
            }
        }
        if let Some(color) = lookup(ty, name) {
            out.push(ColorMatch {
                start_byte: full_start,
                end_byte: full_end,
                color,
            });
        }
    }
}

fn lookup(ty: &str, name: &str) -> Option<Rgba> {
    match (ty, name) {
        ("Color" | "Srgba" | "LinearRgba", "WHITE") => Some(Rgba::WHITE),
        ("Color" | "Srgba" | "LinearRgba", "BLACK") => Some(Rgba::BLACK),
        ("Color" | "Srgba" | "LinearRgba", "NONE") => Some(Rgba::NONE),
        ("Srgba" | "Color", n) => lookup_named(n),
        _ => None,
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
    fn white() {
        let m = detect_str("let c = Color::WHITE;");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].color, Rgba::WHITE);
    }

    #[test]
    fn none_const() {
        let m = detect_str("Color::NONE");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].color.a, 0.0);
    }

    #[test]
    fn ignores_non_color() {
        let m = detect_str("Foo::BAR");
        assert!(m.is_empty());
    }
}
