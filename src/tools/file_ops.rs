use std::fs;

use anyhow::Result;
use md5;
use sha2::{Digest, Sha256};
use tokio::{process::Command, task};
use walkdir::WalkDir;

use crate::tools::base::validate_path;

pub async fn tree_view(path: Option<String>, max_depth: Option<usize>) -> Result<String> {
    task::spawn_blocking(move || {
        let dir = path.unwrap_or_else(|| ".".to_string());
        let _ = validate_path(&dir)?;
        let depth = max_depth.unwrap_or(3);
        let mut output = String::new();
        for entry in WalkDir::new(dir).max_depth(depth).into_iter().flatten() {
            let indent = "  ".repeat(entry.depth());
            output.push_str(&format!(
                "{}{}\n",
                indent,
                entry.file_name().to_string_lossy()
            ));
        }
        Ok(output)
    })
    .await?
}

pub async fn diff_files(file1: &str, file2: &str) -> Result<String> {
    let p1 = validate_path(file1)?;
    let p2 = validate_path(file2)?;
    let output = Command::new("diff")
        .arg("-u")
        .arg(p1)
        .arg(p2)
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn hash_file(path: String, algorithm: Option<String>) -> Result<String> {
    let p = validate_path(&path)?;
    task::spawn_blocking(move || {
        let content = fs::read(p)?;
        let alg = algorithm.unwrap_or_else(|| "sha256".to_string());
        if alg == "md5" {
            Ok(format!("{:x}", md5::compute(content)))
        } else {
            let mut hasher = Sha256::new();
            hasher.update(content);
            Ok(format!("{:x}", hasher.finalize()))
        }
    })
    .await?
}

pub async fn count_lines(path: String) -> Result<String> {
    let p = validate_path(&path)?;
    task::spawn_blocking(move || {
        let content = fs::read_to_string(p)?;
        let lines = content.lines().count();
        let words = content.split_whitespace().count();
        let chars = content.len();
        Ok(format!(
            "Lines: {}, Words: {}, Chars: {}",
            lines, words, chars
        ))
    })
    .await?
}
