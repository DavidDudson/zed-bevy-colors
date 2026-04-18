# zed-bevy-color

[![CI](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml/badge.svg)](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/DavidDudson/zed-bevy-colors?display_name=tag&sort=semver)](https://github.com/DavidDudson/zed-bevy-colors/releases)
[![Zed extension](https://img.shields.io/badge/zed-extension-084CCF?logo=zedindustries&logoColor=white)](https://zed.dev/extensions?query=bevy-color)
[![License](https://img.shields.io/github/license/DavidDudson/zed-bevy-colors)](LICENSE)

Inline color swatches for [Bevy](https://bevy.org) `Color` literals in
[Zed](https://zed.dev) (and any LSP-aware editor).

Implemented as a small Rust LSP server speaking
`textDocument/documentColor`, plus a Zed extension that downloads the
prebuilt server binary on first use.

> **Using VS Code?** See
> [FrTerstappen/bevy-color](https://github.com/FrTerstappen/bevy-color)
> — same idea, VS Code extension.

## For users

### Install (Zed)

Once published to the Zed extensions registry:

> Extensions → search "Bevy Color" → Install

Requires a recent Zed release (extension uses `zed_extension_api 0.7`;
older Zed versions will refuse to load it). See the [Zed extension
compatibility table](https://zed.dev/docs/extensions/developing-extensions)
for the exact floor.

### Configure swatch rendering

Zed setting `lsp_document_colors` controls how swatches appear:

```jsonc
{
  "lsp_document_colors": "inlay"   // default — small square next to value
  // "border"     — colored border around the literal
  // "background" — colored background highlight
  // "none"       — disable
}
```

### Other editors

The LSP server is editor-agnostic. Use it from VS Code (any LSP plugin),
Helix, Neovim, etc.

```sh
cargo install --path crates/bevy-color-lsp
bevy-color-lsp   # speaks LSP over stdio
```

### Logging

Set the `BEVY_COLOR_LSP_LOG` environment variable to control server log
output (written to stderr). Uses [`tracing-subscriber`'s `EnvFilter`
syntax](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html):

```sh
BEVY_COLOR_LSP_LOG=debug bevy-color-lsp
BEVY_COLOR_LSP_LOG=bevy_color_lsp=trace bevy-color-lsp
```

Defaults to `info` when unset.

### What it detects

| Pattern | Example |
|---|---|
| Bevy `Color` constructors | `Color::srgb(1.0, 0.5, 0.0)`, `Color::srgba`, `Color::hsl`, `Color::hsv`, `Color::hwb`, `Color::oklab`, `Color::oklch`, `Color::linear_rgb`, `Color::srgb_u8` |
| Color-space struct constructors | `Srgba::new`, `LinearRgba::new`, `Hsla::new`, `Hsva::new`, `Hwba::new`, `Oklaba::new`, `Oklcha::new` |
| Bevy named constants | `Color::WHITE`, `Color::BLACK`, `Color::NONE` |
| Hex strings | `Srgba::hex("FF8800")`, `Color::hex("#abc")` |
| CSS / Tailwind / basic palettes | `palettes::css::TOMATO`, `palettes::tailwind::BLUE_500`, `palettes::basic::RED` |

Detection is syntactic (tree-sitter-rust) — no type resolution, but no
false positives on calls to non-`Color` types either.

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

## For contributors

<a href="https://github.com/DavidDudson/zed-bevy-colors/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=DavidDudson/zed-bevy-colors" alt="Contributors" />
</a>

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full dev loop, commit
conventions, release flow, and how to publish new extension versions to
the Zed registry. Quick start with nix + direnv:

```sh
git clone https://github.com/DavidDudson/zed-bevy-colors
cd zed-bevy-color
direnv allow            # or: nix develop
cargo build -p bevy-color-lsp --release
```

Without nix: install Rust stable with `rustfmt`, `clippy`, and the
`wasm32-wasip2` target (the `rust-toolchain.toml` does this
automatically under `rustup`).

Browse the API documentation locally:

```sh
cargo doc --workspace --no-deps --open
```

## License

[MIT](LICENSE)
