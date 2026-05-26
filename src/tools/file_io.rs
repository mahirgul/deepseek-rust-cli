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
            fs::rename(src, dst).await?;
            count += 1;
        }
    }
    Ok(format!("Successfully renamed {} files.", count))
}

pub async fn cleanup_file(path: &str) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    let mut cleaned_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_end();
        if !trimmed.is_empty() || !cleaned_lines.is_empty() {
            cleaned_lines.push(trimmed);
        }
    }

    // Remove trailing empty lines
    while let Some(last) = cleaned_lines.last() {
        if last.is_empty() {
            cleaned_lines.pop();
        } else {
            break;
        }
    }

    let cleaned_content = cleaned_lines.join("\n");
    fs::write(p, &cleaned_content).await?;
    Ok("File cleaned up (trailing spaces removed, line endings normalized).".to_string())
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
        let out_path = format!("{}_{}.txt", output_prefix, count + 1);
        fs::write(out_path, part).await?;
        count += 1;
    }

    Ok(format!("File split into {} parts.", count))
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineEdit {
    pub start_line: usize,
    pub end_line: usize,
    pub replacement_content: String,
    pub target_content: Option<String>,
}

pub async fn edit_file_by_lines(path: &str, edits: Vec<LineEdit>) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Sort edits by start_line in descending order so that changing content size doesn't shift the
    // indices of subsequent edits
    let mut sorted_edits = edits;
    sorted_edits.sort_by(|a, b| b.start_line.cmp(&a.start_line));

    // Verify no overlapping edits
    for i in 0..sorted_edits.len().saturating_sub(1) {
        if sorted_edits[i + 1].end_line >= sorted_edits[i].start_line {
            anyhow::bail!(
                "Overlapping edits detected: edit at {}-{} overlaps with edit at {}-{}",
                sorted_edits[i + 1].start_line,
                sorted_edits[i + 1].end_line,
                sorted_edits[i].start_line,
                sorted_edits[i].end_line
            );
        }
    }

    for edit in sorted_edits {
        if edit.start_line == 0 {
            anyhow::bail!("Line numbers are 1-indexed; start_line cannot be 0");
        }
        if edit.end_line < edit.start_line {
            anyhow::bail!(
                "end_line ({}) cannot be less than start_line ({})",
                edit.end_line,
                edit.start_line
            );
        }

        let start_idx = edit.start_line - 1;
        let end_idx = edit.end_line - 1; // inclusive

        // Handle completely empty file insertion
        if lines.is_empty() && edit.start_line == 1 {
            let replacement_lines: Vec<String> = edit
                .replacement_content
                .lines()
                .map(|s| s.to_string())
                .collect();
            lines = replacement_lines;
            continue;
        }

        // Handle appending past the last line
        if start_idx == lines.len() {
            let replacement_lines: Vec<String> = edit
                .replacement_content
                .lines()
                .map(|s| s.to_string())
                .collect();
            lines.extend(replacement_lines);
            continue;
        }

        if start_idx >= lines.len() {
            anyhow::bail!(
                "start_line ({}) is out of bounds (file has {} lines)",
                edit.start_line,
                lines.len()
            );
        }

        let actual_end_idx = if end_idx >= lines.len() {
            lines.len() - 1
        } else {
            end_idx
        };

        if let Some(target) = &edit.target_content {
            // Retrieve current lines
            let current_chunk = lines[start_idx..=actual_end_idx].join("\n");

            // Fuzzy compare target content and current chunk
            let norm_target: String = target.split_whitespace().collect::<Vec<_>>().join(" ");
            let norm_current: String = current_chunk
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            if norm_target != norm_current {
                anyhow::bail!(
                    "Target content verification failed at lines {}-{}.\nExpected (normalized): \
                     {}\nFound (normalized): {}",
                    edit.start_line,
                    edit.end_line,
                    norm_target,
                    norm_current
                );
            }
        }

        // Replacement lines
        let replacement_lines: Vec<String> = edit
            .replacement_content
            .lines()
            .map(|s| s.to_string())
            .collect();

        // Splice replacement content
        lines.splice(start_idx..=actual_end_idx, replacement_lines);
    }

    // Maintain ending newline if present originally
    let mut new_content = lines.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(&p, new_content).await?;
    Ok("File successfully edited by lines.".to_string())
}

