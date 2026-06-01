use super::create_tool;
use crate::api::types::Tool;
use serde_json::json;

pub fn add_system_schemas(tools: &mut Vec<Tool>) {
    tools.push(create_tool(
        "execute_shell_command",
        "Execute a shell command.",
        json!({
            "command": { "type": "string" },
            "is_background": { "type": "boolean" }
        }),
        vec!["command"],
    ));
    tools.push(create_tool(
        "get_system_info",
        "Get system information.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "start_background_process",
        "Start a command in the background, allowing the agent to continuously monitor its logs \
         and terminate it when needed. Ideal for dev servers.",
        json!({
            "command": { "type": "string", "description": "The command to run" },
            "cwd": { "type": "string", "description": "Optional: working directory" },
            "env": {
                "type": "object",
                "description": "Optional: environment variables as key-value pairs"
            }
        }),
        vec!["command"],
    ));
    tools.push(create_tool(
        "read_background_process_logs",
        "Read accumulated stdout/stderr logs from a running background process.",
        json!({
            "pid": { "type": "integer", "description": "The process ID (PID) of the background process" }
        }),
        vec!["pid"],
    ));
    tools.push(create_tool(
        "kill_background_process",
        "Kill a background process started by the agent.",
        json!({
            "pid": { "type": "integer", "description": "The process ID (PID) to terminate" }
        }),
        vec!["pid"],
    ));
    tools.push(create_tool(
        "list_background_processes",
        "List all active background processes started by the agent.",
        json!({}),
        vec![],
    ));
    tools.push(create_tool(
        "check_port_status",
        "Check if a local port is occupied, free, or blocked.",
        json!({
            "port": { "type": "integer", "description": "Port number to check" },
            "host": { "type": "string", "description": "Optional: host (defaults to '127.0.0.1')" }
        }),
        vec!["port"],
    ));
}
