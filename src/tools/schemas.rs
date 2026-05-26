use serde_json::json;

use crate::api::types::Tool;

/// Full unfiltered tool list (used when context detection isn't available).
pub fn get_tools_schemas() -> Vec<Tool> {
    get_filtered_tools_schemas(true, true)
}

/// Return tool schemas filtered by context:
/// - `is_git_repo`: include local git tools (status, diff, commit, push, etc.)
/// - `has_github_token`: include GitHub API tools
///
/// Core tools (shell, file I/O, code/web) are always included.
pub fn get_filtered_tools_schemas(is_git_repo: bool, has_github_token: bool) -> Vec<Tool> {
    let mut tools = Vec::with_capacity(40);

    // ─── Shell & System (always) ────────────────────────────
    tools.push(create_tool(
        "execute_shell_command",
        "Execute a shell command.",
        json!({
            "command": { "type": "string" },
            "is_background": { "type": "boolean" }
        }),
        vec!["command"],
    ));
    tools.push(create_tool(
        "get_system_info",
        "Get system information.",
        json!({}),
        vec![],
    ));

    // ─── File I/O (always) ──────────────────────────────────
    tools.push(create_tool(
        "read_local_file",
        "Read a local file.",
        json!({
            "file_path": { "type": "string" },
            "start_line": { "type": "integer" },
            "end_line": { "type": "integer" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "write_local_file",
        "Write to a local file.",
        json!({
            "file_path": { "type": "string" },
            "content": { "type": "string" }
        }),
        vec!["file_path", "content"],
    ));
    tools.push(create_tool(
        "replace_text_in_file",
        "Replace text in a file.",
        json!({
            "file_path": { "type": "string" },
            "old_text": { "type": "string" },
            "new_text": { "type": "string" }
        }),
        vec!["file_path", "old_text", "new_text"],
    ));
    tools.push(create_tool(
        "list_directory",
        "List directory contents.",
        json!({
            "path": { "type": "string" }
        }),
        vec![],
    ));
    tools.push(create_tool(
        "tree_view",
        "Show directory tree.",
        json!({
            "path": { "type": "string" },
            "max_depth": { "type": "integer" }
        }),
        vec![],
    ));
    tools.push(create_tool(
        "delete_file",
        "Delete a file or directory.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "rename_file",
        "Rename or move a file.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "diff_files",
        "Compare two files.",
        json!({
            "file1": { "type": "string" },
            "file2": { "type": "string" }
        }),
        vec!["file1", "file2"],
    ));
    tools.push(create_tool(
        "hash_file",
        "Calculate file hash.",
        json!({
            "path": { "type": "string" },
            "algorithm": { "type": "string", "enum": ["sha256", "md5"] }
        }),
        vec!["path"],
    ));
    tools.push(create_tool(
        "count_lines",
        "Count lines, words and characters in a file.",
        json!({
            "path": { "type": "string" }
        }),
        vec!["path"],
    ));
    tools.push(create_tool(
        "search_files",
        "Search files for a text pattern using native Rust (no shell process needed). Fast \
         parallel search with regex support.",
        json!({
            "query": { "type": "string" },
            "path": { "type": "string" },
            "glob": { "type": "string" },
            "max_results": { "type": "integer" }
        }),
        vec!["query"],
    ));
    tools.push(create_tool(
        "bulk_rename",
        "Rename multiple files in a directory using a regex pattern.",
        json!({
            "path": { "type": "string" },
            "pattern": { "type": "string" },
            "replacement": { "type": "string" }
        }),
        vec!["path", "pattern", "replacement"],
    ));

    tools.push(create_tool(
        "copy_file",
        "Copy a file from source_path to destination_path natively.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "copy_directory",
        "Recursively copy a directory from source_path to destination_path natively.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "create_directory",
        "Create a directory (and any necessary parent directories) natively.",
        json!({
            "directory_path": { "type": "string" }
        }),
        vec!["directory_path"],
    ));
    tools.push(create_tool(
        "file_exists",
        "Check if a file or directory exists at the given path.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "get_file_info",
        "Get metadata for a file (type, size, timestamps, permissions) natively.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));

    // ─── Code & Web (always) ───────────────────────────────
    tools.push(create_tool(
        "run_python_code",
        "Execute Python code snippet.",
        json!({
            "code": { "type": "string" }
        }),
        vec!["code"],
    ));
    tools.push(create_tool(
        "fetch_url",
        "Fetch and clean content from a URL.",
        json!({
            "url": { "type": "string" }
        }),
        vec!["url"],
    ));
    tools.push(create_tool(
        "get_env_var",
        "Read an environment variable.",
        json!({
            "name": { "type": "string" }
        }),
        vec!["name"],
    ));

    // ─── New Advanced Tools ─────────────────────────────────
    tools.push(create_tool(
        "regex_replace_in_file",
        "Replace text in a file using a regular expression.",
        json!({
            "file_path": { "type": "string" },
            "regex": { "type": "string" },
            "replacement": { "type": "string" }
        }),
        vec!["file_path", "regex", "replacement"],
    ));
    tools.push(create_tool(
        "json_update_value",
        "Read a JSON file, update a value at a specified key path (e.g. 'dependencies.tokio.version'), and save it.",
        json!({
            "file_path": { "type": "string" },
            "key_path": { "type": "string" },
            "new_value": { "type": "string" }
        }),
        vec!["file_path", "key_path", "new_value"],
    ));
    tools.push(create_tool(
        "list_symbols",
        "Parse code symbols (functions, structs, classes, etc.) from a file using lightweight regex.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "screenshot_webapp",
        "Take a screenshot of a local web app or website using Microsoft Edge or Google Chrome in headless mode.",
        json!({
            "url": { "type": "string" },
            "output_path": { "type": "string" }
        }),
        vec!["url", "output_path"],
    ));
    tools.push(create_tool(
        "web_search_duckduckgo",
        "Perform an internet search query via DuckDuckGo and return top results.",
        json!({
            "query": { "type": "string" }
        }),
        vec!["query"],
    ));

    // ─── Refactoring & Advanced Ops ────────────────────────
    tools.push(create_tool(
        "move_code_block",
        "Move a code block (function, struct, etc.) from one file to another using regex.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" },
            "block_pattern": { "type": "string" }
        }),
        vec!["source_path", "destination_path", "block_pattern"],
    ));
    tools.push(create_tool(
        "split_file",
        "Split a file into multiple parts based on a regex pattern.",
        json!({
            "file_path": { "type": "string" },
            "split_pattern": { "type": "string" },
            "output_prefix": { "type": "string" }
        }),
        vec!["file_path", "split_pattern", "output_prefix"],
    ));
    tools.push(create_tool(
        "cleanup_file",
        "Clean up a file by removing trailing spaces and normalizing line endings.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "summarize_project",
        "Analyze the current project and provide a high-level summary of files, languages, and structure.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "list_todo_tasks",
        "Search the project for TODO, FIXME, HACK, and BUG comments and list them with file and line info.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "project_checkpoint",
        "Create a project-wide backup archive of the source code and configuration.",
        json!({
            "name": { "type": "string", "description": "Short mnemonic name for the checkpoint" }
        }),
        vec!["name"],
    ));
    tools.push(create_tool(
        "restore_checkpoint",
        "Restore the project from a previously created checkpoint archive.",
        json!({
            "checkpoint_file": { "type": "string", "description": "Filename of the .tar.gz checkpoint" }
        }),
        vec!["checkpoint_file"],
    ));
    tools.push(create_tool(
        "project_wide_replace",
        "Perform a global search and replace across the entire project (filtering target files by glob).",
        json!({
            "old_text": { "type": "string" },
            "new_text": { "type": "string" },
            "glob": { "type": "string", "description": "Glob pattern for files, e.g. '**/*.rs'" }
        }),
        vec!["old_text", "new_text"],
    ));

    // ─── Local Git Operations (only if in a git repo) ──────
    if is_git_repo {
        tools.push(create_tool(
            "git_status",
            "Show git status.",
            json!({
                "path": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_diff",
            "Show git diff.",
            json!({
                "path": { "type": "string" },
                "staged": { "type": "boolean" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_log",
            "Show git commit history.",
            json!({
                "path": { "type": "string" },
                "count": { "type": "integer" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_branch",
            "List, create, delete, or switch git branches.",
            json!({
                "path": { "type": "string" },
                "action": { "type": "string", "enum": ["list", "create", "delete", "switch"] },
                "name": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_add",
            "Stage files for commit.",
            json!({
                "path": { "type": "string" },
                "files": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_commit",
            "Commit staged changes.",
            json!({
                "path": { "type": "string" },
                "message": { "type": "string" }
            }),
            vec!["message"],
        ));
        tools.push(create_tool(
            "git_push",
            "Push commits to remote.",
            json!({
                "path": { "type": "string" },
                "remote": { "type": "string" },
                "branch": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_pull",
            "Pull changes from remote.",
            json!({
                "path": { "type": "string" },
                "remote": { "type": "string" },
                "branch": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_checkout",
            "Checkout branch or file.",
            json!({
                "path": { "type": "string" },
                "target": { "type": "string" }
            }),
            vec!["target"],
        ));
        tools.push(create_tool(
            "git_clone",
            "Clone a git repository.",
            json!({
                "url": { "type": "string" },
                "dest": { "type": "string" }
            }),
            vec!["url"],
        ));
        tools.push(create_tool(
            "git_remote_list",
            "List git remotes.",
            json!({
                "path": { "type": "string" }
            }),
            vec![],
        ));
        tools.push(create_tool(
            "git_stash",
            "Stash, pop, or list git stashes.",
            json!({
                "path": { "type": "string" },
                "action": { "type": "string", "enum": ["save", "pop", "list"] }
            }),
            vec![],
        ));
    }

    // ─── GitHub API Operations (only if GITHUB_TOKEN is set)
    if has_github_token {
        tools.push(create_tool(
            "github_repo_info",
            "Get GitHub repository information. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" }
            }),
            vec!["repo"],
        ));
        tools.push(create_tool(
            "github_repo_list_issues",
            "List GitHub issues for a repository.",
            json!({
                "repo": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "closed", "all"] },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ));
        tools.push(create_tool(
            "github_issue_create",
            "Create a GitHub issue. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "title": { "type": "string" },
                "body": { "type": "string" },
                "labels": { "type": "string" }
            }),
            vec!["repo", "title"],
        ));
        tools.push(create_tool(
            "github_issue_update",
            "Update a GitHub issue. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "issue_number": { "type": "integer" },
                "title": { "type": "string" },
                "body": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "closed"] }
            }),
            vec!["repo", "issue_number"],
        ));
        tools.push(create_tool(
            "github_pr_list",
            "List GitHub pull requests.",
            json!({
                "repo": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "closed", "all"] },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ));
        tools.push(create_tool(
            "github_pr_create",
            "Create a GitHub pull request. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "title": { "type": "string" },
                "head": { "type": "string" },
                "base": { "type": "string" },
                "body": { "type": "string" },
                "draft": { "type": "boolean" }
            }),
            vec!["repo", "title", "head", "base"],
        ));
        tools.push(create_tool(
            "github_pr_info",
            "Get detailed information about a pull request.",
            json!({
                "repo": { "type": "string" },
                "pr_number": { "type": "integer" }
            }),
            vec!["repo", "pr_number"],
        ));
        tools.push(create_tool(
            "github_pr_merge",
            "Merge a GitHub pull request. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "pr_number": { "type": "integer" },
                "method": { "type": "string", "enum": ["merge", "squash", "rebase"] }
            }),
            vec!["repo", "pr_number"],
        ));
        tools.push(create_tool(
            "github_search_code",
            "Search code on GitHub. Requires GITHUB_TOKEN.",
            json!({
                "query": { "type": "string" },
                "repo": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["query"],
        ));
        tools.push(create_tool(
            "github_search_repos",
            "Search GitHub repositories. Requires GITHUB_TOKEN.",
            json!({
                "query": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["query"],
        ));
        tools.push(create_tool(
            "github_get_file",
            "Get file content from a GitHub repository.",
            json!({
                "repo": { "type": "string" },
                "path": { "type": "string" },
                "ref": { "type": "string" }
            }),
            vec!["repo", "path"],
        ));
        tools.push(create_tool(
            "github_workflow_list",
            "List GitHub Actions workflows.",
            json!({
                "repo": { "type": "string" }
            }),
            vec!["repo"],
        ));
        tools.push(create_tool(
            "github_workflow_runs",
            "List GitHub Actions workflow runs.",
            json!({
                "repo": { "type": "string" },
                "workflow_id": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ));
    }

    tools
}

fn create_tool(name: &str, desc: &str, props: serde_json::Value, required: Vec<&str>) -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: crate::api::types::FunctionDefinition {
            name: name.to_string(),
            description: desc.to_string(),
            parameters: json!({
                "type": "object",
                "properties": props,
                "required": required
            }),
        },
    }
}
