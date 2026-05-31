use anyhow::Result;
use regex::Regex;
use tokio::fs;
use walkdir::WalkDir;

use crate::tools::base::validate_path;

pub async fn delete_file(path: &str) -> Result<()> {
    let p = validate_path(path)?;
    let meta = fs::metadata(&p).await?;
    if meta.is_dir() {
        fs::remove_dir_all(p).await?;
    } else {
        fs::remove_file(p).await?;
    }
    Ok(())
}

pub async fn rename_file(src: &str, dst: &str) -> Result<()> {
    let s = validate_path(src)?;
    let d = validate_path(dst)?;
    fs::rename(s, d).await?;
    Ok(())
}

pub async fn bulk_rename(path: &str, pattern: &str, replacement: &str) -> Result<String> {
    let p = validate_path(path)?;
    let re = Regex::new(pattern)?;
    let mut count = 0;
    let mut entries = fs::read_dir(p).await?;

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if re.is_match(&name) {
            let new_name = re.replace_all(&name, replacement).to_string();
            let src = entry.path();
            let dst = src.with_file_name(new_name);
            let validated_dst = validate_path(
                dst.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid path Unicode"))?,
            )?;
            fs::rename(src, validated_dst).await?;
            count += 1;
        }
    }
    Ok(format!("Successfully renamed {} files.", count))
}

pub async fn copy_local_file(src: &str, dst: &str) -> Result<()> {
    let s = validate_path(src)?;
    let d = validate_path(dst)?;
    if let Some(parent) = d.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::copy(s, d).await?;
    Ok(())
}

pub async fn copy_directory(src: &str, dst: &str) -> Result<()> {
    let s = validate_path(src)?;
    let d = validate_path(dst)?;

    if !s.is_dir() {
        anyhow::bail!("Source path is not a directory");
    }

    let src_clone = s.clone();
    let dst_clone = d.clone();

    tokio::task::spawn_blocking(move || {
        for entry in WalkDir::new(&src_clone) {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(&src_clone)?;
            let target = dst_clone.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(path, target)?;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

pub async fn create_directory(path: &str) -> Result<()> {
    let p = validate_path(path)?;
    fs::create_dir_all(p).await?;
    Ok(())
}

pub async fn file_exists(path: &str) -> Result<bool> {
    let p = validate_path(path)?;
    Ok(p.exists())
}

pub async fn get_file_info(path: &str) -> Result<String> {
    let p = validate_path(path)?;
    let meta = fs::metadata(&p).await?;

    let file_type = if meta.is_dir() { "Directory" } else { "File" };
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let permissions = format!("{:?}", meta.permissions());

    Ok(format!(
        "Path: {}\nType: {}\nSize: {} bytes\nModified: {}\nPermissions: {}",
        path, file_type, size, modified, permissions
    ))
}

pub async fn split_file(path: &str, pattern: &str, output_prefix: &str) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    let re = Regex::new(pattern)?;
    let mut parts = Vec::new();
    let mut last_idx = 0;

    for mat in re.find_iter(&content) {
        if mat.start() > last_idx {
            parts.push(&content[last_idx..mat.start()]);
        }
        last_idx = mat.start();
    }
    parts.push(&content[last_idx..]);

    let mut count = 0;
    for part in parts {
        if part.trim().is_empty() {
            continue;
        }
        let out_path_raw = format!("{}_{}.txt", output_prefix, count + 1);
        let out_path = validate_path(&out_path_raw)?;
        fs::write(out_path, part).await?;
        count += 1;
    }

    Ok(format!("File split into {} parts.", count))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn tempdir_in_cwd() -> TempDir {
        TempDir::new_in(".").expect("Failed to create temp dir in CWD")
    }

    #[tokio::test]
    async fn test_file_exists() {
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("exists.txt");
        fs::write(&file_path, "exist").unwrap();
        assert!(file_exists(file_path.to_str().unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn test_copy_local_file() {
        let dir = tempdir_in_cwd();
        let src_path = dir.path().join("src.txt");
        let dst_path = dir.path().join("dst.txt");
        fs::write(&src_path, "copy test content").unwrap();

        let src_str = src_path.to_str().unwrap();
        let dst_str = dst_path.to_str().unwrap();
        copy_local_file(src_str, dst_str).await.unwrap();

        let dst_content = fs::read_to_string(&dst_path).unwrap();
        assert_eq!(dst_content, "copy test content");
    }

    #[tokio::test]
    async fn test_copy_directory() {
        let dir = tempdir_in_cwd();
        let src_dir = dir.path().join("src_dir");
        let dst_dir = dir.path().join("dst_dir");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "file a").unwrap();
        fs::write(src_dir.join("b.txt"), "file b").unwrap();

        let src_str = src_dir.to_str().unwrap();
        let dst_str = dst_dir.to_str().unwrap();
        copy_directory(src_str, dst_str).await.unwrap();

        assert!(dst_dir.join("a.txt").exists());
        assert!(dst_dir.join("b.txt").exists());
    }
}
