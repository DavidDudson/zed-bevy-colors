//! Detect Bevy `Color::<ctor>(...)` call expressions.

use std::{ops::Range, sync::LazyLock};

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

use crate::{
    color::{hsl_to_rgb, hsv_to_rgb, hwb_to_rgb, oklab_to_rgb, oklch_to_rgb, Rgba},
    detectors::ColorMatch,
    num::{f32_to_u8_clamped, u32_to_usize},
};

const QUERY_SRC: &str = r"
(call_expression
  function: (scoped_identifier
    path: (identifier) @type
    name: (identifier) @ctor)
  arguments: (arguments) @args) @call
";

// `QUERY_SRC` is a `const &str`; `Query::new` only errors on a syntax
// bug in the source, which the unit tests would catch immediately. A
// failure here is a build-time authoring bug.
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

/// Scan `tree`/`source` for Bevy constructor calls and push [`ColorMatch`] results into `out`.
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
            let name = &QUERY.capture_names()[u32_to_usize(cap.index)];
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
            if c.is_named() {
                inner = Some(c);
            } else if c.utf8_text(bytes).ok() == Some("-") {
                sign = -1.0;
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
    // Dispatch on the last byte: every Rust integer/float suffix terminates
    // in one of `2 4 6 8 e`. One char-match lets us skip the 14-probe loop
    // entirely on any literal that doesn't carry a suffix (common case).
    let Some(&last) = s.as_bytes().last() else {
        return s;
    };
    let candidates: &[&str] = match last {
        b'2' => &["f32", "i32", "u32"],
        b'4' => &["f64", "i64", "u64"],
        b'6' => &["i16", "u16"],
        b'8' => &["i8", "u8", "i128", "u128"],
        b'e' => &["isize", "usize"],
        _ => return s,
    };
    for suf in candidates {
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
        ("Color", "srgb" | "linear_rgb") | ("Srgba" | "LinearRgba", "rgb") => {
            let (r, g, b) = (get(0)?, get(1)?, get(2)?);
            if ty == "LinearRgba" || ctor == "linear_rgb" {
                Some(Rgba::from_linear(r, g, b, 1.0))
            } else {
                Some(Rgba::new(r, g, b, 1.0))
            }
        }
        ("Color", "srgba" | "linear_rgba") | ("Srgba" | "LinearRgba", "new") => {
            let (r, g, b, a) = (get(0)?, get(1)?, get(2)?, get_or_one(3));
            if ty == "LinearRgba" || ctor == "linear_rgba" {
                Some(Rgba::from_linear(r, g, b, a))
            } else {
                Some(Rgba::new(r, g, b, a))
            }
        }
        ("Color", "srgb_u8") | ("Srgba", "rgb_u8") => Some(Rgba::from_u8(
            f32_to_u8_clamped(get(0)?),
            f32_to_u8_clamped(get(1)?),
            f32_to_u8_clamped(get(2)?),
            255,
        )),
        ("Color", "srgba_u8") | ("Srgba", "rgba_u8") => Some(Rgba::from_u8(
            f32_to_u8_clamped(get(0)?),
            f32_to_u8_clamped(get(1)?),
            f32_to_u8_clamped(get(2)?),
            f32_to_u8_clamped(get(3).unwrap_or(255.0)),
        )),
        ("Color" | "Hsla", "hsl") => Some(hsl_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
        ("Color", "hsla") | ("Hsla", "new") => {
            Some(hsl_to_rgb(get(0)?, get(1)?, get(2)?, get_or_one(3)))
        }
        ("Color" | "Hsva", "hsv") => Some(hsv_to_rgb(get(0)?, get(1)?, get(2)?, 1.0)),
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

    // --- Color::srgba / srgba_u8 ---

    #[test]
    fn srgba_with_alpha() {
        let m = detect_str("Color::srgba(1.0, 0.0, 0.5, 0.8)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.a - 0.8).abs() < 0.01);
    }

    #[test]
    fn srgba_u8_with_alpha() {
        let m = detect_str("Color::srgba_u8(255, 0, 128, 200)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.a - 200.0 / 255.0).abs() < 0.01);
    }

    // --- LinearRgba / linear_rgb / linear_rgba ---

    #[test]
    fn linear_rgb_color() {
        let m = detect_str("Color::linear_rgb(1.0, 1.0, 1.0)");
        assert_eq!(m.len(), 1);
        // linear white -> sRGB white
        assert!((m[0].color.r - 1.0).abs() < 0.01);
    }

    #[test]
    fn linear_rgba_color() {
        let m = detect_str("Color::linear_rgba(1.0, 1.0, 1.0, 0.5)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn linear_rgba_struct_rgb() {
        let m = detect_str("LinearRgba::rgb(0.5, 0.5, 0.5)");
        assert_eq!(m.len(), 1);
        // 0.5 linear -> ~0.735 sRGB
        assert!(m[0].color.r > 0.7 && m[0].color.r < 0.76);
    }

    #[test]
    fn linear_rgba_struct_new() {
        let m = detect_str("LinearRgba::new(1.0, 0.0, 0.0, 1.0)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.g).abs() < 0.01);
    }

    // --- Hsla::new ---

    #[test]
    fn hsla_struct_new() {
        let m = detect_str("Hsla::new(0.0, 1.0, 0.5, 0.7)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01); // red hue
        assert!((m[0].color.a - 0.7).abs() < 0.01);
    }

    // --- Color::hsla ---

    #[test]
    fn hsla_color() {
        let m = detect_str("Color::hsla(120.0, 1.0, 0.5, 0.5)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.g - 1.0).abs() < 0.01); // green
        assert!((m[0].color.a - 0.5).abs() < 0.01);
    }

    // --- Color::hsv / Hsva::new ---

    #[test]
    fn hsv_color() {
        let m = detect_str("Color::hsv(240.0, 1.0, 1.0)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.b - 1.0).abs() < 0.01); // blue
    }

    #[test]
    fn hsva_struct_new() {
        let m = detect_str("Hsva::new(240.0, 1.0, 1.0, 0.6)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.b - 1.0).abs() < 0.01);
        assert!((m[0].color.a - 0.6).abs() < 0.01);
    }

    // --- Color::hwb / Hwba::new ---

    #[test]
    fn hwb_color() {
        let m = detect_str("Color::hwb(0.0, 0.0, 0.0)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01); // red
    }

    #[test]
    fn hwba_struct_new() {
        let m = detect_str("Hwba::new(0.0, 0.5, 0.5, 1.0)");
        assert_eq!(m.len(), 1);
        // w+b=1 => gray 0.5
        assert!((m[0].color.r - 0.5).abs() < 0.01);
    }

    // --- Oklaba::new / Color::oklch / Oklcha::new ---

    #[test]
    fn oklaba_struct_new() {
        let m = detect_str("Oklaba::new(0.5, 0.0, 0.0, 1.0)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn oklch_color() {
        let m = detect_str("Color::oklch(0.5, 0.0, 0.0)");
        assert_eq!(m.len(), 1);
        // zero chroma => near-gray
        assert!((m[0].color.r - m[0].color.g).abs() < 0.05);
    }

    #[test]
    fn oklcha_struct_new() {
        let m = detect_str("Oklcha::new(0.5, 0.0, 0.0, 0.9)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.a - 0.9).abs() < 0.01);
    }

    // --- Srgba::rgba_u8 ---

    #[test]
    fn srgba_rgba_u8_struct() {
        let m = detect_str("Srgba::rgba_u8(255, 128, 0, 200)");
        assert_eq!(m.len(), 1);
        assert!((m[0].color.r - 1.0).abs() < 0.01);
        assert!((m[0].color.g - 128.0 / 255.0).abs() < 0.01);
        assert!((m[0].color.a - 200.0 / 255.0).abs() < 0.01);
    }

    // --- unrecognised type/ctor yields no match ---

    #[test]
    fn unknown_ctor_yields_no_match() {
        let m = detect_str("Color::unknown_ctor(1.0, 0.0, 0.0)");
        assert!(m.is_empty());
    }
}
