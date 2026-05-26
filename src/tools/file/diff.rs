use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, path::Path};

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

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
