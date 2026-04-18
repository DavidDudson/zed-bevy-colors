use crate::color::Rgba;
use std::ops::Range;
use tree_sitter::Tree;

pub mod bevy_const;
pub mod bevy_ctor;
pub mod bevy_hex;
pub mod palette;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatch {
    pub start_byte: usize,
    pub end_byte: usize,
    pub color: Rgba,
}

pub fn detect_all(tree: &Tree, source: &str) -> Vec<ColorMatch> {
    detect_in_range(tree, source, None)
}

pub fn detect_in_range(
    tree: &Tree,
    source: &str,
    byte_range: Option<Range<usize>>,
) -> Vec<ColorMatch> {
    let mut out = Vec::new();
    bevy_ctor::detect(tree, source, byte_range.clone(), &mut out);
    bevy_const::detect(tree, source, byte_range.clone(), &mut out);
    bevy_hex::detect(tree, source, byte_range.clone(), &mut out);
    palette::detect(tree, source, byte_range, &mut out);
    dedupe(&mut out);
    out
}

fn dedupe(matches: &mut Vec<ColorMatch>) {
    matches.sort_by_key(|m| (m.start_byte, m.end_byte));
    matches.dedup_by_key(|m| (m.start_byte, m.end_byte));
}
