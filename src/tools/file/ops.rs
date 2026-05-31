use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

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
        let p = crate::tools::base::validate_path(path)?;
        let backup = tokio::fs::read(&p).await.ok();
        undo.push(UndoAction {
            r#type: "delete".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });
        tools::file_io::delete_file(p.to_str().unwrap())
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
        let src_p = crate::tools::base::validate_path(src)?;
        let dst_p = crate::tools::base::validate_path(dst)?;
        undo.push(UndoAction {
            r#type: "rename".to_string(),
            path: dst_p.to_string_lossy().to_string(),
            backup: Some(src_p.to_string_lossy().as_bytes().to_vec()),
        });
        tools::file_io::rename_file(src_p.to_str().unwrap(), dst_p.to_str().unwrap())
            .await
            .map(|_| "File moved.".to_string())
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

pub struct SearchFilesTool;
#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
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
        let path = args.get("path").and_then(|v| v.as_str());
        let glob = args.get("glob").and_then(|v| v.as_str());
        let max = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        tools::file_io::search_files(query, path, glob, max).await
    }
}

pub struct ListSymbolsTool;
#[async_trait]
impl Tool for ListSymbolsTool {
    fn name(&self) -> &str {
        "list_symbols"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path'"))?;
        let p = crate::tools::base::validate_path(path_str)?;
        if !p.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path_str));
        }

        let content = tokio::fs::read_to_string(&p).await?;
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut symbols = Vec::new();

        match ext {
            "rs" => {
                let re_fn =
                    regex::Regex::new(r"(?m)^(?:\s*pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*")?;
                let re_struct = regex::Regex::new(r"(?m)^(?:\s*pub\s+)?struct\s+([a-zA-Z0-9_]+)")?;
                let re_enum = regex::Regex::new(r"(?m)^(?:\s*pub\s+)?enum\s+([a-zA-Z0-9_]+)")?;
                let re_impl =
                    regex::Regex::new(r"(?m)^(?:\s*pub\s+)?impl(?:\s*<.*>)?\s+([a-zA-Z0-9_]+)")?;

                for (line_idx, line) in content.lines().enumerate() {
                    let line_num = line_idx + 1;
                    if let Some(cap) = re_fn.captures(line) {
                        symbols.push(format!("Line {}: fn {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_struct.captures(line) {
                        symbols.push(format!("Line {}: struct {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_enum.captures(line) {
                        symbols.push(format!("Line {}: enum {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_impl.captures(line) {
                        symbols.push(format!("Line {}: impl {}", line_num, &cap[1]));
                    }
                }
            }
            "py" => {
                let re_def = regex::Regex::new(r"(?m)^\s*(?:async\s+)?def\s+([a-zA-Z0-9_]+)\s*\(")?;
                let re_class = regex::Regex::new(r"(?m)^\s*class\s+([a-zA-Z0-9_]+)")?;

                for (line_idx, line) in content.lines().enumerate() {
                    let line_num = line_idx + 1;
                    if let Some(cap) = re_def.captures(line) {
                        symbols.push(format!("Line {}: def {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_class.captures(line) {
                        symbols.push(format!("Line {}: class {}", line_num, &cap[1]));
                    }
                }
            }
            "js" | "ts" | "jsx" | "tsx" => {
                let re_fn = regex::Regex::new(
                    r"(?m)^(?:\s*export\s+)?(?:async\s+)?function\s+([a-zA-Z0-9_]+)\s*",
                )?;
                let re_class = regex::Regex::new(r"(?m)^(?:\s*export\s+)?class\s+([a-zA-Z0-9_]+)")?;
                let re_const_fn = regex::Regex::new(
                    r"(?m)^(?:\s*export\s+)?const\s+([a-zA-Z0-9_]+)\s*=\s*(?:async\s*)?\(.*?\)\s*=>",
                )?;

                for (line_idx, line) in content.lines().enumerate() {
                    let line_num = line_idx + 1;
                    if let Some(cap) = re_fn.captures(line) {
                        symbols.push(format!("Line {}: function {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_class.captures(line) {
                        symbols.push(format!("Line {}: class {}", line_num, &cap[1]));
                    } else if let Some(cap) = re_const_fn.captures(line) {
                        symbols.push(format!("Line {}: const function {}", line_num, &cap[1]));
                    }
                }
            }
            _ => {
                let re_gen =
                    regex::Regex::new(r"(?m)(?:fn|def|function|class|struct)\s+([a-zA-Z0-9_]+)")?;
                for (line_idx, line) in content.lines().enumerate() {
                    let line_num = line_idx + 1;
                    if re_gen.is_match(line) {
                        symbols.push(format!("Line {}: {}", line_num, line.trim()));
                    }
                }
            }
        }

        if symbols.is_empty() {
            Ok("No symbols found in file.".to_string())
        } else {
            Ok(format!(
                "Symbols found in {}:\n{}",
                path_str,
                symbols.join("\n")
            ))
        }
    }
}

pub struct BulkRenameTool;
#[async_trait]
impl Tool for BulkRenameTool {
    fn name(&self) -> &str {
        "bulk_rename"
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern'"))?;
        let replacement = args
            .get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'replacement'"))?;

        tools::file_io::bulk_rename(path, pattern, replacement).await
    }
}

pub struct CopyFileTool;
#[async_trait]
impl Tool for CopyFileTool {
    fn name(&self) -> &str {
        "copy_file"
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

        let src_p = crate::tools::base::validate_path(src)?;
        let dst_p = crate::tools::base::validate_path(dst)?;

        // Support undo
        let backup = tokio::fs::read(&dst_p).await.ok();
        undo.push(UndoAction {
            r#type: "write".to_string(),
            path: dst_p.to_string_lossy().to_string(),
            backup,
        });

        tools::file_io::copy_local_file(src_p.to_str().unwrap(), dst_p.to_str().unwrap()).await?;
        Ok(format!("File copied from {} to {}.", src, dst))
    }
}

pub struct CopyDirectoryTool;
#[async_trait]
impl Tool for CopyDirectoryTool {
    fn name(&self) -> &str {
        "copy_directory"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
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

        tools::file_io::copy_directory(src, dst).await?;
        Ok(format!("Directory copied from {} to {}.", src, dst))
    }
}

pub struct CreateDirectoryTool;
#[async_trait]
impl Tool for CreateDirectoryTool {
    fn name(&self) -> &str {
        "create_directory"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let path = args
            .get("directory_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'directory_path'"))?;

        tools::file_io::create_directory(path).await?;
        Ok(format!("Directory created: {}", path))
    }
}

pub struct FileExistsTool;
#[async_trait]
impl Tool for FileExistsTool {
    fn name(&self) -> &str {
        "file_exists"
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

        let exists = tools::file_io::file_exists(path).await?;
        Ok(exists.to_string())
    }
}

pub struct GetFileInfoTool;
#[async_trait]
impl Tool for GetFileInfoTool {
    fn name(&self) -> &str {
        "get_file_info"
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

        tools::file_io::get_file_info(path).await
    }
}

pub struct ViewSymbolContentsTool;
#[async_trait]
impl Tool for ViewSymbolContentsTool {
    fn name(&self) -> &str {
        "view_symbol_contents"
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
        let symbol_name = args
            .get("symbol_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'symbol_name'"))?;

        tools::file_ops::view_symbol_contents(path, symbol_name).await
    }
}
