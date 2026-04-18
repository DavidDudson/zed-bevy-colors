use crate::color::{hsl_to_rgb, hsv_to_rgb, hwb_to_rgb, oklab_to_rgb, oklch_to_rgb, Rgba};
use crate::detectors::ColorMatch;
use std::ops::Range;
use std::sync::LazyLock;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

const QUERY_SRC: &str = r#"
(call_expression
  function: (scoped_identifier
    path: (identifier) @type
    name: (identifier) @ctor)
  arguments: (arguments) @args) @call
"#;

#[allow(clippy::expect_used)]
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&tree_sitter_rust::LANGUAGE.into(), QUERY_SRC).expect("compile bevy_ctor query")
});

const COLOR_TYPES: &[&str] = &[
    "Color",
    "Srgba",
    "LinearRgba",
    "Hsla",
    "Hsva",
    "Hwba",
    "Laba",
    "Lcha",
    "Oklaba",
    "Oklcha",
    "Xyza",
];

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
        let mut args: Option<Node<'_>> = None;
        let mut call: Option<Node<'_>> = None;
        for cap in m.captures {
            let name = &QUERY.capture_names()[cap.index as usize];
            let text = cap.node.utf8_text(bytes).unwrap_or("");
            match *name {
                "type" => ty = text,
                "ctor" => ctor = text,
                "args" => args = Some(cap.node),
                "call" => call = Some(cap.node),
                _ => {}
            }
        }
        if !COLOR_TYPES.contains(&ty) {
            continue;
        }
        let Some(args) = args else { continue };
        let Some(call) = call else { continue };
        let nums = collect_number_args(args, bytes);
        if let Some(color) = build_color(ty, ctor, &nums) {
            out.push(ColorMatch {
                start_byte: call.start_byte(),
                end_byte: call.end_byte(),
                color,
            });
        }
    }
}

fn collect_number_args(args: Node<'_>, bytes: &[u8]) -> Vec<f32> {
    let mut nums = Vec::new();
    let mut walker = args.walk();
    for child in args.named_children(&mut walker) {
        if let Some(v) = parse_number(child, bytes) {
            nums.push(v);
        }
    }
    nums
}

fn parse_number(node: Node<'_>, bytes: &[u8]) -> Option<f32> {
    let kind = node.kind();
    if kind == "float_literal" || kind == "integer_literal" || kind == "negative_literal" {
        let text = node.utf8_text(bytes).ok()?;
        return parse_numeric_literal(text);
    }
    if kind == "unary_expression" {
        let mut sign: f32 = 1.0;
        let mut inner = None;
        let mut w = node.walk();
        for c in node.children(&mut w) {
            if !c.is_named() {
                if c.utf8_text(bytes).ok() == Some("-") {
                    sign = -1.0;
                }
            } else {
                inner = Some(c);
            }
        }
        let v = parse_number(inner?, bytes)?;
        return Some(sign * v);
    }
    None
}

fn parse_numeric_literal(text: &str) -> Option<f32> {
    let cleaned: String = text.chars().filter(|c| *c != '_').collect();
    let cleaned = strip_rust_suffix(&cleaned);
    cleaned.parse::<f32>().ok()
}

fn strip_rust_suffix(s: &str) -> &str {
    for suf in [
        "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize",
    ] {
        if let Some(stripped) = s.strip_suffix(suf) {
            return stripped;
        }
    }
    s
}

fn build_color(ty: &str, ctor: &str, n: &[f32]) -> Option<Rgba> {
    let get = |i: usize| n.get(i).copied();
    let get_or_one = |i: usize| n.get(i).copied().unwrap_or(1.0);
    match (ty, ctor) {
        ("Color", "srgb") | ("Color", "linear_rgb") | ("Srgba", "rgb") | ("LinearRgba", "rgb") => {
            let (r, g, b) = (get(0)?, get(1)?, get(2)?);
            if ty == "LinearRgba" || ctor == "linear_rgb" {
                Some(Rgba::from_linear(r, g, b, 1.0))
            } else {
                Some(Rgba::new(r, g, b, 1.0))
            }
        }
        ("Color", "srgba")
        | ("Color", "linear_rgba")
        | ("Srgba", "new")
        | ("LinearRgba", "new") => {
            let (r, g, b, a) = (get(0)?, get(1)?, get(2)?, get_or_one(3));
            if ty == "LinearRgba" || ctor == "linear_rgba" {
                Some(Rgba::from_linear(r, g, b, a))
            } else {
                Some(Rgba::new(r, g, b, a))
            }
        }
        ("Color", "srgb_u8") | ("Srgba", "rgb_u8") => {
            Some(Rgba::from_u8(get(0)? as u8, get(1)? as u8, get(2)? as u8, 255))
        }
        ("Color", "srgba_u8") | ("Srgba", "rgba_u8") => Some(Rgba::from_u8(
            get(0)? as u8,
            get(1)? as u8,
            get(2)? as u8,
            get(3).unwrap_or(255.0) as u8,
        )),
        ("Color", "hsl") | ("Hsla", "hsl") => Some(hsl_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Color", "hsla") | ("Hsla", "new") => {
            Some(hsl_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3)))
        }
        ("Color", "hsv") | ("Hsva", "hsv") => Some(hsv_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Hsva", "new") => Some(hsv_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3))),
        ("Color", "hwb") => Some(hwb_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Hwba", "new") => Some(hwb_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3))),
        ("Color", "oklab") => Some(oklab_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Oklaba", "new") => Some(oklab_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3))),
        ("Color", "oklch") => Some(oklch_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Oklcha", "new") => Some(oklch_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3))),
        _ => None,
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
    fn srgb() {
        let m = detect_str("let c = Color::srgb(1.0, 0.5, 0.0);");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.g - 0.5).abs() < 0.01);
        assert!((m[0].color.b - 0.0).abs() < 0.01);
    }

    #[test]
    fn srgb_u8() {
        let m = detect_str("Color::srgb_u8(255, 128, 0)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.g - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn hsl_cyan() {
        let m = detect_str("Color::hsl(180.0, 1.0, 0.5)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.g - 1.0).abs() < 0.01);
        assert!((m[0].color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn srgba_struct() {
        let m = detect_str("Srgba::new(0.1, 0.2, 0.3, 0.4)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.a - 0.4).abs() < 0.01);
    }

    #[test]
    fn ignores_non_color_call() {
        let m = detect_str("Vec3::new(1.0, 2.0, 3.0)");
        assert!(m.is_empty());
    }

    #[test]
    fn handles_negative() {
        let m = detect_str("Color::oklab(0.5, -0.1, 0.1)");
        assert_eq!(m.len(), 1);
    }
}
