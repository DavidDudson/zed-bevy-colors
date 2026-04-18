//! Color detection across detector sub-strategies.
//!
//! All byte offsets produced here are valid UTF-8 byte positions in the
//! source string passed to [`detect_all`] or [`detect_in_range`].

use crate::color::Rgba;
use std::ops::Range;
use tree_sitter::Tree;

pub mod bevy_const;
pub mod bevy_ctor;
pub mod bevy_hex;
pub mod palette;

/// A color detected at a byte range within the source text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatch {
    /// Inclusive start byte offset in the source string.
    pub start_byte: usize,
    /// Exclusive end byte offset in the source string.
    pub end_byte: usize,
    /// The resolved color value.
    pub color: Rgba,
}

/// Detect all color matches in the entire `source` text.
#[must_use]
pub fn detect_all(tree: &Tree, source: &str) -> Vec<ColorMatch> {
    detect_in_range(tree, source, None)
}

/// Detect color matches within an optional byte range of `source`.
///
/// When `byte_range` is `Some`, only nodes overlapping that range are visited.
/// The returned matches still carry absolute byte offsets into `source`.
#[must_use]
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
