use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, path::Path};
use walkdir::WalkDir;

use crate::{agent::types::UndoAction, tools::base::Tool};

pub struct ProjectSummaryTool;
#[async_trait]
impl Tool for ProjectSummaryTool {
    fn name(&self) -> &str {
        "summarize_project"
    }
    async fn execute(
        &self,
        _args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let mut extensions: HashMap<String, usize> = HashMap::new();
        let mut total_files = 0;
        let mut total_lines = 0;
        let mut core_files = Vec::new();

        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.to_string_lossy().contains(".git")
                    || path.to_string_lossy().contains("target")
                {
                    continue;
                }

                total_files += 1;
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *extensions.entry(ext.to_string()).or_insert(0) += 1;
                }

                // Check for core files
                let name = entry.file_name().to_string_lossy();
                if name == "Cargo.toml"
                    || name == "package.json"
                    || name == "main.rs"
                    || name == "lib.rs"
                {
                    core_files.push(path.display().to_string());
                }

                // Sample lines
                if let Ok(content) = std::fs::read_to_string(path) {
                    total_lines += content.lines().count();
                }
            }
        }

        let mut summary = format!(
            "Project Summary:\n- Total Files: {}\n- Total Lines: {}\n\nCore Files Found:\n",
            total_files, total_lines
        );
        for f in core_files {
            summary.push_str(&format!("  - {}\n", f));
        }

        summary.push_str("\nFile Extensions:\n");
        let mut ext_list: Vec<_> = extensions.into_iter().collect();
        ext_list.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (ext, count) in ext_list.into_iter().take(10) {
            summary.push_str(&format!("  - .{}: {} files\n", ext, count));
        }

        Ok(summary)
    }
}

pub struct ListTodoTasksTool;
#[async_trait]
impl Tool for ListTodoTasksTool {
    fn name(&self) -> &str {
        "list_todo_tasks"
    }
    async fn execute(
        &self,
        _args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let mut tasks = Vec::new();
        let keywords = ["TODO", "FIXME", "HACK", "BUG"];

        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.to_string_lossy().contains(".git")
                    || path.to_string_lossy().contains("target")
                {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(path) {
                    for (i, line) in content.lines().enumerate() {
                        for kw in &keywords {
                            if line.contains(kw) {
                                tasks.push(format!(
                                    "{}:{}: {}",
                                    path.display(),
                                    i + 1,
                                    line.trim()
                                ));
                            }
                        }
                    }
                }
            }
        }

        if tasks.is_empty() {
            Ok("No TODO/FIXME tasks found in project.".to_string())
        } else {
            Ok(format!(
                "Found {} tasks:\n\n{}",
                tasks.len(),
                tasks.join("\n")
            ))
        }
    }
}
