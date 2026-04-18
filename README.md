# zed-bevy-color

Inline color swatches for [Bevy](https://bevy.org) `Color` literals in
[Zed](https://zed.dev) (and any LSP-aware editor).

Implemented as a small Rust LSP server speaking
`textDocument/documentColor`, plus a Zed extension that downloads the
prebuilt server binary on first use.

## What it detects

| Pattern | Example |
|---|---|
| Bevy `Color` constructors | `Color::srgb(1.0, 0.5, 0.0)`, `Color::srgba`, `Color::hsl`, `Color::hsv`, `Color::hwb`, `Color::oklab`, `Color::oklch`, `Color::linear_rgb`, `Color::srgb_u8` |
| Color-space struct constructors | `Srgba::new`, `LinearRgba::new`, `Hsla::new`, `Hsva::new`, `Hwba::new`, `Oklaba::new`, `Oklcha::new` |
| Bevy named constants | `Color::WHITE`, `Color::BLACK`, `Color::NONE` |
| Hex strings | `Srgba::hex("FF8800")`, `Color::hex("#abc")` |
| CSS / Tailwind / basic palettes | `palettes::css::TOMATO`, `palettes::tailwind::BLUE_500`, `palettes::basic::RED` |

Detection is syntactic (tree-sitter-rust) — no type resolution, but no
false positives on calls to non-`Color` types either.

## Install (Zed)

Once published to the Zed extensions registry:

> Extensions → search "Bevy Color" → Install

For local development:

```sh
git clone https://github.com/ddudson/zed-bevy-color
cd zed-bevy-color
cargo build -p bevy-color-lsp --release
```

Then in Zed: `zed: install dev extension` → select
`crates/zed-extension/`. The extension expects `bevy-color-lsp` either:

1. on `$PATH` (set by `cargo install --path crates/bevy-color-lsp`), or
2. downloaded from a GitHub release on first launch.

## Configure swatch rendering

Zed setting `lsp_document_colors` controls how swatches appear:

```jsonc
{
  "lsp_document_colors": "inlay"   // default — small square next to value
  // "border"     — colored border around the literal
  // "background" — colored background highlight
  // "none"       — disable
}
```

## Build the LSP server only

The server is editor-agnostic. Use it from VS Code (any LSP plugin),
Helix, Neovim, etc.

```sh
cargo install --path crates/bevy-color-lsp
bevy-color-lsp   # speaks LSP over stdio
```

## Build the Zed extension (Wasm)

Requires `wasm32-wasip2` target:

```sh
rustup target add wasm32-wasip2
cargo build -p zed-bevy-color-extension --release --target wasm32-wasip2
```

## Architecture

```
crates/
├── bevy-color-lsp/      # tower-lsp server, tree-sitter-rust parser
└── zed-extension/       # WASM wrapper, downloads binary from GH release
```

LSP capabilities advertised:

- `textDocument/documentColor` — returns `ColorInformation[]` for every
  Bevy color literal in the document.
- `textDocument/colorPresentation` — returns a single
  `Color::srgb_u8(...)` or `Color::srgba(...)` snippet (Zed's picker UI
  for this is limited as of writing — see
  [zed-industries/zed#52208](https://github.com/zed-industries/zed/issues/52208)).

## License

Dual-licensed under MIT or Apache-2.0 at your option.
