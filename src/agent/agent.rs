use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    agent::{
        context::get_project_context,
        executor::{execute_tool_cached, execute_tools_parallel, ToolCache},
        history::{load_history, save_history},
    },
    api::{
        client::DeepSeekClient,
        streaming::StreamParser,
        types::{Message, TokenUsage, ToolCall},
    },
    config::Config,
    tools::schemas::get_tools_schemas,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApprovalResult {
    Yes,
    No,
    Always,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Reasoning {
        content: String,
    },
    Content {
        content: String,
    },
    ToolStart {
        name: String,
        args: String,
    },
    ToolEnd {
        name: String,
        /// Truncated result output for display (colorized by TUI)
        result: Option<String>,
    },
    Error {
        content: String,
    },
    ApprovalRequest {
        name: String,
        args: String,
    },
    Done {
        token_usage: TokenUsage,
    },
    Aborted {
        token_usage: TokenUsage,
    },
}

#[derive(Debug, Clone)]
pub struct UndoAction {
    pub r#type: String,
    pub path: String,
    /// For 'write', 'replace', 'delete': contains file content.
    /// For 'rename': contains the original source path as bytes.
    pub backup: Option<Vec<u8>>,
}

pub struct DeepSeekAgent {
    pub client: Arc<DeepSeekClient>,
    /// Tool result cache to avoid redundant operations
    pub tool_cache: ToolCache,
    pub model: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub config: Config,
    pub token_usage: TokenUsage,
    pub undo_stack: Vec<UndoAction>,
    pub auto_approve: bool,
    /// Shared cancel token — can be cancelled from outside the agent mutex
    pub cancel_token: Arc<std::sync::Mutex<CancellationToken>>,
    pub run_id: Arc<std::sync::atomic::AtomicUsize>,
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
            cancel_token: Arc::new(std::sync::Mutex::new(CancellationToken::new())),
            run_id: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            tool_cache: HashMap::new(),
        }
    }

    /// Reset the cancellation token for a new request
    pub fn reset_cancel(&mut self) {
        let mut token = self.cancel_token.lock().unwrap_or_else(|e| e.into_inner());
        *token = CancellationToken::new();
    }

    /// Abort the current streaming request
    pub fn abort(&self) {
        if let Ok(token) = self.cancel_token.lock() {
            token.cancel();
        } else {
            // If mutex is poisoned, the token in its current state is invalid.
            // Replace it with a fresh cancelled token so subsequent checks see cancelled.
            // We can't easily replace the Arc'd inner value, so just log.
            tracing::warn!("Cancel token mutex poisoned during abort");
        }
    }

    /// Check if cancelled (lock-free clone for use in hot loops)
    fn is_cancelled(&self) -> bool {
        self.cancel_token
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_cancelled()
    }

    /// Remove incomplete tool-call chain from messages after an abort.
    /// The API requires every `tool` message be preceded by an assistant
    /// message with matching `tool_calls`. After abort mid-execution,
    /// we may have orphaned tool results or an unfulfilled tool_calls.
    fn cleanup_aborted_messages(&mut self) {
        // Strip trailing `tool` messages (orphaned results)
        while self.messages.last().is_some_and(|m| m.role == "tool") {
            self.messages.pop();
        }
        // If the last message is an assistant with tool_calls, remove it too
        // (it has no matching tool responses, or only partial ones)
        if self
            .messages
            .last()
            .is_some_and(|m| m.role == "assistant" && m.tool_calls.is_some())
        {
            self.messages.pop();
        }
    }

    pub async fn chat_stream(
        &mut self,
        user_input: String,
        tx: mpsc::Sender<AgentEvent>,
        approval_rx: &mut mpsc::Receiver<ApprovalResult>,
    ) -> Result<()> {
        self.manage_context();
        self.reset_cancel();
        // Clear tool cache each request
        self.tool_cache.clear();
        let res = self
            .chat_stream_inner(user_input, tx.clone(), approval_rx)
            .await;

        // If cancelled, clean up orphaned tool messages BEFORE saving,
        // otherwise malformed history breaks subsequent API calls.
        if self.is_cancelled() {
            self.cleanup_aborted_messages();
        }
        self.save();

        // If cancelled, send aborted event
        if self.is_cancelled() {
            let _ = tx
                .send(AgentEvent::Aborted {
                    token_usage: self.token_usage.clone(),
                })
                .await;
        }

        res
    }

    async fn chat_stream_inner(
        &mut self,
        user_input: String,
        tx: mpsc::Sender<AgentEvent>,
        approval_rx: &mut mpsc::Receiver<ApprovalResult>,
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
            if self.is_cancelled() {
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

            let cancel_token = self
                .cancel_token
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            let response_res = tokio::select! {
                res = self.client.chat_completions(
                    &self.model,
                    self.messages.clone(),
                    Some(get_tools_schemas()),
                    options,
                ) => res,
                _ = cancel_token.cancelled() => {
                    break;
                }
            };

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

            loop {
                let item_res = tokio::select! {
                    item = stream.next() => item,
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                };

                let item = match item_res {
                    Some(item) => item,
                    None => break,
                };

                // Check cancellation during streaming
                if self.is_cancelled() {
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
                                        .send(AgentEvent::Reasoning { content: reasoning })
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
                                    if tx.send(AgentEvent::Content { content }).await.is_err() {
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
            if self.is_cancelled() {
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

                // Execute in parallel (or single, with unified result type)
                let results: Vec<(usize, Result<String>, Vec<UndoAction>)> = if tool_inputs.len()
                    == 1
                {
                    // Single tool - execute directly with cache
                    let (name, args) = tool_inputs
                        .into_iter()
                        .next()
                        .expect("single tool input must exist");
                    let mut temp_undo = Vec::new();

                    // Special handling for 'cd' to keep state
                    if name == "execute_shell_command" {
                        if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                            if let Some(stripped) = cmd.strip_prefix("cd ") {
                                let new_dir = stripped.trim().trim_matches('"').trim_matches('\'');
                                let target_path = self.cwd.join(new_dir);
                                if let Ok(validated) = crate::tools::base::validate_path(
                                    target_path.to_str().unwrap_or("."),
                                ) {
                                    if validated.exists() && validated.is_dir() {
                                        self.cwd = validated;
                                    }
                                }
                            }
                        }
                    }

                    let (result, _cached) = execute_tool_cached(
                        &name,
                        &args,
                        &mut temp_undo,
                        &mut self.tool_cache,
                        Some(&self.cwd),
                    )
                    .await;
                    vec![(0, result, temp_undo)]
                } else {
                    // Multiple tools - execute in parallel
                    execute_tools_parallel(&tool_inputs, Some(self.cwd.clone())).await
                };

                // Notify end and add tool messages
                // Each result tuple is (original_tool_index, result, undo_actions)
                for (tool_idx, result, undo_actions) in results {
                    // Merge undo actions
                    self.undo_stack.extend(undo_actions);

                    let (_orig_idx, tc) = &approved_calls[tool_idx];
                    let result_str = match result {
                        Ok(res) => res,
                        Err(e) => format!("Error: {}", e),
                    };

                    // Truncate result for TUI display (max ~500 chars)
                    let display_result = Some(if result_str.len() > 500 {
                        let trunc: String = result_str.chars().take(500).collect();
                        format!(
                            "{}\n... (truncated, {} total chars)",
                            trunc,
                            result_str.len()
                        )
                    } else {
                        result_str.clone()
                    });

                    let _ = tx
                        .send(AgentEvent::ToolEnd {
                            name: tc.function.name.clone(),
                            result: display_result,
                        })
                        .await;

                    // Check cancellation
                    if self.is_cancelled() {
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
                        result: Some(msg.clone()),
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
            if self.is_cancelled() {
                break;
            }
        }

        // Note: AgentEvent::Done is sent by the caller (main.rs) after chat_stream returns

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

        // Remove oldest messages (after system prompt at index 0) while
        // preserving assistant/tool message pairing. Removing an assistant
        // with tool_calls also removes its tool results; removing a tool
        // message also removes its parent assistant.
        while current_chars > max_chars && self.messages.len() > 1 {
            // We always remove starting from index 1 (after system message).
            // Determine how many consecutive messages to remove as a group.
            let mut remove_count = 1;
            let idx = 1;

            if idx >= self.messages.len() {
                break;
            }

            match self.messages[idx].role.as_str() {
                "assistant" if self.messages[idx].tool_calls.is_some() => {
                    // If this assistant has tool_calls, also remove the
                    // following tool messages that belong to it.
                    let mut j = idx + 1;
                    while j < self.messages.len() && self.messages[j].role == "tool" {
                        j += 1;
                    }
                    remove_count = j - idx;
                }
                "assistant" => {}
                "tool" => {
                    // This tool message belongs to the preceding assistant.
                    // Remove the assistant too to keep pairing intact.
                    // Extend removal forward to include all sibling tool messages.
                    let mut j = idx + 1;
                    while j < self.messages.len() && self.messages[j].role == "tool" {
                        j += 1;
                    }
                    // Also remove the preceding assistant
                    let start = idx - 1; // assistant is just before
                    remove_count = j - start;
                    // Drain from `start` instead of `idx`
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

            // Remove the determined number of messages
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
}
