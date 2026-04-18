# zed-bevy-color

[![CI](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml/badge.svg)](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/DavidDudson/zed-bevy-colors/graph/badge.svg)](https://codecov.io/gh/DavidDudson/zed-bevy-colors)
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

## Benchmarks

`cargo bench -p bevy-color-lsp --bench pipeline` (criterion). Source
sizes are synthetic Bevy functions with ~6 color literals each. Point
estimates are the middle of criterion's [low, mean, high] triplet;
measured locally with the release profile (`lto = "thin"`,
`codegen-units = 1`, `strip = true`).

### Parse + detect

| Size (fns) | Parse | Detect | Detect (suffixed literals) |
|---:|---:|---:|---:|
| 1 | 26 µs | 53 µs | — |
| 10 | 256 µs | 469 µs | — |
| 50 | 1.28 ms | 2.32 ms | 1.58 ms |
| 200 | 5.18 ms | 9.28 ms | 6.38 ms |

### Full pipeline

| Size | Cold (parse + detect + range) | Cached (cache hit) |
|---:|---:|---:|
| 1 fn | 83 µs | 10 ns |
| 10 | 756 µs | 47 ns |
| 50 | 3.69 ms | 105 ns |
| 200 | 14.8 ms | 825 ns |

### LSP document ops

| Op | small | medium | large |
|---|---:|---:|---:|
| `byte_ranges_to_lsp` (10 / 100 / 1000 ranges) | 2.75 µs | 27.1 µs | 272 µs |
| Store `replace` → `colors_for` (10 / 100 / 500 fns) | 740 µs | 7.32 ms | 37.2 ms |
| Incremental keystroke (10 / 50 / 200 / 500 fns) | 79 µs | 112 µs | 231 µs / 491 µs |
| Full resync keystroke (same sizes) | 747 µs | 3.72 ms | 15.0 ms / 37.7 ms |

### Position math (UTF-16 ↔ byte offsets)

`position_to_byte` uses the document's line-start index → O(1)
(~1.7–2.2 ns regardless of file size). `byte_to_position` scans from
file start, so cost scales with cursor position:

| File size (fns) | byte → pos (start / middle / end) |
|---:|---:|
| 10 | 430 ps / 897 ns / 1.78 µs |
| 100 | 428 ps / 8.71 µs / 17.4 µs |
| 1000 | 427 ps / 88.0 µs / 175 µs |

### Micro-benches

| Bench | Time |
|---|---:|
| `parse_hex` (6-item mixed corpus) | 174 ns |
| `palette::lookup_named` (6-item mixed corpus) | 123 ns |

### Edge cases + stress

| Bench | Time |
|---|---:|
| Empty source | 270 ns |
| 200 fns, zero color literals | 1.72 ms |
| Palette-heavy 200 fns (`palettes::*::NAME`) | 6.45 ms |
| UTF-8 multibyte 200 fns (emoji + CJK) | 3.79 ms |
| Large source (2000 fns, ~512 KB) | 156 ms |
| LSP keystroke session (200 fns, 50 edits) | 27.5 ms |
| Concurrent store (4 threads × 100 mixed ops) | 597 ms |
| Concurrent store (8 threads × 100 mixed ops) | 1.53 s |

Concurrent benches are stochastic; treat as order-of-magnitude.

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
