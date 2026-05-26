use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

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

pub struct ScreenshotWebappTool;
#[async_trait]
impl Tool for ScreenshotWebappTool {
    fn name(&self) -> &str {
        "screenshot_webapp"
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
        let output_path = args
            .get("output_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'output_path'"))?;

        let p = crate::tools::base::validate_path(output_path)?;
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let p_str = p.to_string_lossy().to_string();

        let mut cmd = tokio::process::Command::new("msedge");
        cmd.arg("--headless")
            .arg("--disable-gpu")
            .arg(format!("--screenshot={}", p_str))
            .arg(url);

        let res = cmd.output().await;
        match res {
            Ok(output) if output.status.success() => Ok(format!("Screenshot saved to {}", p_str)),
            _ => {
                let mut cmd2 = tokio::process::Command::new("chrome");
                cmd2.arg("--headless")
                    .arg("--disable-gpu")
                    .arg(format!("--screenshot={}", p_str))
                    .arg(url);
                match cmd2.output().await {
                    Ok(output) if output.status.success() => {
                        Ok(format!("Screenshot saved to {}", p_str))
                    }
                    Ok(output) => {
                        Err(anyhow::anyhow!("Browser exited with error: {}", String::from_utf8_lossy(&output.stderr)))
                    }
                    Err(e) => {
                        Err(anyhow::anyhow!("Failed to start msedge or chrome. Make sure at least one is installed and in your PATH. Error: {}", e))
                    }
                }
            }
        }
    }
}

pub struct WebSearchDuckDuckGoTool;
#[async_trait]
impl Tool for WebSearchDuckDuckGoTool {
    fn name(&self) -> &str {
        "web_search_duckduckgo"
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
        tools::web_ops::web_search_duckduckgo(query).await
    }
}
