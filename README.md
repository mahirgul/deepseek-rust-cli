# DeepSeek Rust CLI Agent 🚀

An autonomous AI software engineer and CLI assistant powered by DeepSeek. This project was developed with the assistance of **Gemini CLI**.

> **⚠️ Note:** This project is currently in a **Testing & Development** phase. Use it at your own risk.

[![Release](https://img.shields.io/github/v/release/mahirgul/deepseek-rust-cli)](https://github.com/mahirgul/deepseek-rust-cli/releases)
[![License](https://img.shields.io/github/license/mahirgul/deepseek-rust-cli)](LICENSE)
[![CI](https://github.com/mahirgul/deepseek-rust-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/mahirgul/deepseek-rust-cli/actions/workflows/ci.yml)

## ✨ Features

- **🧠 Advanced Reasoning:** Real-time display of the model's thinking process (DeepSeek Reasoning).
- **🛠️ Robust Toolset:** 34 built-in tools — BASH commands, file I/O, search, Git, GitHub API, and web fetching.
- **🐙 GitHub Integration:** Create issues/PRs, search code/repos, manage workflows via GitHub API.
- **🔄 Undo Support:** Easily revert file changes made by the AI.
- **📁 Dynamic Context:** Automatic injection of project structure, git status, and local memory.
- **🎨 Visual Feedback:** Interactive progress bars, syntax-highlighted output, and spinner animations.
- **🤖 Autonomous Mode:** Optional auto-approval for seamless tool execution.
- **💬 Multi-session:** Resume, export, and manage multiple chat sessions.
- **🔐 Safety-first:** Dangerous commands and destructive operations require user approval.

## 🚀 Quick Install

### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
iwr https://raw.githubusercontent.com/mahirgul/deepseek-rust-cli/main/install.ps1 -useb | iex
```

### Cargo
```bash
cargo install --git https://github.com/mahirgul/deepseek-rust-cli
```

## 🛠️ Configuration

The tool requires a DeepSeek API Key. Create a `.env` file in your project or set it as an environment variable:

```bash
export DEEPSEEK_API_KEY="your_api_key_here"
```

Optional settings are stored in `.deep/config.json` and can be managed via the `/config` slash command.

### GitHub Integration (Optional)
```bash
export GITHUB_TOKEN="ghp_xxxxxxxx"
```

## 📖 Usage

Run the agent:
```bash
deepseek-rust-cli
```

### Slash Commands:
- `/help` - Show help menu
- `/model <name>` - Switch AI model
- `/sessions` - List all chat sessions
- `/resume <id>` - Switch to/resume a session
- `/undo` - Undo last file action
- `/tokens` - Show current token usage
- `/savemem <msg>` - Save critical info to .deep/memory.md
- `/export` - Export session to Markdown
- `/clear` - Clear terminal screen
- `/forget` - Wipe current history
- `/auto` - Toggle auto-approve mode
- `/info` - Show session info
- `/config <key> [value]` - View or modify config
- `/temperature <value>` - Set model temperature
- `/retry` - Retry last request

### Example Interactions:
```
> Create a PR merging feature-branch into main with title "Add login system"
> Search for TODO comments in the codebase
> Find all functions that use the deprecated API
> List open issues in rust-lang/rust
```

## 🏗️ Building from Source

Ensure you have [Rust](https://rustup.rs/) installed.

```bash
git clone https://github.com/mahirgul/deepseek-rust-cli.git
cd deepseek-rust-cli
cargo build --release
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
