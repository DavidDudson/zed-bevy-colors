use crate::color::Rgba;
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
    let mut out = Vec::new();
    bevy_ctor::detect(tree, source, &mut out);
    bevy_const::detect(tree, source, &mut out);
    bevy_hex::detect(tree, source, &mut out);
    palette::detect(tree, source, &mut out);
    dedupe(&mut out);
    out
}

fn dedupe(matches: &mut Vec<ColorMatch>) {
    matches.sort_by_key(|m| (m.start_byte, m.end_byte));
    matches.dedup_by_key(|m| (m.start_byte, m.end_byte));
}
