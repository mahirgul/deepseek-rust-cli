use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

static DANGEROUS_RE: OnceLock<Vec<Regex>> = OnceLock::new();

pub fn is_dangerous_tool(name: &str, args: &serde_json::Map<String, Value>) -> bool {
    if name == "delete_file" {
        return true;
    }
    if name == "execute_shell_command" {
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
