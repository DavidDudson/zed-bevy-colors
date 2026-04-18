//! Detect Bevy `Color::hex("...")` call expressions.

use crate::color::parse_hex;
use crate::detectors::ColorMatch;
use crate::num::u32_to_usize;
use std::ops::Range;
use std::sync::LazyLock;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

const QUERY_SRC: &str = r#"
(call_expression
  function: (scoped_identifier
    path: (identifier) @type
    name: (identifier) @ctor
    (#match? @type "^(Color|Srgba|LinearRgba)$")
    (#eq? @ctor "hex"))
  arguments: (arguments
    (string_literal
      (string_content) @hex))) @call
"#;

// `QUERY_SRC` is a `const &str`; `Query::new` only errors on a syntax
// bug in the source, which the unit tests would catch immediately. A
// failure here is a build-time authoring bug.
#[allow(clippy::expect_used)]
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC).expect("compile bevy_hex query")
});

const HEX_TYPES: &[&str] = &["Color", "Srgba", "LinearRgba"];

/// Scan `tree`/`source` for `Color::hex("…")` calls and push [`ColorMatch`] results into `out`.
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
        let mut ty = "";
        let mut ctor = "";
        let mut hex_text = "";
        let mut call_start = 0;
        let mut call_end = 0;
        for cap in m.captures {
            let cap_name = &QUERY.capture_names()[u32_to_usize(cap.index)];
            let text = cap.node.utf8_text(bytes).unwrap_or("");
            match *cap_name {
                "type" => ty = text,
                "ctor" => ctor = text,
                "hex" => hex_text = text,
                "call" => {
                    call_start = cap.node.start_byte();
                    call_end = cap.node.end_byte();
                }
                _ => {}
            }
        }
        if !HEX_TYPES.contains(&ty) || ctor != "hex" {
            continue;
        }
        if let Some(color) = parse_hex(hex_text) {
            out.push(ColorMatch { start_byte: call_start, end_byte: call_end, color });
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
    fn srgba_hex_six() {
        let m = detect_str(r#"Srgba::hex("FF8800")"#);
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.g - 0.533).abs() < 0.01);
    }

    #[test]
    fn color_hex_with_hash() {
        let m = detect_str(r##"Color::hex("#abc")"##);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn ignores_non_hex_ctor() {
        let m = detect_str(r#"Color::srgb("FF8800")"#);
        assert!(m.is_empty());
    }
}
