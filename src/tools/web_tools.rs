use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::agent::UndoAction, tools, tools::base::Tool};

pub struct RunPythonTool;
#[async_trait]
impl Tool for RunPythonTool {
    fn name(&self) -> &str {
        "run_python_code"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'code'"))?;
        tools::code_ops::run_python_code(code).await
    }
}

pub struct FetchUrlTool;
#[async_trait]
impl Tool for FetchUrlTool {
    fn name(&self) -> &str {
        "fetch_url"
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
        tools::web_ops::fetch_url(url).await
    }
}

pub struct GetEnvVarTool;
#[async_trait]
impl Tool for GetEnvVarTool {
    fn name(&self) -> &str {
        "get_env_var"
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
        Ok(tools::web_ops::get_env_var(name))
    }
}
