use std::{fs, path::PathBuf};

use crate::api::types::Message;

/// Repair a message list that may have orphaned tool messages or unfulfilled
/// assistant tool_calls. The API requires every `tool` message to be preceded
/// by an assistant message with matching `tool_calls`, and every assistant
/// with `tool_calls` must have its tool responses before the next user message.
fn repair_history(mut messages: Vec<Message>) -> Vec<Message> {
    // Strip trailing orphaned tool messages
    while messages.last().is_some_and(|m| m.role == "tool") {
        messages.pop();
    }
    // Strip trailing assistant message with unfulfilled tool_calls
    if messages
        .last()
        .is_some_and(|m| m.role == "assistant" && m.tool_calls.is_some())
    {
        messages.pop();
    }

    // Walk through and fix mid-sequence corruption:
    // 1. Tool messages whose tool_call_id doesn't match the preceding assistant
    // 2. Assistant messages with tool_calls that have no following tool messages
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role == "tool" {
            let tool_id = messages[i].tool_call_id.as_deref().unwrap_or("");
            // Look backwards for matching assistant message
            let mut found = false;
            for j in (0..i).rev() {
                if messages[j].role == "assistant" {
                    if let Some(ref tcs) = messages[j].tool_calls {
                        if tcs.iter().any(|tc| tc.id == tool_id) {
                            found = true;
                        }
                    }
                    break; // Only check the immediately preceding assistant
                }
            }
            if !found {
                // Remove orphaned tool message
                messages.remove(i);
                continue;
            }
        } else if messages[i].role == "assistant" && messages[i].tool_calls.is_some() {
            // Check if this assistant has at least one tool response before
            // the next user or assistant message
            let mut has_tool_response = false;
            for msg in messages.iter().skip(i + 1) {
                match msg.role.as_str() {
                    "tool" => {
                        has_tool_response = true;
                    }
                    "user" | "assistant" => {
                        break;
                    }
                    _ => {}
                }
            }
            if !has_tool_response {
                // Remove assistant with no tool responses
                messages.remove(i);
                continue;
            }
        }
        i += 1;
    }

    messages
}

pub fn load_history(session_id: &str) -> Vec<Message> {
    let path = get_history_path(session_id);
    if let Some(msgs) = fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<Vec<Message>>(&c).ok())
    {
        let original_len = msgs.len();
        let repaired = repair_history(msgs);
        // Save the repaired history back so the corruption doesn't persist
        if repaired.len() != original_len {
            save_history(session_id, &repaired);
        }
        return repaired;
    }
    Vec::new()
}

pub fn save_history(session_id: &str, messages: &[Message]) {
    let path = get_history_path(session_id);
    let _ = fs::create_dir_all(path.parent().unwrap());
    if let Ok(json) = serde_json::to_string_pretty(messages) {
        let _ = fs::write(path, json);
    }
}

pub fn get_history_path(session_id: &str) -> PathBuf {
    let safe_id: String = session_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let mut path = PathBuf::from(".deep/history");
    path.push(format!("{}.json", safe_id));
    path
}
