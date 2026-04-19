# zed-bevy-color

[![CI](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml/badge.svg)](https://github.com/DavidDudson/zed-bevy-colors/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/DavidDudson/zed-bevy-colors/graph/badge.svg)](https://codecov.io/gh/DavidDudson/zed-bevy-colors)
[![Release](https://img.shields.io/github/v/release/DavidDudson/zed-bevy-colors?display_name=tag&sort=semver)](https://github.com/DavidDudson/zed-bevy-colors/releases)
[![Zed extension](https://img.shields.io/badge/zed-extension-084CCF?logo=zedindustries&logoColor=white)](https://zed.dev/extensions?query=bevy-color)
[![License](https://img.shields.io/github/license/DavidDudson/zed-bevy-colors)](LICENSE)

Inline color swatches for [Bevy](https://bevy.org) `Color` literals in
[Zed](https://zed.dev) (and any LSP-aware editor).

![Inlay color swatches next to Bevy `Color` literals in Zed](docs/screenshots/inlay-swatches.png)

Implemented as a small Rust LSP server speaking
`textDocument/documentColor`, plus a Zed extension that downloads the
prebuilt server binary on first use.

> **Using VS Code?** See
> [FrTerstappen/bevy-color](https://github.com/FrTerstappen/bevy-color)
> — same idea, VS Code extension.

## For users

### Install the LSP server

Three ways to get the `bevy-color-lsp` binary on your system. **Zed
users can skip this** — the extension auto-downloads the right
prebuilt for your platform on first activation. All other editors
need the binary on `$PATH`.

**Prebuilt binary (recommended)** — grab the asset for your platform
from the [latest release](https://github.com/DavidDudson/zed-bevy-colors/releases/latest):

| OS / arch | Asset |
|---|---|
| Linux x86_64 | `bevy-color-lsp-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `bevy-color-lsp-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `bevy-color-lsp-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `bevy-color-lsp-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `bevy-color-lsp-x86_64-pc-windows-msvc.zip` |

Extract and drop `bevy-color-lsp` (or `bevy-color-lsp.exe`) into a
directory on your `$PATH` — e.g. `~/.local/bin`, `/usr/local/bin`,
`%LOCALAPPDATA%\Microsoft\WinGet\Links`.

```sh
# Linux/macOS one-liner (substitute the asset for your platform)
curl -L https://github.com/DavidDudson/zed-bevy-colors/releases/latest/download/bevy-color-lsp-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C ~/.local/bin
```

**From source (Rust toolchain required)** — clone the repo:

```sh
git clone https://github.com/DavidDudson/zed-bevy-colors
cargo install --path zed-bevy-colors/crates/bevy-color-lsp
```

This installs `bevy-color-lsp` into `~/.cargo/bin/` (which `rustup`
adds to your `$PATH`).

**Verify** the binary is on `$PATH`:

```sh
command -v bevy-color-lsp   # POSIX (Linux/macOS) — prints the resolved path
where bevy-color-lsp        # Windows
```

(The binary itself takes no flags — it speaks LSP over stdio. Don't
run it standalone; your editor invokes it.)

### Editor setup

#### Zed

Once published to the Zed extensions registry:

> Extensions (`zed: extensions`) → search **Bevy Color** → Install

The extension downloads the prebuilt server binary on first use — no
separate `cargo install` step needed. Requires a recent Zed release
(extension uses `zed_extension_api 0.7`; older Zed versions will
refuse to load it). See the [Zed extension compatibility table](https://zed.dev/docs/extensions/developing-extensions)
for the exact floor.

**Pre-registry / dev install:** clone this repo, then in Zed run
`zed: install dev extension` and select `crates/zed-extension/`. The
dev extension expects `bevy-color-lsp` on `$PATH` (it does **not**
download a binary in dev mode), so install the server first
(`cargo install --path crates/bevy-color-lsp`).

**Swatch rendering** is controlled by Zed's `lsp_document_colors`
setting. Edit your `settings.json` (`zed: open settings`):

```jsonc
{
  "lsp_document_colors": "inlay"   // default — small square next to value
  // "border"     — colored border around the literal
  // "background" — colored background highlight
  // "none"       — disable
}
```

#### Helix

Requires Helix with [PR #12308](https://github.com/helix-editor/helix/pull/12308)
included (LSP `documentColor` support). This shipped after Helix
24.07 — use a current release or build from `master`.

Install the server (see above), then add to your
`~/.config/helix/languages.toml`:

```toml
[language-server.bevy-color-lsp]
command = "bevy-color-lsp"

[[language]]
name = "rust"
language-servers = ["rust-analyzer", "bevy-color-lsp"]
```

Open a Rust file containing a Bevy `Color` literal — Helix renders
the inline swatches automatically; no further config required.

If you want to verify the server is being invoked, set
`BEVY_COLOR_LSP_LOG=debug` (see [Logging](#logging)) before launching
Helix and check `~/.cache/helix/helix.log`.

#### Neovim (built-in LSP)

Requires a Neovim version + plugin combination that renders
`textDocument/documentColor` results. Plugins: [`mason-lspconfig`](https://github.com/williamboman/mason-lspconfig.nvim)
+ [`document-color.nvim`](https://github.com/mrshmllow/document-color.nvim)
or any equivalent that consumes `textDocument/documentColor`.

Register the server with `nvim-lspconfig` (manual config since this
LSP is not in the lspconfig registry):

```lua
local configs = require("lspconfig.configs")
local lspconfig = require("lspconfig")

if not configs.bevy_color_lsp then
  configs.bevy_color_lsp = {
    default_config = {
      cmd = { "bevy-color-lsp" },
      filetypes = { "rust" },
      root_dir = lspconfig.util.root_pattern("Cargo.toml"),
      settings = {},
    },
  }
end

lspconfig.bevy_color_lsp.setup({})
```

Then attach a documentColor renderer (e.g.
`require("document-color").setup{}`).

#### Other LSP clients

Any editor that speaks LSP and renders `textDocument/documentColor`
will work. Point the client at the `bevy-color-lsp` binary; it speaks
LSP over stdio with no command-line arguments.

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

Criterion benches live under `crates/bevy-color-lsp/benches/`. Run
locally with `cargo bench -p bevy-color-lsp`. Headline numbers on a
200-function synthetic source: ~15 ms cold pipeline, sub-µs cached
hit, ~230 µs incremental keystroke.

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
