# Tools

The agent has access to a wide variety of tools to interact with your system and codebase.

## 📂 File Operations
- `read_file`: Read content from a file.
- `write_file`: Create or overwrite a file.
- `replace`: Perform surgical, fuzzy text replacement.
- `list_directory`: List files in a directory.
- `glob`: Search for files using patterns.

## 🐚 System Operations
- `run_shell_command`: Execute arbitrary shell commands (with timeouts and approval).
- `get_system_info`: Retrieve OS, CPU, and memory stats.

## 🐙 Git & GitHub
- `git_ops`: Direct git operations (status, diff, log).
- `github_tools`: Manage issues, pull requests, and search repositories.

## 🌐 Web & Search
- `google_web_search`: Search the web for up-to-date information.
- `web_fetch`: Fetch and analyze content from specific URLs.

## 🛠️ Tool Registry
All tools are implemented using a trait-based system located in `src/tools/`. This ensures type safety, automatic schema generation for the LLM, and easy extensibility.
