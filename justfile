# ─── DeepSeek Rust CLI - Justfile ────────────────────────────
# Cross-platform task runner. https://github.com/casey/just
# Install: cargo install just
# Usage:   just [command]

# Default: build in release mode
default: build-release

# ─── Build ────────────────────────────────────────────────────
build:
    cargo build --workspace

build-release:
    cargo build --release --workspace

build-all:
    cargo build --release --workspace --target x86_64-unknown-linux-gnu
    cargo build --release --workspace --target x86_64-pc-windows-msvc
    cargo build --release --workspace --target x86_64-apple-darwin

# ─── Test ─────────────────────────────────────────────────────
test:
    cargo test --workspace

test-verbose:
    cargo test --workspace -- --nocapture --test-threads=1

test-coverage:
    cargo tarpaulin --out Html --output-dir coverage

# ─── Lint & Format ────────────────────────────────────────────
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-fix:
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

lint: fmt-check clippy

# ─── Security Audit ───────────────────────────────────────────
audit:
    cargo audit

deny:
    cargo deny check

security: audit deny

# ─── Clean ──────────────────────────────────────────────────
clean:
    cargo clean
    rm -rf dist/ coverage/

# ─── Install ──────────────────────────────────────────────────
install: build-release
    cp target/release/deepseek-rust-cli /usr/local/bin/

install-local:
    cargo install --path . --force

# ─── Distribution ─────────────────────────────────────────────
dist-linux:
    cargo build --release --target x86_64-unknown-linux-gnu
    @mkdir -p dist
    cp target/x86_64-unknown-linux-gnu/release/deepseek-rust-cli dist/
    cd dist && tar czf deepseek-rust-cli-linux-x86_64.tar.gz deepseek-rust-cli
    sha256sum dist/deepseek-rust-cli-linux-x86_64.tar.gz > dist/deepseek-rust-cli-linux-x86_64.tar.gz.sha256

dist-windows:
    cargo build --release --target x86_64-pc-windows-msvc
    @mkdir -p dist
    cp target/x86_64-pc-windows-msvc/release/deepseek-rust-cli.exe dist/
    cd dist && 7z a deepseek-rust-cli-windows-x86_64.zip deepseek-rust-cli.exe
    sha256sum dist/deepseek-rust-cli-windows-x86_64.zip > dist/deepseek-rust-cli-windows-x86_64.zip.sha256

dist-macos:
    cargo build --release --target x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    @mkdir -p dist
    cp target/x86_64-apple-darwin/release/deepseek-rust-cli dist/deepseek-rust-cli-macos-x86_64
    cp target/aarch64-apple-darwin/release/deepseek-rust-cli dist/deepseek-rust-cli-macos-aarch64
    cd dist && tar czf deepseek-rust-cli-macos-x86_64.tar.gz deepseek-rust-cli-macos-x86_64
    cd dist && tar czf deepseek-rust-cli-macos-aarch64.tar.gz deepseek-rust-cli-macos-aarch64

# ─── Shell Completions ────────────────────────────────────────
completions:
    @mkdir -p completions
    cargo run --release -- --generate-completion bash > completions/deepseek-rust-cli.bash
    cargo run --release -- --generate-completion zsh > completions/deepseek-rust-cli.zsh
    cargo run --release -- --generate-completion fish > completions/deepseek-rust-cli.fish
    cargo run --release -- --generate-completion powershell > completions/deepseek-rust-cli.ps1

# ─── Dev Setup ────────────────────────────────────────────────
setup:
    cargo install sccache just cargo-audit cargo-deny cargo-tarpaulin
    rustup component add rustfmt clippy

# ─── Run ──────────────────────────────────────────────────────
run:
    cargo run --release

run-debug:
    cargo run -- --debug

run-auto:
    cargo run -- --auto-approve

# ─── Benchmark ────────────────────────────────────────────────
bench:
    cargo build --release
    hyperfine './target/release/deepseek-rust-cli --help'
