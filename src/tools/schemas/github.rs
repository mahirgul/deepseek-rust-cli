use super::create_tool;
use crate::api::types::Tool;
use serde_json::json;

pub fn add_github_schemas(tools: &mut Vec<Tool>) {
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
