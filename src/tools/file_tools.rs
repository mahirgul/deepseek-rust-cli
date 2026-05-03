use crate::agent::agent::UndoAction;
use crate::tools;
use crate::tools::base::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct ReadFileTool;
#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_local_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path'"))?;
        let start = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let end = args
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        tools::file_io::read_local_file(path, start, end).await
    }
}

pub struct WriteFileTool;
#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_local_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path'"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content'"))?;
        let backup = tokio::fs::read(path).await.ok();
        undo.push(UndoAction {
            r#type: "write".to_string(),
            path: path.to_string(),
            backup,
        });
        tools::file_io::write_local_file(path, content)
            .await
            .map(|_| "File written.".to_string())
    }
}

pub struct ReplaceTextTool;
#[async_trait]
impl Tool for ReplaceTextTool {
    fn name(&self) -> &str {
        "replace_text_in_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path'"))?;
        let old = args
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_text'"))?;
        let new = args
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_text'"))?;
        let backup = tokio::fs::read(path).await.ok();
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: path.to_string(),
            backup,
        });
        tools::file_io::fuzzy_replace_in_file(path, old, new).await
    }
}

pub struct DeleteFileTool;
#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path'"))?;
        let backup = tokio::fs::read(path).await.ok();
        undo.push(UndoAction {
            r#type: "delete".to_string(),
            path: path.to_string(),
            backup,
        });
        tools::file_io::delete_file(path)
            .await
            .map(|_| "File deleted.".to_string())
    }
}

pub struct RenameFileTool;
#[async_trait]
impl Tool for RenameFileTool {
    fn name(&self) -> &str {
        "rename_file"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let src = args
            .get("source_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'source_path'"))?;
        let dst = args
            .get("destination_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'destination_path'"))?;
        undo.push(UndoAction {
            r#type: "rename".to_string(),
            path: dst.to_string(),
            backup: Some(src.as_bytes().to_vec()),
        });
        tools::file_io::rename_file(src, dst)
            .await
            .map(|_| "File moved.".to_string())
    }
}

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

pub struct DiffFilesTool;
#[async_trait]
impl Tool for DiffFilesTool {
    fn name(&self) -> &str {
        "diff_files"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let f1 = args
            .get("file1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file1'"))?;
        let f2 = args
            .get("file2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file2'"))?;
        tools::file_ops::diff_files(f1, f2).await
    }
}

pub struct HashFileTool;
#[async_trait]
impl Tool for HashFileTool {
    fn name(&self) -> &str {
        "hash_file"
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?
            .to_string();
        let alg = args
            .get("algorithm")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        tools::file_ops::hash_file(path, alg).await
    }
}

pub struct CountLinesTool;
#[async_trait]
impl Tool for CountLinesTool {
    fn name(&self) -> &str {
        "count_lines"
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?
            .to_string();
        tools::file_ops::count_lines(path).await
    }
}
