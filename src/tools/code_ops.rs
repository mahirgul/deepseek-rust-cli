use anyhow::Result;
use tokio::process::Command;

pub async fn run_python_code(code: &str) -> Result<String> {
    let output = Command::new("python3").arg("-c").arg(code).output().await?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}
