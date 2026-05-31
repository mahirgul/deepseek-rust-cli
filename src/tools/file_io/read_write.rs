use anyhow::Result;
use tokio::fs;

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
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).await?;
    }
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
            let line_ending = if content.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            fs::write(p, new_lines.join(line_ending)).await?;
            return Ok("Text replaced successfully (fuzzy match).".to_string());
        }
    }

    Err(anyhow::anyhow!(
        "Could not find a match for the provided text, even with fuzzy matching."
    ))
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

    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let cleaned_content = cleaned_lines.join(line_ending);
    fs::write(p, &cleaned_content).await?;
    Ok("File cleaned up (trailing spaces removed, line endings normalized).".to_string())
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
    sorted_edits.sort_by_key(|b| std::cmp::Reverse(b.start_line));

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
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut new_content = lines.join(line_ending);
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        if line_ending == "\r\n" {
            new_content.push_str("\r\n");
        } else {
            new_content.push('\n');
        }
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
    hunks.sort_by_key(|b| std::cmp::Reverse(b.old_start));

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

        let _actual_end_idx = if end_idx > lines.len() {
            lines.len()
        } else {
            end_idx
        };

        lines.splice(start_idx.._actual_end_idx, hunk.new_lines);
    }

    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut new_content = lines.join(line_ending);
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        if line_ending == "\r\n" {
            new_content.push_str("\r\n");
        } else {
            new_content.push('\n');
        }
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
}
