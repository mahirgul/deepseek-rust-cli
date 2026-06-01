use super::create_tool;
use crate::api::types::Tool;
use serde_json::json;

pub fn add_web_schemas(tools: &mut Vec<Tool>) {
    tools.push(create_tool(
        "run_python_code",
        "Execute Python code snippet.",
        json!({
            "code": { "type": "string" }
        }),
        vec!["code"],
    ));
    tools.push(create_tool(
        "fetch_url",
        "Fetch and clean content from a URL.",
        json!({
            "url": { "type": "string" }
        }),
        vec!["url"],
    ));
    tools.push(create_tool(
        "get_env_var",
        "Read an environment variable.",
        json!({
            "name": { "type": "string" }
        }),
        vec!["name"],
    ));
    tools.push(create_tool(
        "regex_replace_in_file",
        "Replace text in a file using a regular expression.",
        json!({
            "file_path": { "type": "string" },
            "regex": { "type": "string" },
            "replacement": { "type": "string" }
        }),
        vec!["file_path", "regex", "replacement"],
    ));
    tools.push(create_tool(
        "json_update_value",
        "Read a JSON file, update a value at a specified key path (e.g. \
         'dependencies.tokio.version'), and save it.",
        json!({
            "file_path": { "type": "string" },
            "key_path": { "type": "string" },
            "new_value": { "type": "string" }
        }),
        vec!["file_path", "key_path", "new_value"],
    ));
    tools.push(create_tool(
        "edit_file_by_lines",
        "Edit a file by specifying one or more non-overlapping line ranges. This is highly \
         efficient for modifying code without rewriting the whole file.",
        json!({
            "file_path": {
                "type": "string",
                "description": "Path to the file to edit"
            },
            "edits": {
                "type": "array",
                "description": "List of line-based replacement edits",
                "items": {
                    "type": "object",
                    "properties": {
                        "start_line": {
                            "type": "integer",
                            "description": "The 1-indexed start line number of the range to replace (inclusive)"
                        },
                        "end_line": {
                            "type": "integer",
                            "description": "The 1-indexed end line number of the range to replace (inclusive)"
                        },
                        "replacement_content": {
                            "type": "string",
                            "description": "The new content to insert in place of the target line range"
                        },
                        "target_content": {
                            "type": "string",
                            "description": "Optional: The exact content of the lines being replaced. If provided, it will be verified against the file contents to ensure correctness before applying the edit."
                        }
                    },
                    "required": ["start_line", "end_line", "replacement_content"]
                }
            }
        }),
        vec!["file_path", "edits"],
    ));
    tools.push(create_tool(
        "apply_diff_patch",
        "Apply a unified diff patch to a local file. Ideal for complex or multi-hunk code modifications.",
        json!({
            "file_path": { "type": "string", "description": "Path to the file to patch" },
            "patch_content": { "type": "string", "description": "The unified diff content (including hunk headers @@)" }
        }),
        vec!["file_path", "patch_content"],
    ));
    tools.push(create_tool(
        "list_symbols",
        "Parse code symbols (functions, structs, classes, etc.) from a file using lightweight \
         regex.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "view_symbol_contents",
        "View the full implementation code of a specific symbol (function, class, struct, enum, or impl) from a file.",
        json!({
            "file_path": { "type": "string", "description": "Path to the file to inspect" },
            "symbol_name": { "type": "string", "description": "The name of the symbol/definition to view" }
        }),
        vec!["file_path", "symbol_name"],
    ));
    tools.push(create_tool(
        "screenshot_webapp",
        "Take a screenshot of a local web app or website using Microsoft Edge or Google Chrome in \
         headless mode.",
        json!({
            "url": { "type": "string" },
            "output_path": { "type": "string" }
        }),
        vec!["url", "output_path"],
    ));
    tools.push(create_tool(
        "web_search_duckduckgo",
        "Perform an internet search query via DuckDuckGo and return top results.",
        json!({
            "query": { "type": "string" }
        }),
        vec!["query"],
    ));
    tools.push(create_tool(
        "move_code_block",
        "Move a code block (function, struct, etc.) from one file to another using regex.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" },
            "block_pattern": { "type": "string" }
        }),
        vec!["source_path", "destination_path", "block_pattern"],
    ));
    tools.push(create_tool(
        "split_file",
        "Split a file into multiple parts based on a regex pattern.",
        json!({
            "file_path": { "type": "string" },
            "split_pattern": { "type": "string" },
            "output_prefix": { "type": "string" }
        }),
        vec!["file_path", "split_pattern", "output_prefix"],
    ));
    tools.push(create_tool(
        "cleanup_file",
        "Clean up a file by removing trailing spaces and normalizing line endings.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "summarize_project",
        "Analyze the current project and provide a high-level summary of files, languages, and \
         structure.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "list_todo_tasks",
        "Search the project for TODO, FIXME, HACK, and BUG comments and list them with file and \
         line info.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "project_checkpoint",
        "Create a project-wide backup archive of the source code and configuration.",
        json!({
            "name": { "type": "string", "description": "Short mnemonic name for the checkpoint" }
        }),
        vec!["name"],
    ));
    tools.push(create_tool(
        "restore_checkpoint",
        "Restore the project from a previously created checkpoint archive.",
        json!({
            "checkpoint_file": { "type": "string", "description": "Filename of the .tar.gz checkpoint" }
        }),
        vec!["checkpoint_file"],
    ));
    tools.push(create_tool(
        "project_wide_replace",
        "Perform a global search and replace across the entire project (filtering target files by \
         glob).",
        json!({
            "old_text": { "type": "string" },
            "new_text": { "type": "string" },
            "glob": { "type": "string", "description": "Glob pattern for files, e.g. '**/*.rs'" }
        }),
        vec!["old_text", "new_text"],
    ));
}
