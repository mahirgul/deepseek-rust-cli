use std::{fs, sync::OnceLock};

use regex::Regex;

static MENTION_RE: OnceLock<Regex> = OnceLock::new();

pub fn process_mentions(text: &str) -> String {
    let mention_re = MENTION_RE.get_or_init(|| Regex::new(r"@([^\n@\s]+)").unwrap());

    let mut result = String::new();
    let mut last_end = 0;

    for cap in mention_re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let path_str = cap[1].trim();

        // Push everything between last match and current match
        result.push_str(&text[last_end..full_match.start()]);

        // Validate path for security before reading
        let validated_path = match crate::tools::base::validate_path(path_str) {
            Ok(p) => p,
            Err(_) => {
                result.push_str(full_match.as_str());
                last_end = full_match.end();
                continue;
            }
        };

        if validated_path.exists() && validated_path.is_file() {
            if let Ok(content) = fs::read_to_string(&validated_path) {
                let trimmed = if content.len() > 50000 {
                    format!("{}... (truncated)", &content[..50000])
                } else {
                    content
                };
                let replacement = format!("@{} (File Content):\n```\n{}\n```", path_str, trimmed);
                result.push_str(&replacement);
            } else {
                result.push_str(full_match.as_str());
            }
        } else {
            result.push_str(full_match.as_str());
        }

        last_end = full_match.end();
    }

    // Push remaining text
    result.push_str(&text[last_end..]);
    result
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_process_mentions_basic() {
        let dir = TempDir::new_in(".").unwrap();
        let file_path = dir.path().join("mention.txt");
        fs::write(&file_path, "file content").unwrap();

        let path_str = file_path.to_str().unwrap();
        let input = format!("check this file @{}", path_str);
        let processed = process_mentions(&input);

        assert!(processed.contains("file content"));
        assert!(processed.contains("check this file"));
    }
}
