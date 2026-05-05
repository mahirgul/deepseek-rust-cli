use anyhow::Result;
use rayon::prelude::*;
use regex::Regex;
use tokio::fs;
use walkdir::WalkDir;

use crate::tools::base::validate_path;

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
        let window_normalized = window
            .join("\n")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if window_normalized == normalized_old {
            let mut new_lines = lines.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            new_lines.splice(i..i + old_lines.len(), vec![new_text.to_string()]);
            fs::write(p, new_lines.join("\n")).await?;
            return Ok("Text replaced successfully (fuzzy match).".to_string());
        }
    }

    Err(anyhow::anyhow!(
        "Could not find a match for the provided text, even with fuzzy matching."
    ))
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

/// Search files for a text pattern using native Rust (no shell process).
/// Uses literal search by default, with regex support.
/// Returns matching lines with file path and line number.
pub async fn search_files(
    query: &str,
    path: Option<&str>,
    glob_pattern: Option<&str>,
    max_results: usize,
) -> Result<String> {
    let search_path = path.unwrap_or(".");
    let max = max_results.clamp(1, 500);

    // Compile pattern: prefer case-insensitive literal, fall back to raw regex
    let escaped = regex::escape(query);
    let pattern = format!("(?i){}", escaped);
    let re = Regex::new(&pattern)
        .or_else(|_| Regex::new(query))
        .map_err(|e| anyhow::anyhow!("Invalid search pattern: {}", e))?;

    // Collect files matching glob
    let mut results: Vec<String> = Vec::new();
    let walker = WalkDir::new(search_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Always include the root directory (depth 0)
            if e.depth() == 0 {
                return true;
            }
            // Skip hidden directories and common ignore dirs
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            if e.file_type().is_dir() {
                let skip = ["target", "node_modules", "__pycache__", ".git"];
                return !skip.contains(&name.as_ref());
            }
            true
        });

    // Collect candidate files
    let files: Vec<_> = walker
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(glob) = glob_pattern {
                let path_str = e.path().to_string_lossy();
                let filename = e.file_name().to_string_lossy();
                // Simple glob matching: support * and ** patterns
                glob_match(glob, &filename) || glob_match(glob, &path_str)
            } else {
                true
            }
        })
        .collect();

    // Search in parallel using rayon
    let matches: Vec<String> = files
        .par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            let content = std::fs::read_to_string(path).ok()?;
            let mut file_matches = Vec::new();

            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    // Truncate long lines
                    let display = if line.len() > 300 {
                        let truncate_at = line
                            .char_indices()
                            .nth(300)
                            .map(|(i, _)| i)
                            .unwrap_or(line.len());
                        format!("{}...", &line[..truncate_at])
                    } else {
                        line.to_string()
                    };
                    file_matches.push(format!("{}:{}: {}", path.display(), i + 1, display.trim()));
                }
            }

            if file_matches.is_empty() {
                None
            } else {
                Some(file_matches.join("\n"))
            }
        })
        .collect();

    for m in &matches {
        if results.len() >= max {
            break;
        }
        for line in m.lines() {
            if results.len() >= max {
                break;
            }
            results.push(line.to_string());
        }
    }

    if results.is_empty() {
        Ok(format!("No matches found for '{}'.", query))
    } else {
        let total = results.len();
        let truncated = total >= max;
        let mut output = results.join("\n");
        if truncated {
            output.push_str(&format!(
                "\n... (truncated to {} results, {} total matches)",
                max, total
            ));
        }
        Ok(output)
    }
}

/// Simple glob matching: supports * wildcard
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return text.contains(pattern);
    }

    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // Must match at start
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Must match at end
            return text[pos..].ends_with(part);
        } else {
            // Match in middle
            match text[pos..].find(part) {
                Some(idx) => pos += idx + part.len(),
                None => return false,
            }
        }
    }
    true
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
    async fn test_fuzzy_replace_exact() {
        let dir = tempdir_in_cwd();
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
        let dir = tempdir_in_cwd();
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
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\nline 4").unwrap();

        let path_str = file_path.to_str().unwrap();
        let content = read_local_file(path_str, Some(2), Some(3)).await.unwrap();
        assert_eq!(content, "line 2\nline 3");
    }

    #[tokio::test]
    async fn test_search_files_basic() {
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("search_test.rs");
        fs::write(
            &file_path,
            "fn main() {\n    println!(\"hello world\");\n    let x = 42;\n}\n",
        )
        .unwrap();

        let dir_str = dir.path().to_str().unwrap();
        let result = search_files("hello", Some(dir_str), Some("*.rs"), 50)
            .await
            .unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("search_test.rs"));
    }

    #[tokio::test]
    async fn test_search_files_no_match() {
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("empty.rs");
        fs::write(&file_path, "just some text\nnothing here\n").unwrap();

        let dir_str = dir.path().to_str().unwrap();
        let result = search_files("nonexistent", Some(dir_str), None, 50)
            .await
            .unwrap();

        assert!(result.contains("No matches found"));
    }
}
