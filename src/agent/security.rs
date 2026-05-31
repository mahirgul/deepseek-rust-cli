use std::{collections::HashSet, sync::OnceLock};

use regex::Regex;
use serde_json::Value;

static DANGEROUS_RE: OnceLock<Vec<Regex>> = OnceLock::new();

pub fn is_dangerous_tool(name: &str, args: &serde_json::Map<String, Value>) -> bool {
    if name == "delete_file" {
        return true;
    }
    if name == "execute_shell_command" || name == "start_background_process" {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let regexes = DANGEROUS_RE.get_or_init(|| {
            let patterns = [
                r"(\b|[`$])rm(\b|[`$])",
                r"(\b|[`$])del(\b|[`$])",
                r"(\b|[`$])rd(\b|[`$])",
                r"(\b|[`$])rmdir(\b|[`$])",
                r"(\b|[`$])erase(\b|[`$])",
                r"(\b|[`$])dd\b.*\bof=",
                r"(\b|[`$])mkfs(\b|[`$])",
                r">\s*/dev/",
                r"(\b|[`$])chown\b.*-R\b",
                r"(\b|[`$])chmod\b.*777\b",
                r"(\b|[`$])shred(\b|[`$])",
            ];
            patterns.iter().map(|p| Regex::new(p).unwrap()).collect()
        });

        for re in regexes {
            if re.is_match(&cmd) {
                return true;
            }
        }
    }
    false
}

pub fn get_approval_required_tools() -> HashSet<String> {
    [
        "execute_shell_command",
        "start_background_process",
        "write_local_file",
        "replace_text_in_file",
        "delete_file",
        "rename_file",
        "run_python_code",
        "fetch_url",
        "get_env_var",
        // Git operations that modify state
        "git_commit",
        "git_push",
        "git_checkout",
        "git_clone",
        "git_stash",
        // GitHub API write operations
        "github_issue_create",
        "github_issue_update",
        "github_pr_create",
        "github_pr_merge",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn is_path_traversal_arg(args: &serde_json::Map<String, Value>) -> bool {
    let path_keys = [
        "file_path",
        "path",
        "source_path",
        "destination_path",
        "output_path",
        "file1",
        "file2",
        "dest",
        "files",
        "dir",
    ];
    for key in path_keys {
        if let Some(val) = args.get(key).and_then(|v| v.as_str()) {
            if is_traversal_path(val) {
                return true;
            }
            for part in val.split([',', ';']) {
                let part_trimmed = part.trim();
                if !part_trimmed.is_empty() && is_traversal_path(part_trimmed) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek() {
        let buf = std::path::PathBuf::from(c.as_os_str());
        components.next();
        buf
    } else {
        std::path::PathBuf::new()
    };

    let mut normalized = Vec::new();
    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(Component::RootDir.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = normalized.last() {
                    normalized.pop();
                } else if ret.as_os_str().is_empty() || ret == std::path::Path::new("/") {
                    normalized.push(Component::ParentDir);
                }
            }
            Component::Normal(c) => {
                normalized.push(Component::Normal(c));
            }
        }
    }
    for component in normalized {
        ret.push(component.as_os_str());
    }
    ret
}

fn canonicalize_any(path: &std::path::Path) -> std::path::PathBuf {
    match std::fs::canonicalize(path) {
        Ok(c) => c,
        Err(_) => {
            let mut ancestor = path;
            let mut components = Vec::new();
            while let Some(parent) = ancestor.parent() {
                if let Some(file_name) = ancestor.file_name() {
                    components.push(file_name);
                }
                if parent.exists() {
                    if let Ok(can_parent) = std::fs::canonicalize(parent) {
                        let mut result = can_parent;
                        for comp in components.iter().rev() {
                            result.push(comp);
                        }
                        return result;
                    }
                    break;
                }
                ancestor = parent;
            }
            path.to_path_buf()
        }
    }
}

fn is_traversal_path(path_str: &str) -> bool {
    let p = std::path::PathBuf::from(path_str);
    let abs = if p.is_absolute() {
        p
    } else {
        match std::env::current_dir() {
            Ok(mut a) => {
                a.push(p);
                a
            }
            Err(_) => return false,
        }
    };
    let normalized = normalize_path(&abs);
    let canonical = canonicalize_any(&normalized);

    let root = crate::tools::base::STARTUP_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| {
            std::env::current_dir()
                .and_then(std::fs::canonicalize)
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
    if !canonical.starts_with(&root) && !path_str.is_empty() {
        return true;
    }
    false
}
