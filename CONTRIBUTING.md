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
git clone https://github.com/mahirgul/deepseek-rust-cli.git
cd deepseek-rust-cli

# Build the project
cargo build

# Run tests
cargo test

# Check for lint errors
cargo clippy --all-targets --all-features -- -D warnings
```

## 📐 Project Structure

- `src/agent/`: Core logic, command processing, security, and history.
- `src/api/`: DeepSeek API client, streaming, and type definitions.
-src/tools/`:it-based tool registry (`base.rs`, `schemas.rs`, `mod.rs`) and all built-in tools organized by domain:
  -src/tools/file/`: File operations (`read_write.rs`, `ops.rs`, `navigation.rs`, `diff.rs`, `analysis.rs`, `refactor.rs`)
  - `src/tools/git_ops.rs` /git_tools.rs`: Git operations
  - `src/tools/github_ops.rs` / `_tools.rs`: GitHub API
  - `src/tools/system.rs` / `system_tools.rs`: Shell and system info
  - `src/tools/web_ops.rs` / `web_tools.rs`: Web, search, Python, and screenshots
- `src/tui/`: Crossterm-based terminal user interface (event loop, colorizer, keywords).
## 🛠️ Adding a New Tool

To add a new tool to the agent's extensible registry, follow these steps:

1. **Implement the `Tool` Trait**:
   Create or modify a file in the appropriate domain subdirectory inside `src/tools/`. Define a struct for your tool and implement the `Tool` trait from `crate::tools::base::Tool`:
   ```rust
   #[async_trait]
   impl Tool for MyNewTool {
       fn name(&self) -> &str {
           "my_new_tool"
       }
       async fn execute(
           &self,
           args: &HashMap<String, Value>,
           undo_stack: &mut Vec<UndoAction>,
           cwd: Option<&Path>,
       ) -> Result<String> {
           // Your tool logic here
       }
   }
   ```

2. **Register the Tool**:
   Open `src/tools/mod.rs` and append your new tool instance inside `get_all_tools()`:
   ```rust
   Box::new(file::read_write::MyNewTool),
   ```

3. **Define JSON Schema**:
   Open `src/tools/schemas.rs` and push your tool's arguments schema inside `get_filtered_tools_schemas()`:
   ```rust
   tools.push(create_tool(
       "my_new_tool",
       "A clear description of what the tool does.",
       json!({
           "param_name": { "type": "string", "description": "Parameter details" }
       }),
       vec!["param_name"], // List of required parameters
   ));
   ```

4. **Verify**:
   Add integration/unit tests for the tool, ensure everything builds and tests pass by running `cargo test` and `cargo clippy`.

## 📜 License

By contributing, you agree that your contributions will be licensed under its MIT License.