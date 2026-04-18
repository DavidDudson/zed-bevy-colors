//! Crate error type.

use thiserror::Error;

/// Errors surfaced by `bevy-color-lsp` internals.
#[derive(Debug, Error)]
pub enum Error {
    /// Input was not a valid hex color literal.
    #[error("invalid hex color literal: {0:?}")]
    InvalidHex(String),
    /// LSP position references a location outside the current document.
    #[error("position out of document bounds: line={line} character={character}")]
    PositionOutOfBounds {
        /// Line index (0-based, as in LSP).
        line: u32,
        /// Character index in UTF-16 code units.
        character: u32,
    },
    /// Byte-offset arithmetic overflowed while applying an incremental edit.
    #[error("byte offset arithmetic overflow")]
    OffsetOverflow,
    /// Tree-sitter failed to load its grammar.
    #[error("tree-sitter failed to load grammar")]
    GrammarLoad,
    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Crate-local `Result` alias.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn display_each_variant() {
        assert!(Error::InvalidHex("bad".into()).to_string().contains("invalid hex"));
        assert!(Error::PositionOutOfBounds { line: 5, character: 3 }
            .to_string()
            .contains("out of document bounds"));
        assert!(Error::OffsetOverflow.to_string().contains("overflow"));
        assert!(Error::GrammarLoad.to_string().contains("grammar"));
    }
}
