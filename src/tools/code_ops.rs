use anyhow::Result;
use tokio::process::Command;

pub async fn run_python_code(code: &str) -> Result<String> {
    let mut cmd = Command::new("python3");
    cmd.arg("-c").arg(code);

    let output = match cmd.output().await {
        Ok(out) => out,
        Err(_) => {
            // Fallback to 'python'
            Command::new("python").arg("-c").arg(code).output().await?
        }
    };

    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}
