# DeepSeek Rust CLI Agent 🚀

An autonomous AI software engineer and CLI assistant powered by DeepSeek. This project was developed with the assistance of **Gemini CLI**.

> **⚠️ Note:** This project is currently in a **Testing & Development** phase. Use it at your own risk.

[![Release](https://img.shields.io/github/v/release/mahirgul/deepseek-rust-cli)](https://github.com/mahirgul/deepseek-rust-cli/releases)
[![License](https://img.shields.io/github/license/mahirgul/deepseek-rust-cli)](LICENSE)

## ✨ Features

- **🧠 Advanced Reasoning:** Real-time display of the model's thinking process.
- **🛠️ Tool Integration:** Execute BASH commands, read files, apply patches, and fetch web content.
- **🎨 Syntax Highlighting:** Beautifully rendered code blocks in your terminal.
- **🔄 Token-based Memory:** Smart context management with automatic summarization.
- **🛡️ Security First:** Pattern-based safety checks for dangerous commands.
- **📦 Cross-Platform:** Support for Linux (Debian/RedHat), macOS, and Windows.

## 🚀 Quick Install

### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
iwr https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.ps1 -useb | iex
```

## 🛠️ Configuration

The tool requires a DeepSeek API Key. Create a `.env` file in your project or set it as an environment variable:

```bash
export DEEPSEEK_API_KEY="your_api_key_here"
```

Optional settings can be managed in `config.json`.

## 📖 Usage

Run the agent:
```bash
deepseek-rust-cli
```

Available Slash Commands:
- `/help` - Show help menu
- `/model <name>` - Switch AI model
- `/forget` - Clear conversation history
- `/clear` - Clear terminal screen

## 🏗️ Building from Source

Ensure you have [Rust](https://rustup.rs/) installed.

```bash
git clone https://github.com/mahirgul/deepseek-rust-cli.git
cd deepseek-rust-cli
cargo build --release
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
