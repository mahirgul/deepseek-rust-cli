use super::create_tool;
use crate::api::types::Tool;
use serde_json::json;

pub fn add_git_schemas(tools: &mut Vec<Tool>) {
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
