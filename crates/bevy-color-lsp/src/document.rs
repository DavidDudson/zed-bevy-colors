use crate::detectors::{detect_all, ColorMatch};
use crate::parser::parse;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::lsp_types::{Position, Range, Url};

pub struct Document {
    pub text: String,
}

impl Document {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub fn colors(&self) -> Vec<(Range, ColorMatch)> {
        let Some(tree) = parse(&self.text) else {
            return Vec::new();
        };
        detect_all(&tree, &self.text)
            .into_iter()
            .map(|m| (byte_range_to_lsp(&self.text, m.start_byte, m.end_byte), m))
            .collect()
    }
}

pub fn byte_range_to_lsp(text: &str, start: usize, end: usize) -> Range {
    Range {
        start: byte_to_position(text, start),
        end: byte_to_position(text, end),
    }
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
            doc.text = text;
        }
    }

    pub fn close(&self, uri: &Url) {
        self.docs.lock().unwrap().remove(uri);
    }

    pub fn colors_for(&self, uri: &Url) -> Vec<(Range, ColorMatch)> {
        self.docs
            .lock()
            .unwrap()
            .get(uri)
            .map(|d| d.colors())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
