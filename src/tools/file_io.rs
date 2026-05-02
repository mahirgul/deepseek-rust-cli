use crate::tools::base::validate_path;
use anyhow::Result;
use tokio::fs;

pub async fn read_local_file(
    path: &str,
    start: Option<usize>,
    end: Option<usize>,
) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(p).await?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(String::new());
    }

    let s = start.unwrap_or(1).saturating_sub(1);
    let mut e = end.unwrap_or(lines.len());

    if s >= lines.len() {
        return Ok(String::new());
    }

    if e > lines.len() {
        e = lines.len();
    }
    if e < s {
        e = s;
    }

    Ok(lines[s..e].join("\n"))
}

pub async fn write_local_file(path: &str, content: &str) -> Result<()> {
    let p = validate_path(path)?;
    fs::write(p, content).await?;
    Ok(())
}

pub async fn replace_text_in_file(path: &str, old_text: &str, new_text: &str) -> Result<()> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    let new_content = content.replace(old_text, new_text);
    fs::write(p, new_content).await?;
    Ok(())
}

pub async fn list_directory(path: Option<&str>) -> Result<Vec<String>> {
    let dir_str = path.unwrap_or(".");
    let p = validate_path(dir_str)?;
    let mut entries = fs::read_dir(p).await?;
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if let Ok(name) = entry.file_name().into_string() {
            names.push(name);
        }
    }
    Ok(names)
}

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
