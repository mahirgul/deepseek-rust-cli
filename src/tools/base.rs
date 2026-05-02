use anyhow::{Result, bail};
use std::path::PathBuf;

pub fn validate_path(path: &str) -> Result<PathBuf> {
    let p = PathBuf::from(path);

    // Convert to absolute path to check for traversal
    let current_dir = std::env::current_dir()?;
    let absolute_path = if p.is_absolute() {
        p.clone()
    } else {
        current_dir.join(&p)
    };

    // Very basic protection: don't allow going above current directory
    // or specific system directories if we wanted to be stricter.
    // For a CLI agent, we might allow home dir but let's at least prevent ../../ etc

    let canonical = match absolute_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If it doesn't exist, we still want to check the path components
            // for ".." to prevent creation of files outside
            if path.contains("..") {
                bail!("Path traversal attempt detected: {}", path);
            }
            absolute_path
        }
    };

    if !canonical.starts_with(&current_dir) && !path.starts_with("./") && !p.is_relative() {
        // If it's absolute and not in current dir, we might want to warn or bail
        // But for now, let's just ensure no ".." trickery
        if path.contains("..") {
            bail!("Path traversal attempt detected: {}", path);
        }
    }

    Ok(p)
}
