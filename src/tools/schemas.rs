use serde_json::json;

use crate::api::types::Tool;

pub fn get_tools_schemas() -> Vec<Tool> {
    vec![
        // ─── Shell & System ──────────────────────────────────────
        create_tool(
            "execute_shell_command",
            "Execute a shell command.",
            json!({
                "command": { "type": "string" },
                "is_background": { "type": "boolean" }
            }),
            vec!["command"],
        ),
        create_tool(
            "get_system_info",
            "Get system information.",
            json!({}),
            vec![],
        ),
        // ─── File I/O ────────────────────────────────────────────
        create_tool(
            "read_local_file",
            "Read a local file.",
            json!({
                "file_path": { "type": "string" },
                "start_line": { "type": "integer" },
                "end_line": { "type": "integer" }
            }),
            vec!["file_path"],
        ),
        create_tool(
            "write_local_file",
            "Write to a local file.",
            json!({
                "file_path": { "type": "string" },
                "content": { "type": "string" }
            }),
            vec!["file_path", "content"],
        ),
        create_tool(
            "replace_text_in_file",
            "Replace text in a file.",
            json!({
                "file_path": { "type": "string" },
                "old_text": { "type": "string" },
                "new_text": { "type": "string" }
            }),
            vec!["file_path", "old_text", "new_text"],
        ),
        create_tool(
            "list_directory",
            "List directory contents.",
            json!({
                "path": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "tree_view",
            "Show directory tree.",
            json!({
                "path": { "type": "string" },
                "max_depth": { "type": "integer" }
            }),
            vec![],
        ),
        create_tool(
            "delete_file",
            "Delete a file or directory.",
            json!({
                "file_path": { "type": "string" }
            }),
            vec!["file_path"],
        ),
        create_tool(
            "rename_file",
            "Rename or move a file.",
            json!({
                "source_path": { "type": "string" },
                "destination_path": { "type": "string" }
            }),
            vec!["source_path", "destination_path"],
        ),
        create_tool(
            "diff_files",
            "Compare two files.",
            json!({
                "file1": { "type": "string" },
                "file2": { "type": "string" }
            }),
            vec!["file1", "file2"],
        ),
        create_tool(
            "hash_file",
            "Calculate file hash.",
            json!({
                "path": { "type": "string" },
                "algorithm": { "type": "string", "enum": ["sha256", "md5"] }
            }),
            vec!["path"],
        ),
        create_tool(
            "count_lines",
            "Count lines, words and characters in a file.",
            json!({
                "path": { "type": "string" }
            }),
            vec!["path"],
        ),
        create_tool(
            "search_files",
            "Search files for a text pattern using native Rust (no shell process needed). Fast parallel search with regex support.",
            json!({
                "query": { "type": "string" },
                "path": { "type": "string" },
                "glob": { "type": "string" },
                "max_results": { "type": "integer" }
            }),
            vec!["query"],
        ),
        // ─── Code & Web ─────────────────────────────────────────
        create_tool(
            "run_python_code",
            "Execute Python code snippet.",
            json!({
                "code": { "type": "string" }
            }),
            vec!["code"],
        ),
        create_tool(
            "fetch_url",
            "Fetch and clean content from a URL.",
            json!({
                "url": { "type": "string" }
            }),
            vec!["url"],
        ),
        create_tool(
            "get_env_var",
            "Read an environment variable.",
            json!({
                "name": { "type": "string" }
            }),
            vec!["name"],
        ),
        // ─── Local Git Operations ───────────────────────────────
        create_tool(
            "git_status",
            "Show git status.",
            json!({
                "path": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_diff",
            "Show git diff.",
            json!({
                "path": { "type": "string" },
                "staged": { "type": "boolean" }
            }),
            vec![],
        ),
        create_tool(
            "git_log",
            "Show git commit history.",
            json!({
                "path": { "type": "string" },
                "count": { "type": "integer" }
            }),
            vec![],
        ),
        create_tool(
            "git_branch",
            "List, create, delete, or switch git branches.",
            json!({
                "path": { "type": "string" },
                "action": { "type": "string", "enum": ["list", "create", "delete", "switch"] },
                "name": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_add",
            "Stage files for commit.",
            json!({
                "path": { "type": "string" },
                "files": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_commit",
            "Commit staged changes.",
            json!({
                "path": { "type": "string" },
                "message": { "type": "string" }
            }),
            vec!["message"],
        ),
        create_tool(
            "git_push",
            "Push commits to remote.",
            json!({
                "path": { "type": "string" },
                "remote": { "type": "string" },
                "branch": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_pull",
            "Pull changes from remote.",
            json!({
                "path": { "type": "string" },
                "remote": { "type": "string" },
                "branch": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_checkout",
            "Checkout branch or file.",
            json!({
                "path": { "type": "string" },
                "target": { "type": "string" }
            }),
            vec!["target"],
        ),
        create_tool(
            "git_clone",
            "Clone a git repository.",
            json!({
                "url": { "type": "string" },
                "dest": { "type": "string" }
            }),
            vec!["url"],
        ),
        create_tool(
            "git_remote_list",
            "List git remotes.",
            json!({
                "path": { "type": "string" }
            }),
            vec![],
        ),
        create_tool(
            "git_stash",
            "Stash, pop, or list git stashes.",
            json!({
                "path": { "type": "string" },
                "action": { "type": "string", "enum": ["save", "pop", "list"] }
            }),
            vec![],
        ),
        // ─── GitHub API Operations ─────────────────────────────
        create_tool(
            "github_repo_info",
            "Get GitHub repository information. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" }
            }),
            vec!["repo"],
        ),
        create_tool(
            "github_repo_list_issues",
            "List GitHub issues for a repository.",
            json!({
                "repo": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "closed", "all"] },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ),
        create_tool(
            "github_issue_create",
            "Create a GitHub issue. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "title": { "type": "string" },
                "body": { "type": "string" },
                "labels": { "type": "string" }
            }),
            vec!["repo", "title"],
        ),
        create_tool(
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
        ),
        create_tool(
            "github_pr_list",
            "List GitHub pull requests.",
            json!({
                "repo": { "type": "string" },
                "state": { "type": "string", "enum": ["open", "closed", "all"] },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ),
        create_tool(
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
        ),
        create_tool(
            "github_pr_info",
            "Get detailed information about a pull request.",
            json!({
                "repo": { "type": "string" },
                "pr_number": { "type": "integer" }
            }),
            vec!["repo", "pr_number"],
        ),
        create_tool(
            "github_pr_merge",
            "Merge a GitHub pull request. Requires GITHUB_TOKEN.",
            json!({
                "repo": { "type": "string" },
                "pr_number": { "type": "integer" },
                "method": { "type": "string", "enum": ["merge", "squash", "rebase"] }
            }),
            vec!["repo", "pr_number"],
        ),
        create_tool(
            "github_search_code",
            "Search code on GitHub. Requires GITHUB_TOKEN.",
            json!({
                "query": { "type": "string" },
                "repo": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["query"],
        ),
        create_tool(
            "github_search_repos",
            "Search GitHub repositories. Requires GITHUB_TOKEN.",
            json!({
                "query": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["query"],
        ),
        create_tool(
            "github_get_file",
            "Get file content from a GitHub repository.",
            json!({
                "repo": { "type": "string" },
                "path": { "type": "string" },
                "ref": { "type": "string" }
            }),
            vec!["repo", "path"],
        ),
        create_tool(
            "github_workflow_list",
            "List GitHub Actions workflows.",
            json!({
                "repo": { "type": "string" }
            }),
            vec!["repo"],
        ),
        create_tool(
            "github_workflow_runs",
            "List GitHub Actions workflow runs.",
            json!({
                "repo": { "type": "string" },
                "workflow_id": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            vec!["repo"],
        ),
    ]
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
