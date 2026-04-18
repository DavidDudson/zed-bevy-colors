//! Detect palette color references such as `palettes::css::TOMATO`.

use crate::detectors::ColorMatch;
use crate::num::u32_to_usize;
use crate::palette::lookup_palette;
use std::ops::Range;
use std::sync::LazyLock;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

const QUERY_SRC: &str = r#"
(scoped_identifier
  path: (scoped_identifier
    path: (_)
    name: (identifier) @module
    (#match? @module "^(css|tailwind|basic)$"))
  name: (identifier) @name
  (#match? @name "^[A-Z][A-Z0-9_]*$")) @full
"#;

// `QUERY_SRC` is a `const &str`; `Query::new` only errors on a syntax
// bug in the source, which the unit tests would catch immediately. A
// failure here is a build-time authoring bug.
#[allow(clippy::expect_used)]
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC).expect("compile palette query")
});

const MODULES: &[&str] = &["css", "tailwind", "basic"];

/// Scan `tree`/`source` for palette color references and push [`ColorMatch`] results into `out`.
pub fn detect(
    tree: &Tree,
    source: &str,
    byte_range: Option<Range<usize>>,
    out: &mut Vec<ColorMatch>,
) {
    let mut cursor = QueryCursor::new();
    if let Some(r) = byte_range {
        cursor.set_byte_range(r);
    }
    let bytes = source.as_bytes();

    let mut matches = cursor.matches(&QUERY, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        let mut module = "";
        let mut name = "";
        let mut full_start = 0;
        let mut full_end = 0;
        for cap in m.captures {
            let cap_name = &QUERY.capture_names()[u32_to_usize(cap.index)];
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
            out.push(ColorMatch { start_byte: full_start, end_byte: full_end, color });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn detect_str(src: &str) -> Vec<ColorMatch> {
        let tree = parse(src).unwrap();
        let mut out = Vec::new();
        detect(&tree, src, None, &mut out);
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
