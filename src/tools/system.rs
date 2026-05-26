use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::Mutex;

use anyhow::Result;

pub struct ProcessInfo {
    pub cmd: String,
    pub pid: u32,
    pub logs: Arc<Mutex<String>>,
    pub child: Arc<Mutex<tokio::process::Child>>,
}

pub static BACKGROUND_PROCESSES: once_cell::sync::Lazy<Mutex<HashMap<u32, ProcessInfo>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn start_background_process(
    command: &str,
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

    // Pipe stdout and stderr so we can read them
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);

    let stdout_opt = child.stdout.take();
    let stderr_opt = child.stderr.take();

    let logs = Arc::new(Mutex::new(String::new()));
    let child_arc = Arc::new(Mutex::new(child));

    // Spawn task to read stdout continuously and write to logs
    let logs_clone1 = logs.clone();
    if let Some(mut stdout) = stdout_opt {
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = stdout.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&buf[..n]);
                let mut guard = logs_clone1.lock().await;
                guard.push_str(&text);
            }
        });
    }

    // Spawn task to read stderr continuously and write to logs
    let logs_clone2 = logs.clone();
    if let Some(mut stderr) = stderr_opt {
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = stderr.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&buf[..n]);
                let mut guard = logs_clone2.lock().await;
                guard.push_str(&text);
            }
        });
    }

    let mut processes = BACKGROUND_PROCESSES.lock().await;
    processes.insert(
        pid,
        ProcessInfo {
            cmd: command.to_string(),
            pid,
            logs,
            child: child_arc,
        },
    );

    Ok(format!(
        "Started background process with PID: {} for command: '{}'",
        pid, command
    ))
}

pub async fn read_background_process_logs(pid: u32) -> Result<String> {
    let processes = BACKGROUND_PROCESSES.lock().await;
    if let Some(proc_info) = processes.get(&pid) {
        let logs_guard = proc_info.logs.lock().await;
        Ok(logs_guard.clone())
    } else {
        Err(anyhow::anyhow!(
            "No background process found with PID: {}",
            pid
        ))
    }
}

pub async fn kill_background_process(pid: u32) -> Result<String> {
    let mut processes = BACKGROUND_PROCESSES.lock().await;
    if let Some(proc_info) = processes.remove(&pid) {
        let mut child_guard = proc_info.child.lock().await;
        child_guard.kill().await?;
        Ok(format!(
            "Successfully terminated background process with PID: {}",
            pid
        ))
    } else {
        Err(anyhow::anyhow!(
            "No background process found with PID: {}",
            pid
        ))
    }
}

pub async fn list_background_processes() -> Result<String> {
    let mut processes = BACKGROUND_PROCESSES.lock().await;

    // Clean up completed processes and build output
    let mut to_remove = Vec::new();
    let mut output = String::new();

    for (&pid, proc_info) in processes.iter() {
        let mut child_guard = proc_info.child.lock().await;
        match child_guard.try_wait() {
            Ok(Some(_status)) => {
                // Process has exited
                to_remove.push(pid);
            }
            Ok(None) => {
                // Process is still running
                output.push_str(&format!(
                    "PID: {} | Command: '{}' | Status: Running\n",
                    pid, proc_info.cmd
                ));
            }
            Err(_) => {
                // Error querying process, assume dead
                to_remove.push(pid);
            }
        }
    }

    for pid in to_remove {
        processes.remove(&pid);
    }

    if output.is_empty() {
        Ok("No active background processes.".to_string())
    } else {
        Ok(output.trim_end().to_string())
    }
}

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

pub async fn check_port_status(port: u16, host: Option<&str>) -> Result<String> {
    let h = host.unwrap_or("127.0.0.1");
    let addr = format!("{}:{}", h, port);

    // Try to connect to see if something is listening
    let connect_res = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        tokio::net::TcpStream::connect(&addr),
    )
    .await;

    match connect_res {
        Ok(Ok(_stream)) => Ok(format!(
            "Port {} is OCCUPIED (something is listening on it).",
            port
        )),
        _ => {
            // Try to bind to see if it is available for us to listen on, or blocked by system
            match tokio::net::TcpListener::bind(&addr).await {
                Ok(_) => Ok(format!("Port {} is FREE (available for use).", port)),
                Err(e) => Ok(format!("Port {} is BLOCKED or unavailable: {}", port, e)),
            }
        }
    }
}
