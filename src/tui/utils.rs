use crate::tui::colorizer::CodeLang;

pub fn truncate_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if s.chars().count() > max_width {
        let truncated: String = s.chars().take(max_width.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        s.to_string()
    }
}

static ANSI_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn ansi_re() -> &'static regex::Regex {
    ANSI_RE.get_or_init(|| regex::Regex::new("\x1b\\[[0-9;]*m").expect("valid ansi regex"))
}

pub fn truncate_ansi_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let re = ansi_re();
    let mut visible = 0usize;
    let mut result = String::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(m) = re.find(remaining) {
            if m.start() == 0 {
                result.push_str(m.as_str());
                remaining = &remaining[m.end()..];
                continue;
            }
            // Text before the escape
            let text = &remaining[..m.start()];
            for ch in text.chars() {
                if visible >= max_width {
                    result.push('…');
                    return result;
                }
                result.push(ch);
                visible += 1;
            }
            remaining = &remaining[m.start()..];
        } else {
            // No more escapes
            for ch in remaining.chars() {
                if visible >= max_width {
                    result.push('…');
                    return result;
                }
                result.push(ch);
                visible += 1;
            }
            break;
        }
    }
    result
}

pub fn strip_ansi(s: &str) -> String {
    let re = ansi_re();
    re.replace_all(s, "").to_string()
}

pub fn format_tool_args(name: &str, args: &str) -> String {
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(args) {
        match name {
            "execute_shell_command" => {
                if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
                    return format!("  $ {}", cmd);
                }
            }
            "read_local_file" | "write_local_file" | "delete_file" | "cleanup_file"
            | "file_exists" | "get_file_info" => {
                if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    return format!("  📄 {}", path);
                }
            }
            "create_directory" => {
                if let Some(path) = obj.get("directory_path").and_then(|v| v.as_str()) {
                    return format!("  📂 {}", path);
                }
            }
            "replace_text_in_file" | "regex_replace_in_file" => {
                if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    return format!("  📄 {} (replace)", path);
                }
            }
            "move_code_block" | "copy_file" | "copy_directory" => {
                if let (Some(src), Some(dst)) = (
                    obj.get("source_path").and_then(|v| v.as_str()),
                    obj.get("destination_path").and_then(|v| v.as_str()),
                ) {
                    return format!("  📦 {} ➔ {}", src, dst);
                }
            }
            "split_file" => {
                if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    return format!("  ✂️ split: {}", path);
                }
            }
            "bulk_rename" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    return format!("  🏷️ bulk rename in: {}", path);
                }
            }
            "project_wide_replace" => {
                if let (Some(old), Some(new)) = (
                    obj.get("old_text").and_then(|v| v.as_str()),
                    obj.get("new_text").and_then(|v| v.as_str()),
                ) {
                    return format!("  🔍 \"{}\" ➔ \"{}\"", old, new);
                }
            }
            "project_checkpoint" => {
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    return format!("  💾 checkpoint: {}", name);
                }
            }
            "restore_checkpoint" => {
                if let Some(name) = obj.get("checkpoint_file").and_then(|v| v.as_str()) {
                    return format!("  ⏪ restore: {}", name);
                }
            }
            "summarize_project" => return "  📊 summarizing project...".to_string(),
            "list_todo_tasks" => return "  📝 listing tasks...".to_string(),
            "list_directory" | "tree_view" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    return format!("  📂 {}", path);
                }
            }
            "fetch_url" => {
                if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                    return format!("  🌐 {}", url);
                }
            }
            "diff_files" => {
                if let (Some(f1), Some(f2)) = (
                    obj.get("file1").and_then(|v| v.as_str()),
                    obj.get("file2").and_then(|v| v.as_str()),
                ) {
                    return format!("  📄 {} ↔ {}", f1, f2);
                }
            }
            "search_code" | "search_repos" => {
                if let Some(q) = obj.get("query").and_then(|v| v.as_str()) {
                    return format!("  🔍 {}", q);
                }
            }
            _ => {}
        }
        serde_json::to_string_pretty(&obj).unwrap_or_else(|_| args.to_string())
    } else {
        args.to_string()
    }
}

pub fn detect_lang_for_result(tool_name: &str, result: &str) -> CodeLang {
    match tool_name {
        "read_local_file" | "write_local_file" | "replace_text_in_file" => {
            if result.trim_start().starts_with("#!/") {
                if result.contains("python") {
                    return CodeLang::Python;
                }
                if result.contains("bash") || result.contains("sh") {
                    return CodeLang::Shell;
                }
            }
            if result.trim_start().starts_with("<?xml")
                || result.trim_start().starts_with("<!DOCTYPE html")
                || result.trim_start().starts_with("<html")
            {
                return CodeLang::Html;
            }
            if result.trim_start().starts_with("{") || result.trim_start().starts_with("[") {
                return CodeLang::Json;
            }
            if result.contains("fn ") && result.contains("->") {
                return CodeLang::Rust;
            }
            if result.contains("def ") && result.contains("return ") {
                return CodeLang::Python;
            }
            CodeLang::Generic
        }
        "execute_shell_command" => CodeLang::Shell,
        "run_python_code" => CodeLang::Python,
        "github_get_file" => CodeLang::Generic,
        _ => CodeLang::Generic,
    }
}
