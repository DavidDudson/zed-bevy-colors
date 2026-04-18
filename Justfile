default: check

# Fast local check — mirrors ci.yml
check: fmt-check clippy test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

clippy-strict:
    cargo clippy --workspace --all-targets -- \
        -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo

test:
    cargo test --workspace

bench:
    cargo bench -p bevy-color-lsp

wasm:
    cargo build -p zed-bevy-color-extension --release --target wasm32-wasip2

deny:
    cargo deny check

docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Line/region coverage. Skips data tables (palette.rs) and WASM shim.
# Needs `cargo-llvm-cov` + llvm-tools; the nix devshell provides both.
coverage:
    cargo llvm-cov --workspace --summary-only \
        --ignore-filename-regex 'palette\.rs|zed-extension'

# Emit lcov.info for upload (codecov).
coverage-lcov:
    cargo llvm-cov --workspace --lcov --output-path lcov.info \
        --ignore-filename-regex 'palette\.rs|zed-extension'

# Full pre-push gate — mirrors lint.yml + ci.yml
ci: fmt-check clippy test deny docs wasm

watch:
    cargo watch -x 'clippy --workspace --all-targets'
