use crate::agent::agent::UndoAction;
use crate::tools;
use crate::tools::base::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct SystemInfoTool;
#[async_trait]
impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "get_system_info"
    }
    async fn execute(
        &self,
        _args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        tools::system::get_system_info()
    }
}

pub struct ShellTool;
#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "execute_shell_command"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        agent_cwd: Option<&Path>,
    ) -> Result<String> {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command'"))?;
        let bg = args
            .get("is_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .or_else(|| agent_cwd.and_then(|p| p.to_str()));
        let env_vars = args.get("env").and_then(|v| v.as_object()).map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        });
        tools::system::execute_shell_command(cmd, bg, cwd, env_vars).await
    }
}
