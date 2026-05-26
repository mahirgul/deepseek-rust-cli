use crate::agent::{agent::DeepSeekAgent, history::save_history};

impl DeepSeekAgent {
    pub fn save(&self) {
        save_history(&self.session_id, &self.messages);
    }

    pub fn manage_context(&mut self) {
        let max_chars = self.config.max_context_chars;
        let mut current_chars: usize = self
            .messages
            .iter()
            .map(|m| {
                m.content.as_deref().unwrap_or("").len()
                    + m.reasoning_content.as_deref().unwrap_or("").len()
            })
            .sum();

        while current_chars > max_chars && self.messages.len() > 1 {
            let mut remove_count = 1;
            let idx = 1;

            if idx >= self.messages.len() {
                break;
            }

            match self.messages[idx].role.as_str() {
                "assistant" if self.messages[idx].tool_calls.is_some() => {
                    let mut j = idx + 1;
                    while j < self.messages.len() && self.messages[j].role == "tool" {
                        j += 1;
                    }
                    remove_count = j - idx;
                }
                "assistant" => {}
                "tool" => {
                    let mut j = idx + 1;
                    while j < self.messages.len() && self.messages[j].role == "tool" {
                        j += 1;
                    }
                    let start = idx - 1;
                    remove_count = j - start;
                    let mut chars_removed = 0;
                    for _ in 0..remove_count {
                        if start < self.messages.len() {
                            let removed = self.messages.remove(start);
                            chars_removed += removed.content.as_deref().unwrap_or("").len()
                                + removed.reasoning_content.as_deref().unwrap_or("").len();
                        }
                    }
                    current_chars -= chars_removed;
                    continue;
                }
                _ => {}
            }

            let mut chars_removed = 0;
            for _ in 0..remove_count {
                if idx < self.messages.len() {
                    let removed = self.messages.remove(idx);
                    chars_removed += removed.content.as_deref().unwrap_or("").len()
                        + removed.reasoning_content.as_deref().unwrap_or("").len();
                }
            }
            current_chars -= chars_removed;
        }
    }

    pub fn undo(&mut self) -> String {
        if let Some(action) = self.undo_stack.pop() {
            while self.messages.len() > 1
                && (self
                    .messages
                    .last()
                    .map(|m| m.role == "tool" || m.role == "assistant")
                    .unwrap_or(false))
            {
                self.messages.pop();
            }

            match action.r#type.as_str() {
                "write" | "replace" | "delete" => {
                    if let Some(backup) = action.backup {
                        if backup.is_empty() && action.r#type == "write" {
                            let _ = std::fs::remove_file(&action.path);
                            format!("✅ Undone {}: Deleted {}", action.r#type, action.path)
                        } else {
                            let _ = std::fs::write(&action.path, backup);
                            format!("✅ Undone {}: Restored {}", action.r#type, action.path)
                        }
                    } else if action.r#type == "write" || action.r#type == "replace" {
                        let _ = std::fs::remove_file(&action.path);
                        format!(
                            "✅ Undone {}: Deleted new file {}",
                            action.r#type, action.path
                        )
                    } else {
                        "❌ Undo failed: No backup available.".to_string()
                    }
                }
                "rename" => {
                    if let Some(backup) = action.backup {
                        let original_path = String::from_utf8_lossy(&backup).to_string();
                        let _ = std::fs::rename(&action.path, &original_path);
                        format!(
                            "✅ Undone rename: Moved {} back to {}",
                            action.path, original_path
                        )
                    } else {
                        "❌ Undo failed: No backup path available.".to_string()
                    }
                }
                _ => "❌ Undo failed: Unknown action type".to_string(),
            }
        } else {
            "ℹ️ Undo stack is empty.".to_string()
        }
    }

    pub fn cleanup_aborted_messages(&mut self) {
        while self.messages.last().is_some_and(|m| m.role == "tool") {
            self.messages.pop();
        }
        if self
            .messages
            .last()
            .is_some_and(|m| m.role == "assistant" && m.tool_calls.is_some())
        {
            self.messages.pop();
        }
    }
}
