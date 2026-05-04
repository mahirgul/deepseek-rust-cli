use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::agent::UndoAction, tools, tools::base::Tool};

pub struct GithubRepoInfoTool;
#[async_trait]
impl Tool for GithubRepoInfoTool {
    fn name(&self) -> &str {
        "github_repo_info"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        tools::github_ops::github_repo_info(repo).await
    }
}

pub struct GithubRepoListIssuesTool;
#[async_trait]
impl Tool for GithubRepoListIssuesTool {
    fn name(&self) -> &str {
        "github_repo_list_issues"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let state = args.get("state").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::github_ops::github_repo_list_issues(repo, state, limit).await
    }
}

pub struct GithubIssueCreateTool;
#[async_trait]
impl Tool for GithubIssueCreateTool {
    fn name(&self) -> &str {
        "github_issue_create"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'title'"))?;
        let body = args.get("body").and_then(|v| v.as_str());
        let labels = args.get("labels").and_then(|v| v.as_str());
        tools::github_ops::github_issue_create(repo, title, body, labels).await
    }
}

pub struct GithubIssueUpdateTool;
#[async_trait]
impl Tool for GithubIssueUpdateTool {
    fn name(&self) -> &str {
        "github_issue_update"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let issue_number = args
            .get("issue_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'issue_number'"))?;
        let title = args.get("title").and_then(|v| v.as_str());
        let body = args.get("body").and_then(|v| v.as_str());
        let state = args.get("state").and_then(|v| v.as_str());
        tools::github_ops::github_issue_update(repo, issue_number, title, body, state).await
    }
}

pub struct GithubPrListTool;
#[async_trait]
impl Tool for GithubPrListTool {
    fn name(&self) -> &str {
        "github_pr_list"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let state = args.get("state").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::github_ops::github_pr_list(repo, state, limit).await
    }
}

pub struct GithubPrCreateTool;
#[async_trait]
impl Tool for GithubPrCreateTool {
    fn name(&self) -> &str {
        "github_pr_create"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'title'"))?;
        let head = args
            .get("head")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'head'"))?;
        let base = args
            .get("base")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'base'"))?;
        let body = args.get("body").and_then(|v| v.as_str());
        let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
        tools::github_ops::github_pr_create(repo, title, head, base, body, draft).await
    }
}

pub struct GithubPrInfoTool;
#[async_trait]
impl Tool for GithubPrInfoTool {
    fn name(&self) -> &str {
        "github_pr_info"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let pr_number = args
            .get("pr_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pr_number'"))?;
        tools::github_ops::github_pr_info(repo, pr_number).await
    }
}

pub struct GithubPrMergeTool;
#[async_trait]
impl Tool for GithubPrMergeTool {
    fn name(&self) -> &str {
        "github_pr_merge"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let pr_number = args
            .get("pr_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pr_number'"))?;
        let method = args.get("method").and_then(|v| v.as_str());
        tools::github_ops::github_pr_merge(repo, pr_number, method).await
    }
}

pub struct GithubSearchCodeTool;
#[async_trait]
impl Tool for GithubSearchCodeTool {
    fn name(&self) -> &str {
        "github_search_code"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;
        let repo = args.get("repo").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::github_ops::github_search_code(query, repo, limit).await
    }
}

pub struct GithubSearchReposTool;
#[async_trait]
impl Tool for GithubSearchReposTool {
    fn name(&self) -> &str {
        "github_search_repos"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::github_ops::github_search_repos(query, limit).await
    }
}

pub struct GithubGetFileTool;
#[async_trait]
impl Tool for GithubGetFileTool {
    fn name(&self) -> &str {
        "github_get_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let ref_ = args.get("ref").and_then(|v| v.as_str());
        tools::github_ops::github_get_file(repo, path, ref_).await
    }
}

pub struct GithubWorkflowListTool;
#[async_trait]
impl Tool for GithubWorkflowListTool {
    fn name(&self) -> &str {
        "github_workflow_list"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        tools::github_ops::github_workflow_list(repo).await
    }
}

pub struct GithubWorkflowRunsTool;
#[async_trait]
impl Tool for GithubWorkflowRunsTool {
    fn name(&self) -> &str {
        "github_workflow_runs"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo'"))?;
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::github_ops::github_workflow_runs(repo, workflow_id, limit).await
    }
}
