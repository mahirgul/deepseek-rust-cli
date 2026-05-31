use anyhow::Result;
use rayon::prelude::*;
use regex::Regex;
use tokio::fs;
use walkdir::WalkDir;

use crate::tools::base::validate_path;

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
