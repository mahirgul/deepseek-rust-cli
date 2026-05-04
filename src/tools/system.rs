use std::collections::HashMap;

use anyhow::Result;
use tokio::process::Command;

pub async fn execute_shell_command(
    command: &str,
    is_background: bool,
    cwd: Option<&str>,
    env_vars: Option<HashMap<String, String>>,
) -> Result<String> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    };

    if let Some(path) = cwd {
        cmd.current_dir(path);
    }

    if let Some(vars) = env_vars {
        cmd.envs(vars);
    }

    if is_background {
        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(format!("Started background process with PID: {}", pid))
    } else {
        let output = cmd.output().await?;
        Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn get_system_info() -> Result<String> {
    Ok(format!(
        "OS: {}, Arch: {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ))
}
