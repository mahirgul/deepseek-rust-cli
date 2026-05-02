use crate::agent::agent::UndoAction;
use crate::tools;
use crate::tools::base::ToolRegistry;
use anyhow::Result;
use futures::future::join_all;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(120);
const LONG_TOOL_TIMEOUT: Duration = Duration::from_secs(600);

static TOOL_REGISTRY: Lazy<ToolRegistry> = Lazy::new(|| {
    let mut registry = ToolRegistry::new();
    for tool in tools::get_all_tools() {
        registry.register(tool);
    }
    registry
});

/// Get appropriate timeout for a given tool
fn tool_timeout(name: &str) -> Duration {
    match name {
        "git_clone" | "git_push" | "git_pull" | "execute_shell_command" | "fetch_url" => {
            LONG_TOOL_TIMEOUT
        }
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

    tokio::time::timeout(
        timeout,
        execute_tool_inner(name, args_val, undo_stack, agent_cwd),
    )
    .await
    .unwrap_or_else(|_| Ok(format!("Tool '{}' timed out after {:?}", name, timeout)))
}

async fn execute_tool_inner(
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
