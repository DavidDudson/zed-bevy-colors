use crate::detectors::{detect_all, detect_in_range, ColorMatch};
use crate::parser::parse_incremental;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::lsp_types::{Position, Range, Url};
use tree_sitter::{InputEdit, Point, Tree};

/// Bytes of context around an edit to rescan, ensuring partial color
/// expressions split by the edit boundary are still captured.
const RESCAN_CONTEXT: usize = 256;

#[derive(Debug)]
pub struct Document {
    pub text: String,
    line_starts: Vec<usize>,
    tree: Option<Tree>,
    cache: Option<Vec<(Range, ColorMatch)>>,
}

impl Document {
    pub fn new(text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Self { text, line_starts, tree: None, cache: None }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.line_starts = compute_line_starts(&self.text);
        self.tree = None;
        self.cache = None;
    }

    pub fn apply_change(&mut self, range: Option<Range>, new_text: &str) {
        let Some(range) = range else {
            self.set_text(new_text.to_string());
            return;
        };
        let start_byte = position_to_byte(&self.text, &self.line_starts, range.start);
        let old_end_byte = position_to_byte(&self.text, &self.line_starts, range.end);
        let new_end_byte = start_byte + new_text.len();
        let delta = new_end_byte as isize - old_end_byte as isize;

        let start_position = lsp_to_point(range.start);
        let old_end_position = lsp_to_point(range.end);

        self.text.replace_range(start_byte..old_end_byte, new_text);
        self.line_starts = compute_line_starts(&self.text);
        let new_end_position = byte_to_point(&self.text, new_end_byte);

        if let Some(tree) = self.tree.as_mut() {
            tree.edit(&InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            });
        }

        let old_cache = self.cache.take();
        self.tree = parse_incremental(&self.text, self.tree.as_ref());
        if let (Some(tree), Some(old)) = (self.tree.as_ref(), old_cache) {
            self.cache = Some(incremental_color_update(
                &self.text,
                tree,
                old,
                start_byte,
                old_end_byte,
                new_end_byte,
                delta,
            ));
        }
    }

    pub fn colors(&mut self) -> Vec<(Range, ColorMatch)> {
        if let Some(cached) = &self.cache {
            return cached.clone();
        }
        let computed = self.compute_colors();
        self.cache = Some(computed.clone());
        computed
    }

    fn compute_colors(&mut self) -> Vec<(Range, ColorMatch)> {
        if self.tree.is_none() {
            self.tree = parse_incremental(&self.text, None);
        }
        let Some(tree) = self.tree.as_ref() else {
            return Vec::new();
        };
        let mut matches = detect_all(tree, &self.text);
        matches.sort_by_key(|m| (m.start_byte, m.end_byte));
        let ranges = byte_ranges_to_lsp(&self.text, &matches);
        matches.into_iter().zip(ranges).map(|(m, r)| (r, m)).collect()
    }
}

fn incremental_color_update(
    text: &str,
    tree: &Tree,
    old: Vec<(Range, ColorMatch)>,
    edit_start: usize,
    edit_old_end: usize,
    edit_new_end: usize,
    delta: isize,
) -> Vec<(Range, ColorMatch)> {
    let rescan_start = edit_start.saturating_sub(RESCAN_CONTEXT);
    let rescan_end = (edit_new_end + RESCAN_CONTEXT).min(text.len());

    let mut kept: Vec<ColorMatch> = Vec::with_capacity(old.len());
    for (_, m) in old {
        if m.end_byte <= edit_start {
            kept.push(m);
        } else if m.start_byte >= edit_old_end {
            let new_start = (m.start_byte as isize + delta) as usize;
            let new_end = (m.end_byte as isize + delta) as usize;
            kept.push(ColorMatch { start_byte: new_start, end_byte: new_end, color: m.color });
        }
    }
    kept.retain(|m| m.end_byte <= rescan_start || m.start_byte >= rescan_end);

    let mut new_matches = detect_in_range(tree, text, Some(rescan_start..rescan_end));
    kept.append(&mut new_matches);
    kept.sort_by_key(|m| (m.start_byte, m.end_byte));
    kept.dedup_by_key(|m| (m.start_byte, m.end_byte));

    let ranges = byte_ranges_to_lsp(text, &kept);
    kept.into_iter().zip(ranges).map(|(m, r)| (r, m)).collect()
}

fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn lsp_to_point(p: Position) -> Point {
    Point { row: p.line as usize, column: p.character as usize }
}

fn byte_to_point(text: &str, byte: usize) -> Point {
    let mut row = 0usize;
    let mut last_line_start = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if i >= byte {
            break;
        }
        if b == b'\n' {
            row += 1;
            last_line_start = i + 1;
        }
    }
    let column = text
        .get(last_line_start..byte.min(text.len()))
        .map(|s| s.encode_utf16().count())
        .unwrap_or(0);
    Point { row, column }
}

pub fn position_to_byte(text: &str, line_starts: &[usize], pos: Position) -> usize {
    let line = pos.line as usize;
    if line >= line_starts.len() {
        return text.len();
    }
    let line_start = line_starts[line];
    let line_end =
        line_starts.get(line + 1).copied().map(|n| n.saturating_sub(1)).unwrap_or(text.len());
    let line_slice = &text[line_start..line_end];

    let mut col_utf16 = 0u32;
    let mut byte_offset = 0usize;
    for ch in line_slice.chars() {
        if col_utf16 >= pos.character {
            break;
        }
        col_utf16 += ch.len_utf16() as u32;
        byte_offset += ch.len_utf8();
    }
    line_start + byte_offset
}

