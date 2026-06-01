use serde_json::json;

use crate::api::types::Tool;

mod file_io;
mod git;
mod github;
mod system;
mod web;

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
    system::add_system_schemas(&mut tools);

    // ─── File I/O (always) ──────────────────────────────────
    file_io::add_file_io_schemas(&mut tools);

    // ─── Code & Web & Refactoring (always) ──────────────────
    web::add_web_schemas(&mut tools);

    // ─── Local Git Operations (only if in a git repo) ──────
    if is_git_repo {
        git::add_git_schemas(&mut tools);
    }

    // ─── GitHub API Operations (only if GITHUB_TOKEN is set)
    if has_github_token {
        github::add_github_schemas(&mut tools);
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
