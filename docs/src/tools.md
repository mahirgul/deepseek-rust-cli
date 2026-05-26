# Tools

The agent has access to **65+ tools** to interact with your system, codebase, and external APIs. All tools are implemented using a trait-based registry system (`src/tools/base.rs`) providing type safety, automatic JSON Schema generation for the LLM, and flexible execution parameters.

---

## 📂 File Operations (27 tools)

### Read & Write
- `read_local_file`: Read content from a file within a specific line range.
- `write_local_file`: Create a new file or overwrite an existing one.
- `replace_text_in_file`: Perform precise, localized text substitution.
- `edit_file_by_lines`: Edit a file by specifying one or more non-overlapping line ranges.
- `apply_diff_patch`: Apply a unified diff patch to a local file.
- `regex_replace_in_file`: Apply regular expression replacements with group support.
- `json_update_value`: Update structured JSON config files using dot-separated paths (e.g., `dependencies.tokio.version`), supporting escaped dots (`\.`).

### File Management
- `copy_file`: Copy a file from source to destination natively.
- `copy_directory`: Recursively copy a directory natively.
- `create_directory`: Create a directory (and any necessary parent directories) natively.
- `file_exists`: Check if a file or directory exists at the given path.
- `get_file_info`: Get metadata for a file (type, size, timestamps, permissions) natively.
- `delete_file`: Delete a local file or directory.
- `rename_file`: Rename or move a local file or directory.
- `bulk_rename`: Rename multiple files in a directory using a regex pattern.

### Navigation & Inspection
- `list_directory`: List the contents of a directory.
- `tree_view`: View a recursive directory tree hierarchy up to a specified depth.
- `diff_files`: Compare the contents of two local files.
- `hash_file`: Calculate SHA256/MD5 hashes of local files.
- `count_lines`: Count lines, words, and characters in a file.
- `search_files`: Perform fast, parallel regex-based searches across.
- `list_symbols`: Parse function, struct, class, and impl definitions from Rust, Python, and JavaScript/TypeScript files using lightweight regex.
- `view_symbol_contents`: View the full implementation code of a specific symbol (function, class, struct, enum, or impl) from a file.

### Code Refactoring
- `move_code_block`: Move a code block (function, struct, etc.) from one file to another using regex.
- `split_file`: Split a file into multiple parts based on a regex pattern.
- `cleanup_file`: Clean up a file by removing trailing spaces and normalizing line endings.

### Project-Wide Operations
- `project_wide_replace`: Perform a global search and replace across the entire project (filtering target files by glob).
- `summarize_project`: Analyze the current project and provide a high-level summary of files, languages, and structure.
- `list_todo_tasks`: Search the project for TODO, FIXME, HACK, and BUG comments and list them with file and line info.
- `project_checkpoint`: Create a project-wide backup archive of source code and configuration.
- `restore_checkpoint`: Restore the project from a previously created checkpoint archive.

---

## 🐚 System Operations (9 tools)

- `execute_shell_command`: Execute system shell commands (runs with strict warnings, timeouts, optional background task tracking).
- `get_system_info`: Retrieve system architecture, OS, CPU, memory, and environment stats.
- `start_background_process`: Start a command in the background, piping stdout/stderr logs.
- `read_background_process_logs`: Read accumulated logs from a running background process.
- `kill_background_process`: Terminate a running background process.
- `list_background_processes`: List all active background processes started by the agent.
- `check_port_status`: Check if a local port is occupied, free, or blocked.
- `run_python_code`: Execute Python code snippets for arbitrary calculations or testing.
- `get_env_var`: Retrieve specific environment variables.

---

## 🐙 Git Operations (12 tools)

- `git_status`: Show git working tree status.
- `git_diff`: Show git diff (unstaged or staged).
- `git_log`: Show git commit history.
- `git_branch`: List, create, delete, or switch git branches.
- `git_add`: Stage files for commit.
- `git_commit`: Commit changes.
- `git_push`: Push commits to remote.
- `git_pull`: Pull changes from remote.
- `git_checkout`: Checkout branch or file.
- `git_clone`: Clone a git repository.
- `git_remote_list`: List git remotes.
- `git_stash`: Stash, pop, or list git stashes.

---

## 🔗 GitHub API (13 tools, requires `GITHUB_TOKEN`)

- `github_repo_info`: Get repository information.
- `github_repo_list_issues`: List issues in a repository.
- `github_issue_create`: Create a new issue.
- `github_issue_update`: Update an existing issue.
- `github_pr_list`: List pull requests.
- `github_pr_create`: Create a new pull request.
- `github_pr_info`: Get pull request details.
- `github_pr_merge`: Merge a pull request.
- `github_search_code`: Search code across repositories.
- `github_search_repos`: Search for repositories.
- `github_get_file`: Get file content from a repository.
- `github_workflow_list`: List GitHub Actions workflows.
- `github_workflow_runs`: List workflow run history.

---

## 🌐 Web & Search (3 tools)
- `web_search_duckduckgo`: Scrape DuckDuckGo HTML search results safely, filtering out ads and tracking scripts.
- `fetch_url`: Retrieve and format clean markdown text from web pages (stripping scripts, headers, and navigation).
- `screenshot_webapp`: Capture a PNG screenshot of a local web app or website by spawning a headless instance of Microsoft Edge or Google Chrome.

---

> **Total: 65+ tools** across 6 categories — File (27), System (9), Git (12), GitHub (13), Web (3), plus refactoring and project-wide utilities.
