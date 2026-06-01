# DeepSeek Rust CLI Agent 🚀

<img width="486" height="297" alt="image" src="https://github.com/user-attachments/assets/1b6383af-d7c1-49aa-b6f1-abeb05772aea" />


An autonomous AI software engineer and CLI assistant powered by DeepSeek. This project was developed with the assistance of **Gemini CLI**.

> **⚠️ Note:** This project is currently in a **Testing & Development** phase. Use it at your own risk.

[![Release](https://img.shields.io/github/v/release/mahirgul/deepseek-rust-cli)](https://github.com/mahirgul/deepseek-rust-cli/releases)
[![License](https://img.shields.io/github/license/mahirgul/deepseek-rust-cli)](LICENSE)
[![CI](https://github.com/mahirgul/deepseek-rust-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/mahirgul/deepseek-rust-cli/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![GitHub Stars](https://img.shields.io/github/stars/mahirgul/deepseek-rust-cli)](https://github.com/mahirgul/deepseek-rust-cli/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/mahirgul/deepseek-rust-cli)](https://github.com/mahirgul/deepseek-rust-cli/issues)

## ✨ Features

- **🧠 Advanced Reasoning:** Real-time display of the model's thinking process (DeepSeek Reasoning) in a dimmed style with a `🧠 Thinking Process:` header to keep it visually separate from the main response.
- **💬 Structured Responses:** The final assistant output is clearly prefixed with a `💬 Response:` header.
- **🛠️ Extensible Toolset:** 65+ tools (including line-based file edits, unified diff patching, background process lifecycle and logs tracking, network port checking, regex replacements, JSON configs, code symbol and AST-like detail parsing, DuckDuckGo web search, headless screenshots, and more) managed by a trait-based registry system.
- **🐚 Stateful Shell:** Persistent working directory (CWD) support — `cd` commands update the agent's environment state.
- **🎨 Rich TUI Engine:** 
  - **4-Line Dynamic Footer:** Real-time status, folder info, token usage, and command queue.
  - **Horizontal Queue:** Visualize pending and executing tasks at a glance.
  - **Syntax Highlighting:** Instant coloring for tool results (JSON, Rust, Python, etc.) and streaming code blocks.
  - **Visual Separators:** Dimmed horizontal separator lines are printed between all operations to keep the log history clean.
- **⌨️ Advanced Input:** Full cursor control (Home, End, Left, Right), Bracketed Paste support, and persistent history.
- **🛑 Robust Control:** Instant abort via **Esc** which clears all running and pending queues and cleans up streamed context safely to prevent UI deadlocks. Built-in **TerminalGuard** with Ctrl+C listener ensures graceful terminal restoration on unexpected exits.
- **💡 Command Suggestions:** Mistyped or unrecognized slash commands automatically suggest the closest match using Levenshtein distance.
- **🔄 Optimized CI/CD:** Parallelized matrix builds with **mold** linker and **sccache** for lightning-fast automation.
- **🔐 Security:** Mandatory tool approvals and strict path validation. Path traversal warnings bypass global auto-approvals, disable the "Always Approve" option, and enforce individual user permission checks for every request. Uses lexical path normalization and RAII safety guards to prevent privilege leaks.

## 📖 Documentation

Comprehensive documentation is available in the `docs/` directory and can be viewed as an [mdBook](https://rust-lang.github.io/mdBook/):

- **Architecture:** Core component breakdown and execution flow.
- **Tools:** Detailed list of available tools and their capabilities.
- **Contributing:** Guidelines for setting up the development environment.

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

### 🪙 Token Optimization Settings
To optimize and reduce token consumption, the following custom settings are supported:
- `max_context_chars` (default `100000`): The maximum character length of active session history kept. Older messages are automatically pruned when this limit is exceeded.
- `max_tool_output_chars` (default `15000`): The maximum character length of a single tool execution's output stored in the chat history. Extremely long outputs (e.g., compile logs) are truncated to save context window tokens.

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
- `/model <name>` - Show or switch current AI model
- `/sessions` - List all chat sessions
- `/resume <id>` - Switch to/resume a session
- `/undo` - Undo last file/shell action
- `/tokens` - Show current token usage
- `/savemem <msg>` - Save critical info to .deep/memory.md
- `/export` - Export session to Markdown
- `/update` - Check for and install the latest version
- `/clear` - Clear terminal screen
- `/forget` - Wipe current history from disk
- `/auto` - Toggle auto-approve mode
- `/info` - Show detailed session info
- `/config <key> [value]` - View or modify configuration (keys: model, base_url, temperature, top_p, presence_penalty, frequency_penalty, max_tokens, max_iterations, max_context_chars, max_tool_output_chars, show_usage, concise_reasoning, debug)
- `/temperature <value>` - Set model temperature
- `/retry` - Regenerate last assistant response
- `/exit`, `/quit` - Close the application

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
