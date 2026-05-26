use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::{agent::types::UndoAction, tools, tools::base::Tool};

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

pub struct StartBackgroundProcessTool;
#[async_trait]
impl Tool for StartBackgroundProcessTool {
    fn name(&self) -> &str {
        "start_background_process"
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
        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .or_else(|| agent_cwd.and_then(|p| p.to_str()));
        let env_vars = args.get("env").and_then(|v| v.as_object()).map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        });
        tools::system::start_background_process(cmd, cwd, env_vars).await
    }
}

pub struct ReadBackgroundProcessLogsTool;
#[async_trait]
impl Tool for ReadBackgroundProcessLogsTool {
    fn name(&self) -> &str {
        "read_background_process_logs"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let pid = args
            .get("pid")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pid'"))? as u32;
        tools::system::read_background_process_logs(pid).await
    }
}

pub struct KillBackgroundProcessTool;
#[async_trait]
impl Tool for KillBackgroundProcessTool {
    fn name(&self) -> &str {
        "kill_background_process"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let pid = args
            .get("pid")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pid'"))? as u32;
        tools::system::kill_background_process(pid).await
    }
}

pub struct ListBackgroundProcessesTool;
#[async_trait]
impl Tool for ListBackgroundProcessesTool {
    fn name(&self) -> &str {
        "list_background_processes"
    }
    async fn execute(
        &self,
        _args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        tools::system::list_background_processes().await
    }
}

pub struct CheckPortStatusTool;
#[async_trait]
impl Tool for CheckPortStatusTool {
    fn name(&self) -> &str {
        "check_port_status"
    }
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        _undo: &mut Vec<UndoAction>,
        _cwd: Option<&Path>,
    ) -> Result<String> {
        let port = args
            .get("port")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing 'port'"))? as u16;
        let host = args.get("host").and_then(|v| v.as_str());
        tools::system::check_port_status(port, host).await
    }
}
