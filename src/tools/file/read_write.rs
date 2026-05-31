use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

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
        let p = crate::tools::base::validate_path(path)?;
        let backup = tokio::fs::read(&p).await.ok();
        undo.push(UndoAction {
            r#type: "write".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });
        tools::file_io::write_local_file(p.to_str().unwrap(), content)
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
        let p = crate::tools::base::validate_path(path)?;
        let backup = tokio::fs::read(&p).await.ok();
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });
        tools::file_io::fuzzy_replace_in_file(p.to_str().unwrap(), old, new).await
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

pub struct EditFileByLinesTool;
#[async_trait]
impl Tool for EditFileByLinesTool {
    fn name(&self) -> &str {
        "edit_file_by_lines"
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
        let edits_val = args
            .get("edits")
            .ok_or_else(|| anyhow::anyhow!("Missing 'edits'"))?;

        let edits: Vec<tools::file_io::LineEdit> = serde_json::from_value(edits_val.clone())?;

        let p = crate::tools::base::validate_path(path)?;
        let backup = tokio::fs::read(&p).await.ok();
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });

        tools::file_io::edit_file_by_lines(p.to_str().unwrap(), edits).await
    }
}

pub struct ApplyDiffPatchTool;
#[async_trait]
impl Tool for ApplyDiffPatchTool {
    fn name(&self) -> &str {
        "apply_diff_patch"
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
        let patch_content = args
            .get("patch_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'patch_content'"))?;

        let p = crate::tools::base::validate_path(path)?;
        let backup = tokio::fs::read(&p).await.ok();
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: p.to_string_lossy().to_string(),
            backup,
        });

        tools::file_io::apply_diff_patch(p.to_str().unwrap(), patch_content).await
    }
}
