# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-04-19

### Changed

- Color-space conversions (HSL/HSV/HWB/Oklab/Oklch/linear sRGB) now
  delegate to the [`palette`](https://crates.io/crates/palette) crate.
  Public `Rgba` façade and detector API are unchanged. HWB retains an
  explicit `w + b >= 1` short-circuit so the achromatic gray gamut
  matches `bevy_color::Hwba`.
- Internal `palette` module renamed to `named_colors` to free the
  import path for the new crate dependency.

### Fixed

- `release.yml` now builds `x86_64-apple-darwin` by cross-compiling
  from Apple Silicon. The previous `macos-13` runner label is at
  end-of-life and queued indefinitely, so v0.1.0 shipped without an
  Intel Mac binary.

## [0.1.0] - 2026-04-18

### Added

- Initial release: `bevy-color-lsp` server + Zed extension wrapper.
- `textDocument/documentColor` + `textDocument/colorPresentation` for Bevy
  `Color` constructors, color-space structs, named constants, hex strings,
  and CSS/Tailwind/basic palette constants.
- Incremental tree-sitter parsing with cached queries/parser/colors.
- Incremental LSP sync with edit-window-scoped detector queries.
- Criterion benchmark suite.
