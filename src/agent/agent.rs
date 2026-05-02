use crate::agent::context::get_project_context;
use crate::agent::executor::{execute_tool, execute_tools_parallel};
use crate::agent::history::{load_history, save_history};
use crate::api::client::DeepSeekClient;
use crate::api::streaming::StreamParser;
use crate::api::types::{Message, TokenUsage, ToolCall};
use crate::config::Config;
use crate::tools::get_all_tools;
use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApprovalResult {
    Yes,
    No,
    Always,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Reasoning { content: String },
    Content { content: String },
    ToolStart { name: String, args: String },
    ToolEnd { name: String },
    Error { content: String },
    ApprovalRequest { name: String, args: String },
    Done,
    Aborted,
}

#[derive(Debug, Clone)]
pub struct UndoAction {
    pub r#type: String,
    pub path: String,
    pub backup: Option<Vec<u8>>,
}

pub struct DeepSeekAgent {
    pub client: Arc<DeepSeekClient>,
    pub model: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub config: Config,
    pub token_usage: TokenUsage,
    pub undo_stack: Vec<UndoAction>,
    pub auto_approve: bool,
    pub cancel_token: CancellationToken,
    pub cwd: std::path::PathBuf,
}

impl DeepSeekAgent {
    pub fn new(api_key: String, config: Config, session_id: Option<String>) -> Self {
        let sid = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut messages = load_history(&sid);

        if messages.is_empty() {
            let mut base_prompt = config.system_prompt.clone();
            if config.concise_reasoning {
                base_prompt.push_str(
                    "\nKeep your internal reasoning/thinking process very short and concise.",
                );
            }
            let full_sys = format!("{}\n{}", base_prompt, get_project_context());
            messages.push(Message {
                role: "system".to_string(),
                content: Some(full_sys),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        Self {
            client: Arc::new(DeepSeekClient::new(
                api_key,
                config.base_url.clone(),
                config.request_timeout,
            )),
            model: config.model.clone(),
            session_id: sid,
            messages,
            config,
            token_usage: TokenUsage::default(),
            undo_stack: Vec::new(),
            auto_approve: false,
            cancel_token: CancellationToken::new(),
            cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        }
    }

    /// Reset the cancellation token for a new request
    pub fn reset_cancel(&mut self) {
        self.cancel_token = CancellationToken::new();
    }

    /// Abort the current streaming request
    pub fn abort(&self) {
        self.cancel_token.cancel();
    }

    pub async fn chat_stream(
        &mut self,
        user_input: String,
        tx: mpsc::Sender<AgentEvent>,
        approval_rx: mpsc::Receiver<ApprovalResult>,
    ) -> Result<()> {
        self.manage_context();
        self.reset_cancel();
        let res = self
            .chat_stream_inner(user_input, tx.clone(), approval_rx)
            .await;
        self.save();

        // If cancelled, send aborted event
        if self.cancel_token.is_cancelled() {
            let _ = tx.send(AgentEvent::Aborted).await;
        }

        res
    }

    async fn chat_stream_inner(
        &mut self,
        user_input: String,
        tx: mpsc::Sender<AgentEvent>,
        mut approval_rx: mpsc::Receiver<ApprovalResult>,
    ) -> Result<()> {
        if !user_input.is_empty() {
            self.messages.push(Message {
                role: "user".to_string(),
                content: Some(user_input),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let mut iteration = 0;
        while iteration < self.config.max_iterations {
            // Check for cancellation
            if self.cancel_token.is_cancelled() {
                break;
            }

            iteration += 1;
            let options = crate::api::types::ChatOptions {
                temperature: self.config.temperature,
                top_p: self.config.top_p,
                presence_penalty: self.config.presence_penalty,
                frequency_penalty: self.config.frequency_penalty,
                max_tokens: Some(self.config.max_tokens),
            };

            let response_res = self
                .client
                .chat_completions(
                    &self.model,
                    self.messages.clone(),
                    Some(get_all_tools()),
                    options,
                )
                .await;

            let response = match response_res {
                Ok(res) => res,
                Err(e) => {
                    tracing::error!("API Request Failed: {}", e);
                    let _ = tx
                        .send(AgentEvent::Error {
                            content: format!("API Error: {}", e),
                        })
                        .await;
                    break;
                }
            };

            let mut full_content = String::new();
            let mut full_reasoning = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();

            let mut stream = response.bytes_stream();
            let mut stream_error = None;

            while let Some(item) = stream.next().await {
                // Check cancellation during streaming
                if self.cancel_token.is_cancelled() {
                    break;
                }

                match item {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        let chunks = StreamParser::parse_chunk(&chunk_str);

                        for chunk in chunks {
                            if let Some(usage) = chunk.usage {
                                self.token_usage.prompt_tokens += usage.prompt_tokens;
                                self.token_usage.completion_tokens += usage.completion_tokens;
                            }

                            for choice in chunk.choices {
                                if let Some(reasoning) =
                                    choice.delta.reasoning_content.filter(|r| !r.is_empty())
                                {
                                    full_reasoning.push_str(&reasoning);
                                    if tx
                                        .send(AgentEvent::Reasoning {
                                            content: reasoning,
                                        })
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                if let Some(content) =
                                    choice.delta.content.filter(|c| !c.is_empty())
                                {
                                    full_content.push_str(&content);
                                    if tx
                                        .send(AgentEvent::Content { content })
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                if let Some(deltas) = choice.delta.tool_calls {
                                    for delta in deltas {
                                        while tool_calls.len() <= delta.index {
                                            tool_calls.push(ToolCall {
                                                id: String::new(),
                                                r#type: "function".to_string(),
                                                function: crate::api::types::FunctionCall {
                                                    name: String::new(),
                                                    arguments: String::new(),
                                                },
                                            });
                                        }
                                        let tc = &mut tool_calls[delta.index];
                                        if let Some(id) = delta.id {
                                            tc.id.push_str(&id);
                                        }
                                        if let Some(f) = delta.function {
                                            if let Some(n) = f.name {
                                                tc.function.name.push_str(&n);
                                            }
                                            if let Some(a) = f.arguments {
                                                tc.function.arguments.push_str(&a);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        stream_error = Some(format!("Stream Error: {}", e));
                        break;
                    }
                }
            }

            // If cancelled, stop processing
            if self.cancel_token.is_cancelled() {
                break;
            }

            if let Some(err) = stream_error {
                tracing::error!("Response Stream Error: {}", err);
                let _ = tx.send(AgentEvent::Error { content: err }).await;
                break;
            }

            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: if full_content.is_empty() {
                    None
                } else {
                    Some(full_content.clone())
                },
                reasoning_content: if full_reasoning.is_empty() {
                    None
                } else {
                    Some(full_reasoning.clone())
                },
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls.clone())
                },
                tool_call_id: None,
            };
            self.messages.push(assistant_msg);

            if tool_calls.is_empty() {
                break;
            }

            // Group tool calls: check for approval on all first
            let mut approved_calls: Vec<(usize, &ToolCall)> = Vec::new();
            let mut denied_results: Vec<(usize, String, String)> = Vec::new();

            for (i, tc) in tool_calls.iter().enumerate() {
                let name = tc.function.name.as_str();
                let args: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                let needs_approval = (crate::agent::security::get_approval_required_tools()
                    .contains(name)
                    || crate::agent::security::is_dangerous_tool(name, &args))
                    && !self.config.debug;

                let (approved, always) = if needs_approval && !self.auto_approve {
                    if tx
                        .send(AgentEvent::ApprovalRequest {
                            name: tc.function.name.clone(),
                            args: tc.function.arguments.clone(),
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }

                    match approval_rx.recv().await {
                        Some(ApprovalResult::Yes) => (true, false),
                        Some(ApprovalResult::Always) => (true, true),
                        _ => (false, false),
                    }
                } else {
                    (true, false)
                };

                if always {
                    self.auto_approve = true;
                }

                if approved {
                    approved_calls.push((i, tc));
                } else {
                    denied_results.push((
                        i,
                        tc.id.clone(),
                        "Tool execution denied by user.".to_string(),
                    ));
                }
            }

            // Execute approved tools in parallel
            if !approved_calls.is_empty() {
                // Notify start for each tool
                for (_, tc) in &approved_calls {
                    let _ = tx
                        .send(AgentEvent::ToolStart {
                            name: tc.function.name.clone(),
                            args: tc.function.arguments.clone(),
                        })
                        .await;
                }

                // Execute in parallel
                let tool_inputs: Vec<(String, serde_json::Map<String, serde_json::Value>)> =
                    approved_calls
                        .iter()
                        .map(|(_, tc)| {
                            (
                                tc.function.name.clone(),
                                serde_json::from_str(&tc.function.arguments).unwrap_or_default(),
                            )
                        })
                        .collect();

                let results = if tool_inputs.len() == 1 {
                    // Single tool - execute directly for proper undo stack
                    let (name, args) = tool_inputs.into_iter().next().unwrap();
                    
                    // Special handling for 'cd' to keep state
                    if name == "execute_shell_command"
                        && let Some(cmd) = args.get("command").and_then(|v| v.as_str())
                        && let Some(stripped) = cmd.strip_prefix("cd ")
                    {
                        let new_dir = stripped.trim().trim_matches('"').trim_matches('\'');
                        let target_path = self.cwd.join(new_dir);
                        if target_path.exists() && target_path.is_dir()
                            && let Ok(abs_path) = std::fs::canonicalize(target_path)
                        {
                            self.cwd = abs_path;
                        }
                    }

                    let result = execute_tool(&name, &args, &mut self.undo_stack, Some(&self.cwd)).await;
                    vec![(0, result, Vec::new())]
                } else {
                    // Multiple tools - execute in parallel
                    execute_tools_parallel(&tool_inputs, Some(self.cwd.clone())).await
                };

                // Notify end and add tool messages
                for (idx, result, undo_actions) in results {
                    // Merge undo actions
                    self.undo_stack.extend(undo_actions);
                    
                    let (_orig_idx, tc) = &approved_calls[idx];
                    let result_str = match result {
                        Ok(res) => res,
                        Err(e) => format!("Error: {}", e),
                    };

                    let _ = tx
                        .send(AgentEvent::ToolEnd {
                            name: tc.function.name.clone(),
                        })
                        .await;

                    // Check cancellation
                    if self.cancel_token.is_cancelled() {
                        break;
                    }

                    self.messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(result_str),
                        reasoning_content: None,
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
            }

            // Handle denied tools
            for (_, tool_id, msg) in denied_results {
                let _ = tx
                    .send(AgentEvent::ToolEnd {
                        name: "denied".to_string(),
                    })
                    .await;
                self.messages.push(Message {
                    role: "tool".to_string(),
                    content: Some(msg),
                    reasoning_content: None,
                    tool_calls: None,
                    tool_call_id: Some(tool_id),
                });
            }

            // Check cancellation after tool execution
            if self.cancel_token.is_cancelled() {
                break;
            }
        }

        if !self.cancel_token.is_cancelled() {
            let _ = tx.send(AgentEvent::Done).await;
        }

        if self.config.show_token_usage {
            let total = self.token_usage.prompt_tokens + self.token_usage.completion_tokens;
            let usage_msg = format!(
                "\n{} [{} {} | {} {} | {} {}]\n",
                "📊 Token Usage:".bold().blue(),
                "Prompt:".cyan(),
                self.token_usage.prompt_tokens.to_string().cyan(),
                "Completion:".green(),
                self.token_usage.completion_tokens.to_string().green(),
                "Total:".yellow(),
                total.to_string().yellow()
            );
            let _ = tx.send(AgentEvent::Content { content: usage_msg }).await;
        }

        Ok(())
    }

    pub fn save(&self) {
        save_history(&self.session_id, &self.messages);
    }

    fn manage_context(&mut self) {
        let max_chars = 120_000;
        let mut current_chars: usize = self
            .messages
            .iter()
            .map(|m| {
                m.content.as_deref().unwrap_or("").len()
                    + m.reasoning_content.as_deref().unwrap_or("").len()
            })
            .sum();

        while current_chars > max_chars && self.messages.len() > 1 {
            if self.messages.len() > 1 {
                let removed = self.messages.remove(1);
                current_chars -= removed.content.as_deref().unwrap_or("").len()
                    + removed.reasoning_content.as_deref().unwrap_or("").len();
            } else {
                break;
            }
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
                    } else {
                        if action.r#type == "write" || action.r#type == "replace" {
                            let _ = std::fs::remove_file(&action.path);
                            format!(
                                "✅ Undone {}: Deleted new file {}",
                                action.r#type, action.path
                            )
                        } else {
                            "❌ Undo failed: No backup available.".to_string()
                        }
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
}
