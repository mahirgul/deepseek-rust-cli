use crate::models::{AppConfig, Message};
use std::fs;
use std::path::Path;
use termimad::MadSkin;
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use tiktoken_rs::cl100k_base;

pub enum Action {
    Bash(String),
    Fetch(String),
    Read(String, usize, usize),
    Patch(String, String, String), // file, old, new
    None,
}

pub fn is_safe_command(cmd: &str) -> (bool, Option<String>) {
    let dangerous_patterns = vec![
        (r"rm\s+-rf\s+/", "Root dizini silmeye çalışıyor"),
        (r"rm\s+-rf\s+\*", "Tüm dosyaları silmeye çalışıyor"),
        (r"mkfs", "Disk formatlama komutu"),
        (r"> /dev/sd", "Diske doğrudan yazma"),
        (r":\(\)\{ :\|:& \};:", "Fork bomb"),
        (r"dd\s+if=/dev/zero", "Diski sıfırlama"),
    ];

    for (pattern, reason) in dangerous_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(cmd) {
                return (false, Some(reason.to_string()));
            }
        }
    }
    (true, None)
}

pub fn extract_action(response: &str) -> Action {
    if let Some(start) = response.find("```bash\n") {
        let content = &response[start + 8..];
        if let Some(end) = content.find("\n```") {
            return Action::Bash(content[..end].trim().to_string());
        }
    } else if let Some(start) = response.find("<bash>") {
        let content = &response[start + 6..];
        if let Some(end) = content.find("</bash>") {
            return Action::Bash(content[..end].trim().to_string());
        }
    }

    if let Some(start) = response.find("```patch\n") {
        let content = &response[start + 9..];
        if let Some(end) = content.find("\n```") {
            let block = &content[..end];
            let mut file = String::new();
            if let Some(file_line) = block.lines().find(|l| l.starts_with("FILE:")) {
                file = file_line[5..].trim().to_string();
            }

            if let (Some(s_idx), Some(m_idx), Some(e_idx)) = (block.find("<<<<\n"), block.find("====\n"), block.find(">>>>")) {
                let old = &block[s_idx + 5..m_idx].trim_end_matches('\n');
                let new = &block[m_idx + 5..e_idx].trim_end_matches('\n');
                if !file.is_empty() {
                    return Action::Patch(file, old.to_string(), new.to_string());
                }
            }
        }
    }

    if let Some(start) = response.find("```fetch\n") {
        let content = &response[start + 9..];
        if let Some(end) = content.find("\n```") {
            return Action::Fetch(content[..end].trim().to_string());
        }
    }

    let mut read_content = None;
    if let Some(start) = response.find("```read\n") {
        let content = &response[start + 8..];
        if let Some(end) = content.find("\n```") {
            read_content = Some(content[..end].trim());
        }
    }

    if let Some(line) = read_content {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(s), Ok(e)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                return Action::Read(parts[0].to_string(), s, e);
            }
        }
        return Action::Read(parts.get(0).unwrap_or(&"").to_string(), 0, 0);
    }

    Action::None
}

pub fn load_config() -> AppConfig {
    if let Ok(content) = fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
            return config;
        }
    }
    AppConfig::default()
}

pub fn load_history() -> Vec<Message> {
    if let Ok(content) = fs::read_to_string(".deep/history.json") {
        if let Ok(history) = serde_json::from_str::<Vec<Message>>(&content) {
            return history;
        }
    }
    Vec::new()
}

pub fn save_history(messages: &[Message]) {
    if !Path::new(".deep").exists() {
        let _ = fs::create_dir_all(".deep");
    }
    if let Ok(json) = serde_json::to_string_pretty(messages) {
        let _ = fs::write(".deep/history.json", json);
    }
}

pub fn render_markdown(text: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let skin = MadSkin::default();

    // Split text into markdown and code blocks for manual highlighting if needed,
    // but for now let's use a simpler approach: 
    // Termimad for general markdown, and we can try to intercept code blocks.
    
    // Improved approach: Parse code blocks and highlight them specifically.
    let mut current_pos = 0;
    while let Some(start_idx) = text[current_pos..].find("```") {
        let absolute_start = current_pos + start_idx;
        // Print markdown before code block
        skin.print_text(&text[current_pos..absolute_start]);
        
        let block_content = &text[absolute_start + 3..];
        if let Some(end_idx) = block_content.find("```") {
            let line_end = block_content.find('\n').unwrap_or(0);
            let lang = block_content[..line_end].trim();
            let code = &block_content[line_end..end_idx].trim_start_matches('\n');
            
            highlight_code(code, lang, &ps, &ts);
            
            current_pos = absolute_start + 3 + end_idx + 3;
        } else {
            break;
        }
    }
    if current_pos < text.len() {
        skin.print_text(&text[current_pos..]);
    }
}

fn highlight_code(code: &str, lang: &str, ps: &SyntaxSet, ts: &ThemeSet) {
    let syntax = ps.find_syntax_by_token(lang).unwrap_or_else(|| ps.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, ps).unwrap_or_default();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped);
    }
    println!("\x1b[0m"); // Reset colors
}

pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    bpe.encode_with_special_tokens(text).len()
}

pub fn get_total_tokens(messages: &[Message]) -> usize {
    messages.iter().map(|m| count_tokens(&m.content)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bash_action() {
        let response = "Here is the command:\n```bash\nls -la\n```";
        match extract_action(response) {
            Action::Bash(cmd) => assert_eq!(cmd, "ls -la"),
            _ => panic!("Expected Bash action"),
        }
    }

    #[test]
    fn test_extract_patch_action() {
        let response = "```patch\nFILE: test.txt\n<<<<\nold\n====\nnew\n>>>>\n```";
        match extract_action(response) {
            Action::Patch(file, old, new) => {
                assert_eq!(file, "test.txt");
                assert_eq!(old, "old");
                assert_eq!(new, "new");
            }
            _ => panic!("Expected Patch action"),
        }
    }

    #[test]
    fn test_is_safe_command() {
        let (safe, _) = is_safe_command("ls -la");
        assert!(safe);
        let (unsafe_cmd, _) = is_safe_command("rm -rf /");
        assert!(!unsafe_cmd);
    }

    #[test]
    fn test_token_counting() {
        let text = "Hello world";
        assert!(count_tokens(text) > 0);
    }
}