pub fn byte_ranges_to_lsp(text: &str, matches: &[ColorMatch]) -> Vec<Range> {
    let mut points: Vec<(usize, bool, usize)> = Vec::with_capacity(matches.len() * 2);
    for (i, m) in matches.iter().enumerate() {
        points.push((m.start_byte, true, i));
        points.push((m.end_byte, false, i));
    }
    points.sort_by_key(|p| p.0);

    let mut starts = vec![Position::default(); matches.len()];
    let mut ends = vec![Position::default(); matches.len()];

    let mut line = 0u32;
    let mut col_utf16 = 0u32;
    let mut idx = 0usize;
    let mut iter = points.into_iter().peekable();

    for ch in text.chars() {
        while let Some(&(b, is_start, mi)) = iter.peek() {
            if b > idx {
                break;
            }
            let pos = Position { line, character: col_utf16 };
            if is_start {
                starts[mi] = pos;
            } else {
                ends[mi] = pos;
            }
            iter.next();
        }
        let len = ch.len_utf8();
        if ch == '\n' {
            line += 1;
            col_utf16 = 0;
        } else {
            col_utf16 += ch.len_utf16() as u32;
        }
        idx += len;
    }
    for (b, is_start, mi) in iter {
        debug_assert!(b >= idx);
        let pos = Position { line, character: col_utf16 };
        if is_start {
            starts[mi] = pos;
        } else {
            ends[mi] = pos;
        }
    }

    starts.into_iter().zip(ends).map(|(start, end)| Range { start, end }).collect()
}

pub fn byte_to_position(text: &str, byte: usize) -> Position {
    let mut line = 0u32;
    let mut col_utf16 = 0u32;
    let mut idx = 0;
    for ch in text.chars() {
        if idx >= byte {
            break;
        }
        let len = ch.len_utf8();
        if ch == '\n' {
            line += 1;
            col_utf16 = 0;
        } else {
            col_utf16 += ch.len_utf16() as u32;
        }
        idx += len;
    }
    Position { line, character: col_utf16 }
}

#[derive(Debug, Default)]
pub struct DocumentStore {
    docs: Mutex<HashMap<Url, Document>>,
}

impl DocumentStore {
    // TODO(Stream 3): remove once std::Mutex is swapped for parking_lot::Mutex.
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub fn open(&self, uri: Url, text: String) {
        self.docs.lock().unwrap().insert(uri, Document::new(text));
    }

    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub fn replace(&self, uri: &Url, text: String) {
        if let Some(doc) = self.docs.lock().unwrap().get_mut(uri) {
            doc.set_text(text);
        }
    }

    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub fn apply_change(&self, uri: &Url, range: Option<Range>, text: &str) {
        if let Some(doc) = self.docs.lock().unwrap().get_mut(uri) {
            doc.apply_change(range, text);
        }
    }

    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub fn close(&self, uri: &Url) {
        self.docs.lock().unwrap().remove(uri);
    }

    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> {
        self.docs.lock().unwrap().get_mut(uri).map(|d| d.colors()).unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn position_first_line() {
        let p = byte_to_position("hello", 3);
        assert_eq!(p.line, 0);
        assert_eq!(p.character, 3);
    }

    #[test]
    fn position_second_line() {
        let p = byte_to_position("ab\ncd", 4);
        assert_eq!(p.line, 1);
        assert_eq!(p.character, 1);
    }

    #[test]
    fn position_utf16_emoji() {
        let p = byte_to_position("a\u{1F600}b", 5);
        assert_eq!(p.character, 3);
    }

    #[test]
    fn position_to_byte_round_trip() {
        let text = "ab\nc\u{1F600}de\nfg";
        let starts = compute_line_starts(text);
        let p = Position { line: 1, character: 3 };
        let byte = position_to_byte(text, &starts, p);
        assert_eq!(&text[byte..byte + 1], "d");
    }

    #[test]
    fn store_open_then_colors() {
        let store = DocumentStore::default();
        let uri = Url::parse("file:///t.rs").unwrap();
        store.open(uri.clone(), "Color::WHITE".to_string());
        let cs = store.colors_for(&uri);
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn cache_invalidates_on_replace() {
        let store = DocumentStore::default();
        let uri = Url::parse("file:///t.rs").unwrap();
        store.open(uri.clone(), "Color::WHITE".to_string());
        assert_eq!(store.colors_for(&uri).len(), 1);
        store.replace(&uri, "Color::BLACK; Color::WHITE".to_string());
        assert_eq!(store.colors_for(&uri).len(), 2);
    }

    #[test]
    fn incremental_edit_updates_colors() {
        let mut doc = Document::new("let a = Color::WHITE;".to_string());
        assert_eq!(doc.colors().len(), 1);
        let range = Range {
            start: Position { line: 0, character: 21 },
            end: Position { line: 0, character: 21 },
        };
        doc.apply_change(Some(range), " let b = Color::BLACK;");
        let cs = doc.colors();
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn incremental_replace_inside_literal() {
        let mut doc = Document::new("let a = Color::srgb(1.0, 0.0, 0.0);".to_string());
        let cs = doc.colors();
        assert!((cs[0].1.color.r - 1.0).abs() < 0.01);
        let range = Range {
            start: Position { line: 0, character: 20 },
            end: Position { line: 0, character: 23 },
        };
        doc.apply_change(Some(range), "0.0");
        let cs = doc.colors();
        assert!(cs[0].1.color.r.abs() < 0.01);
    }

    #[test]
    fn batch_ranges_match_per_call() {
        let text = "a\nbb\ncccc\nColor::WHITE";
        let m = ColorMatch { start_byte: 10, end_byte: 22, color: Rgba::WHITE };
        let batched = byte_ranges_to_lsp(text, &[m]);
        let single = Range { start: byte_to_position(text, 10), end: byte_to_position(text, 22) };
        assert_eq!(batched[0], single);
    }
}
