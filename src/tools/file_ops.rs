use std::fs;

use anyhow::Result;
use md5;
use sha2::{Digest, Sha256};
use tokio::task;
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

    let content1 = tokio::fs::read_to_string(p1).await?;
    let content2 = tokio::fs::read_to_string(p2).await?;

    let mut output = String::new();
    for r in diff::lines(&content1, &content2) {
        match r {
            diff::Result::Left(l) => {
                output.push_str("- ");
                output.push_str(l);
                output.push('\n');
            }
            diff::Result::Both(l, _) => {
                output.push_str("  ");
                output.push_str(l);
                output.push('\n');
            }
            diff::Result::Right(r) => {
                output.push_str("+ ");
                output.push_str(r);
                output.push('\n');
            }
        }
    }
    Ok(output)
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

pub async fn move_code_block(
    src_path: &str,
    dst_path: &str,
    block_pattern: &str,
) -> Result<String> {
    let sp = validate_path(src_path)?;
    let dp = validate_path(dst_path)?;

    let src_content = tokio::fs::read_to_string(&sp).await?;
    let mut dst_content = tokio::fs::read_to_string(&dp).await.unwrap_or_default();

    let re = regex::Regex::new(block_pattern)?;
    if let Some(mat) = re.find(&src_content) {
        let block = mat.as_str().to_string();
        let new_src = src_content.replace(&block, "");

        // Append to destination
        if !dst_content.is_empty() && !dst_content.ends_with('\n') {
            dst_content.push('\n');
        }
        dst_content.push_str(&block);
        dst_content.push('\n');

        tokio::fs::write(sp, new_src).await?;
        tokio::fs::write(dp, dst_content).await?;

        Ok(format!(
            "Moved code block matching '{}' from {} to {}.",
            block_pattern, src_path, dst_path
        ))
    } else {
        Err(anyhow::anyhow!("Code block not found in source file."))
    }
}

pub async fn view_symbol_contents(path: &str, symbol_name: &str) -> Result<String> {
    let p = validate_path(path)?;
    let content = tokio::fs::read_to_string(&p).await?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

    let lines: Vec<&str> = content.lines().collect();
    let mut start_line_idx = None;

    match ext {
        "rs" | "go" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" => {
            let patterns = vec![
                format!(
                    r"(?m)^(?:\s*pub\s+)?(?:async\s+)?fn\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?struct\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?class\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?enum\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?impl(?:\s*<.*>)?\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?type\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(
                    r"(?m)^(?:\s*pub\s+)?trait\s+{}\b",
                    regex::escape(symbol_name)
                ),
            ];

            for pattern in patterns {
                if let Ok(re) = regex::Regex::new(&pattern) {
                    for (idx, line) in lines.iter().enumerate() {
                        if re.is_match(line) {
                            start_line_idx = Some(idx);
                            break;
                        }
                    }
                }
                if start_line_idx.is_some() {
                    break;
                }
            }

            if let Some(start_idx) = start_line_idx {
                let mut brace_count = 0;
                let mut seen_brace = false;
                let mut end_line_idx = start_idx;

                for (idx, line) in lines.iter().enumerate().skip(start_idx) {
                    end_line_idx = idx;
                    for c in line.chars() {
                        if c == '{' {
                            brace_count += 1;
                            seen_brace = true;
                        } else if c == '}' {
                            brace_count -= 1;
                        }
                    }
                    if seen_brace && brace_count <= 0 {
                        break;
                    }
                }

                let result_lines = &lines[start_idx..=end_line_idx];
                return Ok(result_lines.join("\n"));
            }
        }
        "py" => {
            let patterns = vec![
                format!(
                    r"(?m)^\s*(?:async\s+)?def\s+{}\b",
                    regex::escape(symbol_name)
                ),
                format!(r"(?m)^\s*class\s+{}\b", regex::escape(symbol_name)),
            ];

            for pattern in patterns {
                if let Ok(re) = regex::Regex::new(&pattern) {
                    for (idx, line) in lines.iter().enumerate() {
                        if re.is_match(line) {
                            start_line_idx = Some(idx);
                            break;
                        }
                    }
                }
                if start_line_idx.is_some() {
                    break;
                }
            }

            if let Some(start_idx) = start_line_idx {
                let start_line = lines[start_idx];
                let start_indent = start_line.len() - start_line.trim_start().len();
                let mut end_line_idx = start_idx;

                for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let indent = line.len() - line.trim_start().len();
                    if indent <= start_indent {
                        break;
                    }
                    end_line_idx = idx;
                }

                let result_lines = &lines[start_idx..=end_line_idx];
                return Ok(result_lines.join("\n"));
            }
        }
        _ => {}
    }

    Err(anyhow::anyhow!(
        "Symbol '{}' not found in file (or unsupported file extension: {})",
        symbol_name,
        ext
    ))
}
