use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::agent::UndoAction, tools, tools::base::Tool};

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

pub struct RegexReplaceTool;
#[async_trait]
impl Tool for RegexReplaceTool {
    fn name(&self) -> &str {
        "regex_replace_in_file"
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
        let regex_str = args
            .get("regex")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'regex'"))?;
        let replacement = args
            .get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'replacement'"))?;

        let p = crate::tools::base::validate_path(path)?;
        let re = regex::Regex::new(regex_str)?;
        let content = tokio::fs::read_to_string(&p).await?;

        let backup = Some(content.as_bytes().to_vec());
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });

        let new_content = re.replace_all(&content, replacement).to_string();
        tokio::fs::write(&p, new_content).await?;
        Ok("Regex replacement complete.".to_string())
    }
}

pub struct JsonUpdateValueTool;
#[async_trait]
impl Tool for JsonUpdateValueTool {
    fn name(&self) -> &str {
        "json_update_value"
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
        let key_path = args
            .get("key_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key_path'"))?;
        let new_value_str = args
            .get("new_value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_value'"))?;

        let p = crate::tools::base::validate_path(path)?;
        let new_val: serde_json::Value = serde_json::from_str(new_value_str)
            .unwrap_or_else(|_| serde_json::Value::String(new_value_str.to_string()));

        let raw_content = tokio::fs::read(&p).await?;
        let mut json_data: serde_json::Value = serde_json::from_slice(&raw_content)?;

        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: p.to_string_lossy().to_string(),
            backup: Some(raw_content),
        });

        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut chars = key_path.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' && chars.peek() == Some(&'.') {
                current_part.push('.');
                chars.next();
            } else if c == '.' {
                parts.push(current_part);
                current_part = String::new();
            } else {
                current_part.push(c);
            }
        }
        parts.push(current_part);

        if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
            return Err(anyhow::anyhow!("Empty key_path"));
        }

        let mut current = &mut json_data;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(part.clone(), new_val.clone());
                } else {
                    return Err(anyhow::anyhow!("Value at path is not a JSON object"));
                }
            } else {
                if !current.is_object() {
                    *current = serde_json::Value::Object(serde_json::Map::new());
                }
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(part) {
                    obj.insert(
                        part.clone(),
                        serde_json::Value::Object(serde_json::Map::new()),
                    );
                }
                current = obj.get_mut(part).unwrap();
            }
        }

        let updated_raw = serde_json::to_vec_pretty(&json_data)?;
        tokio::fs::write(&p, updated_raw).await?;
        Ok(format!("Successfully updated JSON path '{}'.", key_path))
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
