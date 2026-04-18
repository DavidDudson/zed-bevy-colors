# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-18

### Added

- Initial release: `bevy-color-lsp` server + Zed extension wrapper.
- `textDocument/documentColor` + `textDocument/colorPresentation` for Bevy
  `Color` constructors, color-space structs, named constants, hex strings,
  and CSS/Tailwind/basic palette constants.
- Incremental tree-sitter parsing with cached queries/parser/colors.
- Incremental LSP sync with edit-window-scoped detector queries.
- Criterion benchmark suite.
