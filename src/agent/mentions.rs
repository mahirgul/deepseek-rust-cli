use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

static MENTION_RE: OnceLock<Regex> = OnceLock::new();

pub fn process_mentions(text: &str) -> String {
    let mention_re = MENTION_RE.get_or_init(|| Regex::new(r"@([^\n@\s]+)").unwrap());

    let mut result = String::new();
    let mut last_end = 0;

    for cap in mention_re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let path_str = cap[1].trim();
        let path = Path::new(path_str);

        // Push everything between last match and current match
        result.push_str(&text[last_end..full_match.start()]);

        if path.exists() && path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
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
