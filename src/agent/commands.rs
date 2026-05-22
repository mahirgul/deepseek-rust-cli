use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::agent::{agent::DeepSeekAgent, history::load_history};

pub async fn process_command(agent: &mut DeepSeekAgent, text: &str) -> Result<Option<String>> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }

    let cmd = parts[0].to_lowercase();
    match cmd.as_str() {
        "/model" => {
            if parts.len() > 1 {
                agent.model = parts[1].to_string();
                agent.config.model = agent.model.clone();
                let _ = agent.config.save();
                Ok(Some(format!("Model switched to {}", agent.model)))
            } else {
                Ok(Some(format!("Current model: {}", agent.model)))
            }
        }
        "/clear" => {
            agent.messages.truncate(1);
            agent.save();
            Ok(Some("History cleared.".to_string()))
        }
        "/forget" => {
            agent.messages.truncate(1);
            let path = PathBuf::from(".deep/history").join(format!("{}.json", agent.session_id));
            let _ = fs::remove_file(path);
            Ok(Some(
                "Session history forgotten and deleted from disk.".to_string(),
            ))
        }
        "/undo" => Ok(Some(agent.undo())),
        "/tokens" => Ok(Some(format!(
            "Token Usage: {} prompt, {} completion (Total: {})",
            agent.token_usage.prompt_tokens,
            agent.token_usage.completion_tokens,
            agent.token_usage.prompt_tokens + agent.token_usage.completion_tokens
        ))),
        "/temperature" => {
            if parts.len() > 1 {
                if let Ok(temp) = parts[1].parse::<f32>() {
                    agent.config.temperature = temp;
                    let _ = agent.config.save();
                    Ok(Some(format!("Temperature set to {}", temp)))
                } else {
                    Ok(Some("Invalid temperature value.".to_string()))
                }
            } else {
                Ok(Some(format!(
                    "Current temperature: {}",
                    agent.config.temperature
                )))
            }
        }
        "/auto" => {
            agent.auto_approve = !agent.auto_approve;
            let status = if agent.auto_approve {
                "enabled"
            } else {
                "disabled"
            };
            Ok(Some(format!("Auto-approve is now {}", status)))
        }
        "/info" => {
            let info = format!(
                "Session ID: {}\nModel: {}\nTemperature: {}\nAuto-approve: {}\nHistory length: {} \
                 messages\nTokens: P:{} C:{} T:{}",
                agent.session_id,
                agent.model,
                agent.config.temperature,
                agent.auto_approve,
                agent.messages.len(),
                agent.token_usage.prompt_tokens,
                agent.token_usage.completion_tokens,
                agent.token_usage.prompt_tokens + agent.token_usage.completion_tokens
            );
            Ok(Some(info))
        }
        "/sessions" => {
            let mut sessions = Vec::new();
            if let Ok(entries) = fs::read_dir(".deep/history") {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str().filter(|n| n.ends_with(".json"))
                    {
                        sessions.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
            if sessions.is_empty() {
                Ok(Some("No saved sessions found.".to_string()))
            } else {
                Ok(Some(format!(
                    "Available sessions:\n- {}",
                    sessions.join("\n- ")
                )))
            }
        }
        "/resume" => {
            if parts.len() > 1 {
                let new_sid = parts[1].to_string();
                agent.session_id = new_sid.clone();
                agent.messages = load_history(&new_sid);
                if agent.messages.is_empty() {
                    let full_sys = format!(
                        "{}\n{}",
                        agent.config.system_prompt,
                        crate::agent::context::get_project_context()
                    );
                    agent.messages.push(crate::api::types::Message {
                        role: "system".to_string(),
                        content: Some(full_sys),
                        reasoning_content: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    agent.save();
                    Ok(Some(format!(
                        "Session {} not found, started new session with system prompt.",
                        new_sid
                    )))
                } else {
                    Ok(Some(format!("Resumed session: {}", new_sid)))
                }
            } else {
                Ok(Some("Usage: /resume <session_id>".to_string()))
            }
        }
        "/savemem" => {
            if parts.len() > 1 {
                let note = text.trim_start_matches("/savemem").trim();
                let mut memory = fs::read_to_string(".deep/memory.md").unwrap_or_default();
                memory.push_str(&format!("\n- {}\n", note));
                fs::write(".deep/memory.md", memory)?;
                Ok(Some("Note saved to memory.md".to_string()))
            } else {
                Ok(Some("Usage: /savemem <note content>".to_string()))
            }
        }
        "/export" => {
            let mut export = format!(
                "# DeepSeek Session Export\n\n- **Session:** {}\n- **Model:** {}\n\n---\n\n",
                agent.session_id, agent.model
            );
            for msg in &agent.messages {
                export.push_str(&format!(
                    "### {}\n{}\n\n",
                    msg.role.to_uppercase(),
                    msg.content.as_deref().unwrap_or("(No content)")
                ));
            }
            let filename = format!("export_{}.md", agent.session_id);
            fs::write(&filename, export)?;
            Ok(Some(format!("Session exported to {}", filename)))
        }
        "/retry" => Ok(Some("RETRY".to_string())),
        "/config" => {
            if parts.len() > 1 {
                let key = parts[1].to_lowercase();
                if parts.len() > 2 {
                    let val = parts[2];
                    match key.as_str() {
                        "model" => agent.config.model = val.to_string(),
                        "url" | "base_url" => agent.config.base_url = val.to_string(),
                        "temp" | "temperature" => {
                            if let Ok(v) = val.parse() {
                                agent.config.temperature = v
                            } else {
                                return Ok(Some("Invalid float".into()));
                            }
                        }
                        "top_p" => {
                            if let Ok(v) = val.parse() {
                                agent.config.top_p = v
                            } else {
                                return Ok(Some("Invalid float".into()));
                            }
                        }
                        "presence" | "presence_penalty" => {
                            if let Ok(v) = val.parse() {
                                agent.config.presence_penalty = v
                            } else {
                                return Ok(Some("Invalid float".into()));
                            }
                        }
                        "frequency" | "frequency_penalty" => {
                            if let Ok(v) = val.parse() {
                                agent.config.frequency_penalty = v
                            } else {
                                return Ok(Some("Invalid float".into()));
                            }
                        }
                        "max_tokens" => {
                            if let Ok(v) = val.parse() {
                                agent.config.max_tokens = v
                            } else {
                                return Ok(Some("Invalid integer".into()));
                            }
                        }
                        "max_iterations" => {
                            if let Ok(v) = val.parse() {
                                agent.config.max_iterations = v
                            } else {
                                return Ok(Some("Invalid integer".into()));
                            }
                        }
                        "tokens" | "show_token_usage" => {
                            agent.config.show_token_usage =
                                val.to_lowercase() == "true" || val == "1"
                        }
                        "short" | "concise_reasoning" => {
                            agent.config.concise_reasoning =
                                val.to_lowercase() == "true" || val == "1"
                        }
                        "debug" => agent.config.debug = val.to_lowercase() == "true" || val == "1",
                        _ => return Ok(Some(format!("Unknown config key: {}", key))),
                    }
                    let _ = agent.config.save();
                    Ok(Some(format!("Config {} set to {}", key, val)))
                } else {
                    let val = match key.as_str() {
                        "model" => agent.config.model.clone(),
                        "url" | "base_url" => agent.config.base_url.clone(),
                        "temp" | "temperature" => agent.config.temperature.to_string(),
                        "top_p" => agent.config.top_p.to_string(),
                        "presence" | "presence_penalty" => {
                            agent.config.presence_penalty.to_string()
                        }
                        "frequency" | "frequency_penalty" => {
                            agent.config.frequency_penalty.to_string()
                        }
                        "max_tokens" => agent.config.max_tokens.to_string(),
                        "max_iterations" => agent.config.max_iterations.to_string(),
                        "tokens" | "show_token_usage" => agent.config.show_token_usage.to_string(),
                        "short" | "concise_reasoning" => agent.config.concise_reasoning.to_string(),
                        "debug" => agent.config.debug.to_string(),
                        _ => format!("Unknown config key: {}", key),
                    };
                    Ok(Some(format!("{} = {}", key, val)))
                }
            } else {
                let conf = format!(
                    "Current Configuration:\n- model: {}\n- base_url: {}\n- temperature: {}\n- \
                     top_p: {}\n- presence_penalty: {}\n- frequency_penalty: {}\n- max_tokens: \
                     {}\n- max_iterations: {}\n- show_token_usage: {}\n- concise_reasoning: {}\n- \
                     debug: {}",
                    agent.config.model,
                    agent.config.base_url,
                    agent.config.temperature,
                    agent.config.top_p,
                    agent.config.presence_penalty,
                    agent.config.frequency_penalty,
                    agent.config.max_tokens,
                    agent.config.max_iterations,
                    agent.config.show_token_usage,
                    agent.config.concise_reasoning,
                    agent.config.debug
                );
                Ok(Some(conf))
            }
        }
        "/help" => {
            let help = r#"
Available Commands:
  /model [name]    - Show or switch current model
  /clear           - Clear current conversation history
  /forget          - Delete session history from disk
  /undo            - Undo last file/shell operation
  /tokens          - Show current session token usage
  /temperature [v] - Show or set model temperature
  /auto            - Toggle auto-approval for tools
  /info            - Show detailed session info
  /sessions        - List all saved sessions
  /resume <id>     - Switch to a different session
  /savemem <text>  - Save a note to memory.md
  /export          - Export session to a Markdown file
  /retry           - Regenerate last assistant response
  /config          - Show or set configuration values
  /update          - Check for and install updates
  /help            - Show this help message
  /exit, /quit     - Exit the application (also 'exit' or 'quit')
"#;
            Ok(Some(help.trim().to_string()))
        }
        "/update" => crate::updater::run_update().map(Some),
        _ => {
            if cmd.starts_with('/') {
                Ok(Some(suggest_command(&cmd)))
            } else {
                Ok(None)
            }
        }
    }
}

const COMMANDS: &[&str] = &[
    "/model",
    "/clear",
    "/forget",
    "/undo",
    "/tokens",
    "/temperature",
    "/auto",
    "/info",
    "/sessions",
    "/resume",
    "/savemem",
    "/export",
    "/retry",
    "/config",
    "/update",
    "/help",
    "/exit",
    "/quit",
];

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut row: Vec<usize> = (0..=len_b).collect();
    for i in 1..=len_a {
        let mut prev_diag = row[0];
        row[0] = i;
        for j in 1..=len_b {
            let temp = row[j];
            if a_chars[i - 1] == b_chars[j - 1] {
                row[j] = prev_diag;
            } else {
                row[j] = 1 + std::cmp::min(row[j], std::cmp::min(row[j - 1], prev_diag));
            }
            prev_diag = temp;
        }
    }
    row[len_b]
}

fn suggest_command(cmd: &str) -> String {
    let mut best_match = None;
    let mut best_dist = usize::MAX;

    for &c in COMMANDS {
        let dist = levenshtein_distance(cmd, c);
        if dist < best_dist {
            best_dist = dist;
            best_match = Some(c);
        }
    }

    if let Some(m) = best_match {
        if best_dist <= 3 {
            return format!("❌ Unknown command: {}. Did you mean `{}`?", cmd, m);
        }
    }
    format!(
        "❌ Unknown command: {}. Type `/help` to see available commands.",
        cmd
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("cat", "cat"), 0);
        assert_eq!(levenshtein_distance("cat", "cut"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_suggest_command() {
        assert!(suggest_command("/toke").contains("Did you mean `/tokens`?"));
        assert!(suggest_command("/conf").contains("Did you mean `/config`?"));
        assert!(suggest_command("/abcdef").contains("Type `/help` to see available commands"));
    }
}
