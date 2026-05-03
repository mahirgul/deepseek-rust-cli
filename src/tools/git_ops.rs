use anyhow::Result;
use tokio::process::Command;

// ─── Local Git Operations ───────────────────────────────────────────

pub async fn git_status(path: Option<&str>) -> Result<String> {
    let p = path.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("status")
        .arg("-s")
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let out = String::from_utf8_lossy(&output.stdout).to_string();
    if out.is_empty() {
        Ok("Working tree clean.".to_string())
    } else {
        Ok(out)
    }
}

pub async fn git_diff(path: Option<&str>, staged: bool) -> Result<String> {
    let p = path.unwrap_or(".");
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(p).arg("diff");
    if staged {
        cmd.arg("--staged");
    }
    let output = cmd.output().await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let out = String::from_utf8_lossy(&output.stdout).to_string();
    if out.is_empty() {
        Ok("No changes.".to_string())
    } else {
        Ok(out)
    }
}

pub async fn git_log(path: Option<&str>, count: Option<usize>) -> Result<String> {
    let p = path.unwrap_or(".");
    let n = count.unwrap_or(10);
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("log")
        .arg(format!("-n{}", n))
        .arg("--oneline")
        .arg("--decorate")
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn git_branch(
    path: Option<&str>,
    action: Option<&str>,
    name: Option<&str>,
) -> Result<String> {
    let p = path.unwrap_or(".");
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(p);

    match action {
        Some("create") => {
            if let Some(n) = name {
                cmd.arg("branch").arg(n);
                let output = cmd.output().await?;
                if !output.status.success() {
                    return Ok(format!(
                        "Git error: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                return Ok(format!("Branch '{}' created.", n));
            }
        }
        Some("delete") => {
            if let Some(n) = name {
                cmd.arg("branch").arg("-d").arg(n);
                let output = cmd.output().await?;
                if !output.status.success() {
                    return Ok(format!(
                        "Git error: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                return Ok(format!("Branch '{}' deleted.", n));
            }
        }
        Some("switch") => {
            if let Some(n) = name {
                cmd.arg("checkout").arg(n);
                let output = cmd.output().await?;
                if !output.status.success() {
                    return Ok(format!(
                        "Git error: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                return Ok(format!("Switched to branch '{}'.", n));
            }
        }
        _ => {
            // List branches (default)
            cmd.arg("branch").arg("--list");
            let output = cmd.output().await?;
            if !output.status.success() {
                return Ok(format!(
                    "Git error: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    Ok("No action specified.".to_string())
}

pub async fn git_add(path: Option<&str>, files: Option<&str>) -> Result<String> {
    let p = path.unwrap_or(".");
    let targets = files.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("add")
        .arg(targets)
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(format!("Staged: {}", targets))
}

pub async fn git_commit(path: Option<&str>, message: &str) -> Result<String> {
    let p = path.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("commit")
        .arg("-m")
        .arg(message)
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(if out.is_empty() {
        "Committed successfully.".to_string()
    } else {
        out
    })
}

pub async fn git_push(
    path: Option<&str>,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<String> {
    let p = path.unwrap_or(".");
    let r = remote.unwrap_or("origin");
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(p).arg("push").arg(r);
    if let Some(b) = branch {
        cmd.arg(b);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}{}", out, err))
}

pub async fn git_pull(
    path: Option<&str>,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<String> {
    let p = path.unwrap_or(".");
    let r = remote.unwrap_or("origin");
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(p).arg("pull").arg(r);
    if let Some(b) = branch {
        cmd.arg(b);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}{}", out, err))
}

pub async fn git_checkout(path: Option<&str>, target: &str) -> Result<String> {
    let p = path.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("checkout")
        .arg(target)
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(format!("Checked out: {}", target))
}

pub async fn git_clone(url: &str, dest: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg(url);
    if let Some(d) = dest {
        cmd.arg(d);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}{}", out, err))
}

pub async fn git_remote_list(path: Option<&str>) -> Result<String> {
    let p = path.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(p)
        .arg("remote")
        .arg("-v")
        .output()
        .await?;

    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn git_stash(path: Option<&str>, action: Option<&str>) -> Result<String> {
    let p = path.unwrap_or(".");
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(p).arg("stash");

    match action {
        Some("pop") => {
            cmd.arg("pop");
        }
        Some("list") => {
            cmd.arg("list");
        }
        _ => {} // default: stash (save)
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        return Ok(format!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