pub async fn apply_diff_patch(path: &str, patch_content: &str) -> Result<String> {
    let p = validate_path(path)?;
    let content = fs::read_to_string(&p).await?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut patch_lines = patch_content.lines().peekable();

    // Skip headers until we see a hunk starting with @@
    while let Some(&line) = patch_lines.peek() {
        if line.starts_with("@@") {
            break;
        }
        patch_lines.next();
    }

    struct Hunk {
        old_start: usize,
        old_count: usize,
        new_lines: Vec<String>,
    }

    let mut hunks = Vec::new();

    while let Some(line) = patch_lines.next() {
        if line.starts_with("@@") {
            // Parse @@ -old_start,old_count +new_start,new_count @@
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                anyhow::bail!("Invalid hunk header: {}", line);
            }
            let old_part = parts[1].trim_start_matches('-');
            let old_subparts: Vec<&str> = old_part.split(',').collect();
            let old_start: usize = old_subparts[0].parse()?;
            let old_count: usize = if old_subparts.len() > 1 {
                old_subparts[1].parse()?
            } else {
                1
            };

            let mut new_lines = Vec::new();

            while let Some(&p_line) = patch_lines.peek() {
                if p_line.starts_with("@@") {
                    break;
                }
                let p_line = patch_lines.next().unwrap();
                if let Some(stripped) = p_line.strip_prefix('+') {
                    new_lines.push(stripped.to_string());
                } else if p_line.starts_with('-') {
                    // Deleted line, skip
                } else if p_line.starts_with(' ') || p_line.is_empty() {
                    let content_line = if let Some(stripped) = p_line.strip_prefix(' ') {
                        stripped
                    } else {
                        p_line
                    };
                    new_lines.push(content_line.to_string());
                } else if p_line.starts_with('\\') {
                    // Skip \ No newline at end of file
                } else {
                    new_lines.push(p_line.to_string());
                }
            }

            hunks.push(Hunk {
                old_start,
                old_count,
                new_lines,
            });
        }
    }

    // Sort hunks by old_start in descending order to avoid line shifting problems
    hunks.sort_by(|a, b| b.old_start.cmp(&a.old_start));

    for hunk in hunks {
        if hunk.old_start == 0 {
            anyhow::bail!("Hunk start line cannot be 0");
        }
        let start_idx = hunk.old_start - 1;
        let end_idx = start_idx + hunk.old_count;

        if start_idx > lines.len() {
            anyhow::bail!(
                "Hunk start line ({}) is out of bounds (file has {} lines)",
                hunk.old_start,
                lines.len()
            );
        }

        let actual_end_idx = if end_idx > lines.len() {
            lines.len()
        } else {
            end_idx
        };

        lines.splice(start_idx..actual_end_idx, hunk.new_lines);
    }

    let mut new_content = lines.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(&p, new_content).await?;
    Ok("Patch successfully applied.".to_string())
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
        assert_eq!(fs::read_to_string(dst_dir.join("a.txt")).unwrap(), "file a");
    }

    #[tokio::test]
    async fn test_create_directory() {
        let dir = tempdir_in_cwd();
        let new_dir = dir.path().join("nested/folder/here");
        let path_str = new_dir.to_str().unwrap();
        create_directory(path_str).await.unwrap();

        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[tokio::test]
    async fn test_file_exists() {
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("exists.txt");
        let path_str = file_path.to_str().unwrap();
        assert!(!file_exists(path_str).await.unwrap());

        fs::write(&file_path, "").unwrap();
        assert!(file_exists(path_str).await.unwrap());
    }

    #[tokio::test]
    async fn test_get_file_info() {
        let dir = tempdir_in_cwd();
        let file_path = dir.path().join("info.txt");
        fs::write(&file_path, "hello info").unwrap();
        let path_str = file_path.to_str().unwrap();

        let info = get_file_info(path_str).await.unwrap();
        assert!(info.contains("Path:"));
        assert!(info.contains("Type: File"));
        assert!(info.contains("Size: 10 bytes"));
    }

    #[tokio::test]
    async fn test_native_diff_files() {
        let dir = tempdir_in_cwd();
        let f1 = dir.path().join("f1.txt");
        let f2 = dir.path().join("f2.txt");
        fs::write(&f1, "line 1\nline 2\n").unwrap();
        fs::write(&f2, "line 1\nline 2 mod\n").unwrap();

        let f1_str = f1.to_str().unwrap();
        let f2_str = f2.to_str().unwrap();
        let diff = crate::tools::file_ops::diff_files(f1_str, f2_str)
            .await
            .unwrap();

        assert!(diff.contains("- line 2"));
        assert!(diff.contains("+ line 2 mod"));
    }
}
