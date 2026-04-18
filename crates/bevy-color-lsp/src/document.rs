use crate::detectors::{detect_all, ColorMatch};
use crate::parser::parse;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::lsp_types::{Position, Range, Url};

pub struct Document {
    pub text: String,
    cache: Option<Vec<(Range, ColorMatch)>>,
}

impl Document {
    pub fn new(text: String) -> Self {
        Self { text, cache: None }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cache = None;
    }

    pub fn colors(&mut self) -> Vec<(Range, ColorMatch)> {
        if let Some(cached) = &self.cache {
            return cached.clone();
        }
        let computed = self.compute_colors();
        self.cache = Some(computed.clone());
        computed
    }

    fn compute_colors(&self) -> Vec<(Range, ColorMatch)> {
        let Some(tree) = parse(&self.text) else {
            return Vec::new();
        };
        let mut matches = detect_all(&tree, &self.text);
        matches.sort_by_key(|m| (m.start_byte, m.end_byte));
        let ranges = byte_ranges_to_lsp(&self.text, &matches);
        matches
            .into_iter()
            .zip(ranges)
            .map(|(m, r)| (r, m))
            .collect()
    }
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
            let pos = Position {
                line,
                character: col_utf16,
            };
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
        let pos = Position {
            line,
            character: col_utf16,
        };
        if is_start {
            starts[mi] = pos;
        } else {
            ends[mi] = pos;
        }
    }

    starts
        .into_iter()
        .zip(ends)
        .map(|(start, end)| Range { start, end })
        .collect()
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
    Position {
        line,
        character: col_utf16,
    }
}

#[derive(Default)]
pub struct DocumentStore {
    docs: Mutex<HashMap<Url, Document>>,
}

impl DocumentStore {
    pub fn open(&self, uri: Url, text: String) {
        self.docs.lock().unwrap().insert(uri, Document::new(text));
    }

    pub fn update(&self, uri: &Url, text: String) {
        if let Some(doc) = self.docs.lock().unwrap().get_mut(uri) {
            doc.set_text(text);
        }
    }

    pub fn close(&self, uri: &Url) {
        self.docs.lock().unwrap().remove(uri);
    }

    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> {
        self.docs
            .lock()
            .unwrap()
            .get_mut(uri)
            .map(|d| d.colors())
            .unwrap_or_default()
    }
}

#[cfg(test)]
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
    fn store_open_then_colors() {
        let store = DocumentStore::default();
        let uri = Url::parse("file:///t.rs").unwrap();
        store.open(uri.clone(), "Color::WHITE".to_string());
        let cs = store.colors_for(&uri);
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn cache_invalidates_on_update() {
        let store = DocumentStore::default();
        let uri = Url::parse("file:///t.rs").unwrap();
        store.open(uri.clone(), "Color::WHITE".to_string());
        assert_eq!(store.colors_for(&uri).len(), 1);
        store.update(&uri, "Color::BLACK; Color::WHITE".to_string());
        assert_eq!(store.colors_for(&uri).len(), 2);
    }

    #[test]
    fn batch_ranges_match_per_call() {
        let text = "a\nbb\ncccc\nColor::WHITE";
        let m = ColorMatch {
            start_byte: 10,
            end_byte: 22,
            color: Rgba::WHITE,
        };
        let batched = byte_ranges_to_lsp(text, &[m]);
        let single = Range {
            start: byte_to_position(text, 10),
            end: byte_to_position(text, 22),
        };
        assert_eq!(batched[0], single);
    }
}
