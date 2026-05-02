use crate::agent::agent::UndoAction;
use crate::tools;
use anyhow::Result;
use futures::future::join_all;
use serde_json::Value;
use std::time::Duration;

const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(120);
const LONG_TOOL_TIMEOUT: Duration = Duration::from_secs(600);

/// Get appropriate timeout for a given tool
fn tool_timeout(name: &str) -> Duration {
    match name {
        "git_clone" | "git_push" | "git_pull" | "execute_shell_command" | "fetch_url" => LONG_TOOL_TIMEOUT,
        _ => DEFAULT_TOOL_TIMEOUT,
    }
}

pub async fn execute_tool(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    agent_cwd: Option<&std::path::Path>,
) -> Result<String> {
    let timeout = tool_timeout(name);

    tokio::time::timeout(timeout, execute_tool_inner(name, args_val, undo_stack, agent_cwd))
        .await
        .unwrap_or_else(|_| Ok(format!("Tool '{}' timed out after {:?}", name, timeout)))
}

async fn execute_tool_inner(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    agent_cwd: Option<&std::path::Path>,
) -> Result<String> {
    match name {
        // ... (match cases)
        // ─── File I/O ────────────────────────────────────────────
        "read_local_file" => {
            let path = args_val
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let start = args_val
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let end = args_val
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::file_io::read_local_file(path, start, end).await
        }
        "write_local_file" => {
            let path = args_val
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content = args_val
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let backup = tokio::fs::read(path).await.ok();
            undo_stack.push(UndoAction {
                r#type: "write".to_string(),
                path: path.to_string(),
                backup,
            });
            tools::file_io::write_local_file(path, content)
                .await
                .map(|_| "File written.".to_string())
        }
        "replace_text_in_file" => {
            let path = args_val
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let old = args_val
                .get("old_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new = args_val
                .get("new_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let fuzzy = args_val
                .get("fuzzy")
                .and_then(|v| v.as_bool())
                .unwrap_or(true); // default to fuzzy matching
            let backup = tokio::fs::read(path).await.ok();
            undo_stack.push(UndoAction {
                r#type: "replace".to_string(),
                path: path.to_string(),
                backup,
            });
            if fuzzy {
                tools::file_io::fuzzy_replace_in_file(path, old, new).await
            } else {
                tools::file_io::replace_text_in_file(path, old, new)
                    .await
                    .map(|_| "Text replaced.".to_string())
            }
        }
        "delete_file" => {
            let path = args_val
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let backup = tokio::fs::read(path).await.ok();
            undo_stack.push(UndoAction {
                r#type: "delete".to_string(),
                path: path.to_string(),
                backup,
            });
            tools::file_io::delete_file(path)
                .await
                .map(|_| "File deleted.".to_string())
        }
        "rename_file" => {
            let src = args_val
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dst = args_val
                .get("destination_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            undo_stack.push(UndoAction {
                r#type: "rename".to_string(),
                path: dst.to_string(),
                backup: Some(src.as_bytes().to_vec()),
            });
            tools::file_io::rename_file(src, dst)
                .await
                .map(|_| "File moved.".to_string())
        }

        // ─── Shell & System ──────────────────────────────────────
        "execute_shell_command" => {
            let cmd = args_val
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let bg = args_val
                .get("is_background")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            // Use provided cwd or agent's current cwd
            let cwd = args_val.get("cwd")
                .and_then(|v| v.as_str())
                .or_else(|| agent_cwd.and_then(|p| p.to_str()));
                
            let env_vars: Option<std::collections::HashMap<String, String>> = args_val
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect()
                });
            tools::system::execute_shell_command(cmd, bg, cwd, env_vars).await
        }
        "list_directory" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            tools::file_io::list_directory(path)
                .await
                .map(|v| v.join("\n"))
        }
        "tree_view" => {
            let path = args_val
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let depth = args_val
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::file_ops::tree_view(path, depth).await
        }
        "diff_files" => {
            let f1 = args_val
                .get("file1")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let f2 = args_val
                .get("file2")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::file_ops::diff_files(f1, f2).await
        }
        "hash_file" => {
            let path = args_val
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let alg = args_val
                .get("algorithm")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            tools::file_ops::hash_file(path, alg).await
        }
        "count_lines" => {
            let path = args_val
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            tools::file_ops::count_lines(path).await
        }

        // ─── Code & Web ─────────────────────────────────────────
        "run_python_code" => {
            let code = args_val
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::code_ops::run_python_code(code).await
        }
        "fetch_url" => {
            let url = args_val
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::web_ops::fetch_url(url).await
        }
        "get_env_var" => {
            let name = args_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(tools::web_ops::get_env_var(name))
        }
        "get_system_info" => tools::system::get_system_info(),

        // ─── Local Git Operations ───────────────────────────────
        "git_status" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            tools::git_ops::git_status(path).await
        }
        "git_diff" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let staged = args_val
                .get("staged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            tools::git_ops::git_diff(path, staged).await
        }
        "git_log" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let count = args_val
                .get("count")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::git_ops::git_log(path, count).await
        }
        "git_branch" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let action = args_val.get("action").and_then(|v| v.as_str());
            let name = args_val.get("name").and_then(|v| v.as_str());
            tools::git_ops::git_branch(path, action, name).await
        }
        "git_add" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let files = args_val.get("files").and_then(|v| v.as_str());
            tools::git_ops::git_add(path, files).await
        }
        "git_commit" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let message = args_val
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::git_ops::git_commit(path, message).await
        }
        "git_push" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let remote = args_val.get("remote").and_then(|v| v.as_str());
            let branch = args_val.get("branch").and_then(|v| v.as_str());
            tools::git_ops::git_push(path, remote, branch).await
        }
        "git_pull" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let remote = args_val.get("remote").and_then(|v| v.as_str());
            let branch = args_val.get("branch").and_then(|v| v.as_str());
            tools::git_ops::git_pull(path, remote, branch).await
        }
        "git_checkout" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let target = args_val
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::git_ops::git_checkout(path, target).await
        }
        "git_clone" => {
            let url = args_val
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dest = args_val.get("dest").and_then(|v| v.as_str());
            tools::git_ops::git_clone(url, dest).await
        }
        "git_remote_list" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            tools::git_ops::git_remote_list(path).await
        }
        "git_stash" => {
            let path = args_val.get("path").and_then(|v| v.as_str());
            let action = args_val.get("action").and_then(|v| v.as_str());
            tools::git_ops::git_stash(path, action).await
        }

        // ─── GitHub API Operations ─────────────────────────────
        "github_repo_info" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::github_ops::github_repo_info(repo).await
        }
        "github_repo_list_issues" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let state = args_val.get("state").and_then(|v| v.as_str());
            let limit = args_val
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::github_ops::github_repo_list_issues(repo, state, limit).await
        }
        "github_issue_create" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = args_val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let body = args_val.get("body").and_then(|v| v.as_str());
            let labels = args_val.get("labels").and_then(|v| v.as_str());
            tools::github_ops::github_issue_create(repo, title, body, labels).await
        }
        "github_issue_update" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let issue_number = args_val
                .get("issue_number")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let title = args_val.get("title").and_then(|v| v.as_str());
            let body = args_val.get("body").and_then(|v| v.as_str());
            let state = args_val.get("state").and_then(|v| v.as_str());
            tools::github_ops::github_issue_update(repo, issue_number, title, body, state).await
        }
        "github_pr_list" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let state = args_val.get("state").and_then(|v| v.as_str());
            let limit = args_val
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::github_ops::github_pr_list(repo, state, limit).await
        }
        "github_pr_create" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = args_val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let head = args_val
                .get("head")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let base = args_val
                .get("base")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let body = args_val.get("body").and_then(|v| v.as_str());
            let draft = args_val
                .get("draft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            tools::github_ops::github_pr_create(repo, title, head, base, body, draft).await
        }
        "github_pr_info" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let pr_number = args_val
                .get("pr_number")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            tools::github_ops::github_pr_info(repo, pr_number).await
        }
        "github_pr_merge" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let pr_number = args_val
                .get("pr_number")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let method = args_val.get("method").and_then(|v| v.as_str());
            tools::github_ops::github_pr_merge(repo, pr_number, method).await
        }
        "github_search_code" => {
            let query = args_val
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let repo = args_val.get("repo").and_then(|v| v.as_str());
            let limit = args_val
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::github_ops::github_search_code(query, repo, limit).await
        }
        "github_search_repos" => {
            let query = args_val
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = args_val
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::github_ops::github_search_repos(query, limit).await
        }
        "github_get_file" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = args_val
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ref_ = args_val.get("ref").and_then(|v| v.as_str());
            tools::github_ops::github_get_file(repo, path, ref_).await
        }
        "github_workflow_list" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tools::github_ops::github_workflow_list(repo).await
        }
        "github_workflow_runs" => {
            let repo = args_val
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let workflow_id = args_val.get("workflow_id").and_then(|v| v.as_str());
            let limit = args_val
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            tools::github_ops::github_workflow_runs(repo, workflow_id, limit).await
        }

        _ => Ok(format!("Tool {} not implemented or unknown.", name)),
    }
}

/// Execute multiple independent tool calls in parallel
pub async fn execute_tools_parallel(
    tool_calls: &[(String, serde_json::Map<String, Value>)],
    agent_cwd: Option<std::path::PathBuf>,
) -> Vec<(usize, Result<String>, Vec<UndoAction>)> {
    let futures: Vec<_> = tool_calls
        .iter()
        .enumerate()
        .map(|(idx, (name, args))| {
            let name = name.clone();
            let args = args.clone();
            let cwd = agent_cwd.clone();
            async move {
                let mut temp_undo = Vec::new();
                let result = execute_tool(&name, &args, &mut temp_undo, cwd.as_deref()).await;
                (idx, result, temp_undo)
            }
        })
        .collect();

    join_all(futures).await
}
