use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, path::Path};

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

pub struct MoveCodeBlockTool;
#[async_trait]
impl Tool for MoveCodeBlockTool {
    fn name(&self) -> &str {
        "move_code_block"
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
        let pattern = args
            .get("block_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'block_pattern'"))?;

        // Backup both files
        let src_backup = tokio::fs::read(src).await.ok();
        let dst_backup = tokio::fs::read(dst).await.ok();

        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: src.to_string(),
            backup: src_backup,
        });
        undo.push(UndoAction {
            r#type: "replace".to_string(),
            path: dst.to_string(),
            backup: dst_backup,
        });

        tools::file_ops::move_code_block(src, dst, pattern).await
    }
}

pub struct SplitFileTool;
#[async_trait]
impl Tool for SplitFileTool {
    fn name(&self) -> &str {
        "split_file"
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
        let pattern = args
            .get("split_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'split_pattern'"))?;
        let prefix = args
            .get("output_prefix")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'output_prefix'"))?;

        tools::file_io::split_file(path, pattern, prefix).await
    }
}

pub struct CleanupFileTool;
#[async_trait]
impl Tool for CleanupFileTool {
    fn name(&self) -> &str {
        "cleanup_file"
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
            r#type: "replace".to_string(),
            path: path.to_string(),
            backup,
        });

        tools::file_io::cleanup_file(path).await
    }
}

pub struct ProjectCheckpointTool;
#[async_trait]
impl Tool for ProjectCheckpointTool {
    fn name(&self) -> &str {
        "project_checkpoint"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;

        let checkpoint_dir = Path::new(".deep/checkpoints");
        if !checkpoint_dir.exists() {
            std::fs::create_dir_all(checkpoint_dir)?;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.tar.gz", name, timestamp);
        let path = checkpoint_dir.join(&filename);

        // Use tar via shell
        let output = tokio::process::Command::new("tar")
            .args([
                "-czf",
                path.to_str().unwrap(),
                "src",
                "Cargo.toml",
                "README.md",
            ])
            .output()
            .await?;

        if output.status.success() {
            Ok(format!(
                "Project checkpoint '{}' created successfully.",
                filename
            ))
        } else {
            Err(anyhow::anyhow!(
                "Failed to create checkpoint: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

pub struct RestoreCheckpointTool;
#[async_trait]
impl Tool for RestoreCheckpointTool {
    fn name(&self) -> &str {
        "restore_checkpoint"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let name = args
            .get("checkpoint_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'checkpoint_file'"))?;

        let path = Path::new(".deep/checkpoints").join(name);
        if !path.exists() {
            return Err(anyhow::anyhow!("Checkpoint file not found: {}", name));
        }

        let output = tokio::process::Command::new("tar")
            .args(["-xzf", path.to_str().unwrap()])
            .output()
            .await?;

        if output.status.success() {
            Ok(format!("Project restored from checkpoint '{}'.", name))
        } else {
            Err(anyhow::anyhow!(
                "Failed to restore checkpoint: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

pub struct ProjectWideReplaceTool;
#[async_trait]
impl Tool for ProjectWideReplaceTool {
    fn name(&self) -> &str {
        "project_wide_replace"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let old_text = args
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_text'"))?;
        let new_text = args
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_text'"))?;
        let glob_pattern = args
            .get("glob")
            .and_then(|v| v.as_str())
            .unwrap_or("**/*.rs");

        let mut count = 0;
        let mut file_count = 0;

        // Use glob to match files
        if let Ok(paths) = glob::glob(glob_pattern) {
            for entry in paths.filter_map(|e| e.ok()) {
                if entry.is_file() {
                    let path_str = entry.to_string_lossy();
                    if !path_str.contains("target") && !path_str.contains(".git") {
                        if let Ok(content) = std::fs::read_to_string(&entry) {
                            if content.contains(old_text) {
                                let new_content = content.replace(old_text, new_text);
                                std::fs::write(&entry, new_content)?;
                                file_count += 1;
                                count += content.matches(old_text).count();
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "Replaced {} occurrences in {} files matching '{}'.",
            count, file_count, glob_pattern
        ))
    }
}
