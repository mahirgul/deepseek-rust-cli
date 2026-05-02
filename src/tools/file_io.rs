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

pub async fn fuzzy_replace_in_file(path: &str, old_text: &str, new_text: &str) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    
    // Try exact first
    if content.contains(old_text) {
        let new_content = content.replace(old_text, new_text);
        fs::write(p, new_content).await?;
        return Ok("Text replaced successfully (exact match).".to_string());
    }

    // Try normalized whitespace
    let normalized_old = old_text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lines: Vec<&str> = content.lines().collect();
    
    // Simple block matching
    let old_lines: Vec<&str> = old_text.lines().collect();
    if old_lines.is_empty() {
        return Err(anyhow::anyhow!("Old text is empty"));
    }

    // Find a sequence of lines that match (after normalization)
    for i in 0..=lines.len().saturating_sub(old_lines.len()) {
        let window = &lines[i..i + old_lines.len()];
        let window_normalized = window.join("\n").split_whitespace().collect::<Vec<_>>().join(" ");
        
        if window_normalized == normalized_old {
            let mut new_lines = lines.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            new_lines.splice(i..i + old_lines.len(), vec![new_text.to_string()]);
            fs::write(p, new_lines.join("\n")).await?;
            return Ok("Text replaced successfully (fuzzy match).".to_string());
        }
    }

    Err(anyhow::anyhow!("Could not find a match for the provided text, even with fuzzy matching."))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_fuzzy_replace_exact() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello world\nthis is a test").unwrap();
        
        let path_str = file_path.to_str().unwrap();
        let res = fuzzy_replace_in_file(path_str, "hello world", "bye world").await;
        
        assert!(res.is_ok());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "bye world\nthis is a test");
    }

    #[tokio::test]
    async fn test_fuzzy_replace_whitespace() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello   world\nthis is a test").unwrap();
        
        let path_str = file_path.to_str().unwrap();
        // Matching with different whitespace
        let res = fuzzy_replace_in_file(path_str, "hello world", "bye world").await;
        
        assert!(res.is_ok());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "bye world\nthis is a test");
    }

    #[tokio::test]
    async fn test_read_local_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\nline 4").unwrap();
        
        let path_str = file_path.to_str().unwrap();
        let content = read_local_file(path_str, Some(2), Some(3)).await.unwrap();
        assert_eq!(content, "line 2\nline 3");
    }
}
