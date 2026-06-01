use serde_json::json;

use super::create_tool;
use crate::api::types::Tool;

pub fn add_file_io_schemas(tools: &mut Vec<Tool>) {
    tools.push(create_tool(
        "read_local_file",
        "Read a local file.",
        json!({
            "file_path": { "type": "string" },
            "start_line": { "type": "integer" },
            "end_line": { "type": "integer" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "write_local_file",
        "Write to a local file.",
        json!({
            "file_path": { "type": "string" },
            "content": { "type": "string" }
        }),
        vec!["file_path", "content"],
    ));
    tools.push(create_tool(
        "replace_text_in_file",
        "Replace text in a file.",
        json!({
            "file_path": { "type": "string" },
            "old_text": { "type": "string" },
            "new_text": { "type": "string" }
        }),
        vec!["file_path", "old_text", "new_text"],
    ));
    tools.push(create_tool(
        "list_directory",
        "List directory contents.",
        json!({
            "path": { "type": "string" }
        }),
        vec![],
    ));
    tools.push(create_tool(
        "tree_view",
        "Show directory tree.",
        json!({
            "path": { "type": "string" },
            "max_depth": { "type": "integer" }
        }),
        vec![],
    ));
    tools.push(create_tool(
        "delete_file",
        "Delete a file or directory.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "rename_file",
        "Rename or move a file.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "diff_files",
        "Compare two files.",
        json!({
            "file1": { "type": "string" },
            "file2": { "type": "string" }
        }),
        vec!["file1", "file2"],
    ));
    tools.push(create_tool(
        "hash_file",
        "Calculate file hash.",
        json!({
            "path": { "type": "string" },
            "algorithm": { "type": "string", "enum": ["sha256", "md5"] }
        }),
        vec!["path"],
    ));
    tools.push(create_tool(
        "count_lines",
        "Count lines, words and characters in a file.",
        json!({
            "path": { "type": "string" }
        }),
        vec!["path"],
    ));
    tools.push(create_tool(
        "search_files",
        "Search files for a text pattern using native Rust (no shell process needed). Fast \
         parallel search with regex support.",
        json!({
            "query": { "type": "string" },
            "path": { "type": "string" },
            "glob": { "type": "string" },
            "max_results": { "type": "integer" }
        }),
        vec!["query"],
    ));
    tools.push(create_tool(
        "bulk_rename",
        "Rename multiple files in a directory using a regex pattern.",
        json!({
            "path": { "type": "string" },
            "pattern": { "type": "string" },
            "replacement": { "type": "string" }
        }),
        vec!["path", "pattern", "replacement"],
    ));
    tools.push(create_tool(
        "copy_file",
        "Copy a file from source_path to destination_path natively.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "copy_directory",
        "Recursively copy a directory from source_path to destination_path natively.",
        json!({
            "source_path": { "type": "string" },
            "destination_path": { "type": "string" }
        }),
        vec!["source_path", "destination_path"],
    ));
    tools.push(create_tool(
        "create_directory",
        "Create a directory (and any necessary parent directories) natively.",
        json!({
            "directory_path": { "type": "string" }
        }),
        vec!["directory_path"],
    ));
    tools.push(create_tool(
        "file_exists",
        "Check if a file or directory exists at the given path.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
    tools.push(create_tool(
        "get_file_info",
        "Get metadata for a file (type, size, timestamps, permissions) natively.",
        json!({
            "file_path": { "type": "string" }
        }),
        vec!["file_path"],
    ));
}
