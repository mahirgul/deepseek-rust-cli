use crate::agent::agent::UndoAction;
use crate::tools;
use crate::tools::base::ToolRegistry;
use anyhow::Result;
use futures::future::join_all;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(120);
const LONG_TOOL_TIMEOUT: Duration = Duration::from_secs(600);
/// Cache TTL for read-only tool results (5 seconds)
const CACHE_TTL: Duration = Duration::from_secs(5);

/// Read-only tools that can be safely cached
const CACHEABLE_TOOLS: &[&str] = &[
    "read_local_file",
    "list_directory",
    "tree_view",
    "diff_files",
    "hash_file",
    "count_lines",
    "get_system_info",
    "get_env_var",
    "git_status",
    "git_diff",
    "git_log",
    "git_branch",
    "git_remote_list",
    "git_stash",
    "github_repo_info",
    "github_repo_list_issues",
    "github_pr_list",
    "github_pr_info",
    "github_search_code",
    "github_search_repos",
    "github_get_file",
    "github_workflow_list",
    "github_workflow_runs",
];

static TOOL_REGISTRY: Lazy<ToolRegistry> = Lazy::new(|| {
    let mut registry = ToolRegistry::new();
    for tool in tools::get_all_tools() {
        registry.register(tool);
    }
    registry
});

/// Tool result cache entry
#[derive(Clone)]
pub struct CacheEntry {
    pub timestamp: Instant,
    pub result: String,
}

/// Global tool cache — survives across tool calls within the same iteration
pub type ToolCache = HashMap<String, CacheEntry>;

/// Get appropriate timeout for a given tool
fn tool_timeout(name: &str) -> Duration {
    match name {
        "git_clone" | "git_push" | "git_pull" | "execute_shell_command" | "fetch_url" => {
            LONG_TOOL_TIMEOUT
        }
        _ => DEFAULT_TOOL_TIMEOUT,
    }
}

/// Build a cache key from tool name and args
fn cache_key(name: &str, args_val: &serde_json::Map<String, Value>) -> String {
    let mut key = name.to_string();
    let mut sorted: Vec<(&String, &Value)> = args_val.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in sorted {
        key.push(':');
        key.push_str(k);
        key.push('=');
        key.push_str(&v.to_string());
    }
    key
}

fn is_cacheable(name: &str) -> bool {
    CACHEABLE_TOOLS.contains(&name)
}

pub async fn execute_tool(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    agent_cwd: Option<&std::path::Path>,
) -> Result<String> {
    execute_tool_inner(name, args_val, undo_stack, agent_cwd).await
}

async fn execute_tool_inner(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    agent_cwd: Option<&std::path::Path>,
) -> Result<String> {
    let timeout = tool_timeout(name);

    tokio::time::timeout(
        timeout,
        execute_tool_raw(name, args_val, undo_stack, agent_cwd),
    )
    .await
    .unwrap_or_else(|_| {
        Err(anyhow::anyhow!(
            "Tool '{}' timed out after {:?}",
            name,
            timeout
        ))
    })
}

async fn execute_tool_raw(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    agent_cwd: Option<&std::path::Path>,
) -> Result<String> {
    // Convert serde_json::Map to HashMap for the Tool trait
    let mut args = HashMap::new();
    for (k, v) in args_val {
        args.insert(k.clone(), v.clone());
    }

    TOOL_REGISTRY
        .execute(name, &args, undo_stack, agent_cwd)
        .await
}

/// Execute a tool with cache support.
/// Returns (result, was_cache_hit)
pub async fn execute_tool_cached(
    name: &str,
    args_val: &serde_json::Map<String, Value>,
    undo_stack: &mut Vec<UndoAction>,
    cache: &mut ToolCache,
    agent_cwd: Option<&std::path::Path>,
) -> (Result<String>, bool) {
    // Only cache read-only tools
    if is_cacheable(name) {
        let key = cache_key(name, args_val);
        let now = Instant::now();

        if let Some(entry) = cache.get(&key) {
            if now.duration_since(entry.timestamp) < CACHE_TTL {
                tracing::debug!(target: "cache", "Cache hit: {}", key);
                return (Ok(entry.result.clone()), true);
            }
        }

        let result = execute_tool(name, args_val, undo_stack, agent_cwd).await;
        if let Ok(ref res) = result {
            if res.len() < 50_000 {
                // Don't cache very large results
                cache.insert(
                    key,
                    CacheEntry {
                        timestamp: Instant::now(),
                        result: res.clone(),
                    },
                );
            }
        }
        (result, false)
    } else {
        (
            execute_tool(name, args_val, undo_stack, agent_cwd).await,
            false,
        )
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
