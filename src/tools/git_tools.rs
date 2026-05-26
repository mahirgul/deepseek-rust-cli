use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

pub struct GitStatusTool;
#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        tools::git_ops::git_status(path).await
    }
}

pub struct GitDiffTool;
#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let staged = args
            .get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        tools::git_ops::git_diff(path, staged).await
    }
}

pub struct GitLogTool;
#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let count = args
            .get("count")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::git_ops::git_log(path, count).await
    }
}

pub struct GitBranchTool;
#[async_trait]
impl Tool for GitBranchTool {
    fn name(&self) -> &str {
        "git_branch"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let action = args.get("action").and_then(|v| v.as_str());
        let name = args.get("name").and_then(|v| v.as_str());
        tools::git_ops::git_branch(path, action, name).await
    }
}

pub struct GitAddTool;
#[async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &str {
        "git_add"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let files = args.get("files").and_then(|v| v.as_str());
        tools::git_ops::git_add(path, files).await
    }
}

pub struct GitCommitTool;
#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message'"))?;
        tools::git_ops::git_commit(path, message).await
    }
}

pub struct GitPushTool;
#[async_trait]
impl Tool for GitPushTool {
    fn name(&self) -> &str {
        "git_push"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let remote = args.get("remote").and_then(|v| v.as_str());
        let branch = args.get("branch").and_then(|v| v.as_str());
        tools::git_ops::git_push(path, remote, branch).await
    }
}

pub struct GitPullTool;
#[async_trait]
impl Tool for GitPullTool {
    fn name(&self) -> &str {
        "git_pull"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let remote = args.get("remote").and_then(|v| v.as_str());
        let branch = args.get("branch").and_then(|v| v.as_str());
        tools::git_ops::git_pull(path, remote, branch).await
    }
}

pub struct GitCheckoutTool;
#[async_trait]
impl Tool for GitCheckoutTool {
    fn name(&self) -> &str {
        "git_checkout"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'target'"))?;
        tools::git_ops::git_checkout(path, target).await
    }
}

pub struct GitCloneTool;
#[async_trait]
impl Tool for GitCloneTool {
    fn name(&self) -> &str {
        "git_clone"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url'"))?;
        let dest = args.get("dest").and_then(|v| v.as_str());
        tools::git_ops::git_clone(url, dest).await
    }
}

pub struct GitRemoteListTool;
#[async_trait]
impl Tool for GitRemoteListTool {
    fn name(&self) -> &str {
        "git_remote_list"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        tools::git_ops::git_remote_list(path).await
    }
}

pub struct GitStashTool;
#[async_trait]
impl Tool for GitStashTool {
    fn name(&self) -> &str {
        "git_stash"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        let action = args.get("action").and_then(|v| v.as_str());
        tools::git_ops::git_stash(path, action).await
    }
}
