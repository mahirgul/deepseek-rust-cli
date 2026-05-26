# Tools

The agent has access to a wide variety of tools to interact with your system, codebase, and external APIs.

## 📂 File Operations
- `read_local_file`: Read content from a local file within a specific line range.
- `write_local_file`: Create a new file or overwrite an existing one.
- `replace_text_in_file`: Perform precise, localized text substitution.
- `regex_replace_in_file` *(New)*: Apply regular expression replacements with capture group support.
- `json_update_value` *(New)*: Update structured JSON config files using dot-separated paths (e.g., `dependencies.tokio.version`), supporting escaped dots (`\.`).
- `list_directory`: List the contents of a directory.
- `tree_view`: View a recursive directory tree hierarchy up to a specified depth.
- `delete_file`: Delete a local file or directory.
- `rename_file`: Rename or move a local file or directory.
- `diff_files`: Compare the contents of two local files.
- `hash_file`: Calculate SHA256 or MD5 hashes of local files.
- `count_lines`: Count lines, words, and characters in a file.
- `search_files`: Perform fast, parallel regex-based searches across files.
- `list_symbols` *(New)*: Parse function, struct, class, and implementation definitions from Rust, Python, and JavaScript/TypeScript files using lightweight regular expressions.

## 🐚 System Operations
- `execute_shell_command`: Execute system shell commands (runs with strict warnings, timeouts, and optional background task tracking).
- `get_system_info`: Retrieve system architecture, OS, CPU, memory, and environment stats.
- `run_python_code`: Execute python code snippets for arbitrary calculations or testing.
- `get_env_var`: Retrieve specific environment variables.

## 🐙 Git & GitHub
- **Git Operations:** `git_status`, `git_diff`, `git_log`, `git_branch`, `git_add`, `git_commit`, `git_push`, `git_pull`, `git_checkout`, `git_clone`, `git_remote_list`, and `git_stash`.
- **GitHub API:** `github_repo_info`, `github_repo_list_issues`, `github_issue_create`, `github_issue_update`, `github_pr_list`, `github_pr_create`, `github_pr_info`, `github_pr_merge`, `github_search_code`, `github_search_repos`, `github_get_file`, `github_workflow_list`, and `github_workflow_runs` (requires `GITHUB_TOKEN`).

## 🌐 Web & Search
- `web_search_duckduckgo` *(New)*: Scrape DuckDuckGo HTML search results safely, filtering out ads and tracking scripts.
- `fetch_url`: Retrieve and format clean markdown text from web pages (stripping scripts, headers, and navigation).
- `screenshot_webapp` *(New)*: Capture a PNG screenshot of a local web app or website by spawning a headless instance of Microsoft Edge or Google Chrome (with automatic directory creation).

## 🛠️ Tool Registry
All tools are implemented using a trait-based registry system located in `src/tools/base.rs`. This provides type safety, automatic JSON Schema generation for the LLM client, and extensible execution parameters (such as path validation and transaction-based rollback tracking).
