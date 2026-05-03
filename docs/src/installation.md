# Installation

DeepSeek Rust CLI can be installed using several methods.

## 📦 Quick Install

### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
iwr https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.ps1 -useb | iex
```

## 🛠️ From Source (Cargo)

If you have Rust and Cargo installed, you can build it from source:

```bash
cargo install --git https://github.com/mahirgul/deepseek-rust-cli
```

Or clone the repo and build:

```bash
git clone https://github.com/mahirgul/deepseek-rust-cli.git
cd deepseek-rust-cli
cargo build --release
```

The binary will be located at `./target/release/deepseek-rust-cli`.
