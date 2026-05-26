use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

pub struct ListDirectoryTool;
#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args.get("path").and_then(|v| v.as_str());
        tools::file_io::list_directory(path)
            .await
            .map(|v| v.join("\n"))
    }
}

pub struct TreeViewTool;
#[async_trait]
impl Tool for TreeViewTool {
    fn name(&self) -> &str {
        "tree_view"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let depth = args
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::file_ops::tree_view(path, depth).await
    }
}
