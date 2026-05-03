# Contributing to DeepSeek Rust CLI Agent 🚀

First off, thank you for considering contributing to DeepSeek Rust CLI! It's people like you that make this tool better for everyone.

## 🤝 Code of Conduct

By participating in this project, you agree to abide by our standards of professionalism and respect.

## 🛠️ How Can I Contribute?

### Reporting Bugs
- Use the [GitHub Issue Tracker](https://github.com/mahirgul/deepseek-rust-cli/issues).
- Describe the bug, steps to reproduce, and your environment (OS, Rust version).

### Suggesting Enhancements
- Open an issue with the "enhancement" label.
- Explain why the feature would be useful.

### Pull Requests
1. **Fork the repo** and create your branch from `main`.
2. **Install dependencies**: Ensure you have Rust and Cargo installed.
3. **Make your changes**: Follow the existing code style.
4. **Run tests**: `cargo test` and `cargo clippy` must pass.
5. **Update docs**: If you added a tool or feature, update `README.md` or relevant Wiki pages.
6. **Submit the PR**: Provide a clear description of what you've done.

## 🏗️ Development Setup

```bash
# Clone the repository
git clone https://github.com/your-username/deepseek-rust-cli.git
cd deepseek-rust-cli

# Build the project
cargo build

# Run tests
cargo test

# Check for lint errors
cargo clippy --all-targets --all-features -- -D warnings
```

## 📐 Project Structure

- `src/agent/`: Core logic, command processing, and history.
- `src/api/`: DeepSeek API client and streaming implementation.
- `src/tools/`: The trait-based tool registry and all built-in tools.
- `src/tui/`: Ratatui-based terminal user interface.

## 📜 License

By contributing, you agree that your contributions will be licensed under its MIT License.
