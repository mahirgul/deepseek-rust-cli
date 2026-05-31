use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::{
    agent::{
        agent::DeepSeekAgent,
        executor::{execute_tool_cached, execute_tools_parallel},
        types::{AgentEvent, ApprovalResult, UndoAction},
    },
    api::{
        streaming::StreamParser,
        types::{Message, ToolCall},
    },
    tools::schemas::get_filtered_tools_schemas,
};

impl DeepSeekAgent {
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
        tracing::info!("chat_stream_inner started, input len: {}", user_input.len());
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
            if self.is_cancelled() {
                break;
            }

            iteration += 1;
            tracing::info!(
                "Starting iteration {} of {}",
                iteration,
                self.config.max_iterations
            );
            let options = crate::api::types::ChatOptions {
                temperature: self.config.temperature,
                top_p: self.config.top_p,
                presence_penalty: self.config.presence_penalty,
                frequency_penalty: self.config.frequency_penalty,
                max_tokens: Some(self.config.max_tokens),
                thinking_enabled: self.config.thinking_enabled,
                reasoning_effort: self.config.reasoning_effort.clone(),
                json_mode: self.config.json_mode,
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
                    Some(get_filtered_tools_schemas(self.is_git_repo, self.has_github_token)),
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
            let mut parser = StreamParser::new();
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

                if self.is_cancelled() {
                    break;
                }

                match item {
                    Ok(bytes) => {
                        let chunks = parser.parse_chunk(&bytes);

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

            let mut approved_calls: Vec<(usize, &ToolCall)> = Vec::new();
            let mut denied_results: Vec<(usize, String, String)> = Vec::new();

            for (i, tc) in tool_calls.iter().enumerate() {
                if self.is_cancelled() {
                    break;
                }
                let name = tc.function.name.as_str();
                let args: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                let is_traversal = crate::agent::security::is_path_traversal_arg(&args);
                let needs_approval = ((crate::agent::security::get_approval_required_tools()
                    .contains(name)
                    || crate::agent::security::is_dangerous_tool(name, &args))
                    && !self.config.debug)
                    || is_traversal;

                let (approved, always) = if needs_approval && (!self.auto_approve || is_traversal) {
                    let approval_name = if is_traversal {
                        format!("path_traversal_warning:{}", tc.function.name)
                    } else {
                        tc.function.name.clone()
                    };
                    if tx
                        .send(AgentEvent::ApprovalRequest {
                            name: approval_name,
                            args: tc.function.arguments.clone(),
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }

                    let cancel_token = self
                        .cancel_token
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    tokio::select! {
                        res = approval_rx.recv() => {
                            match res {
                                Some(ApprovalResult::Yes) => (true, false),
                                Some(ApprovalResult::Always) => {
                                    if is_traversal {
                                        (true, false)
                                    } else {
                                        (true, true)
                                    }
                                }
                                _ => (false, false),
                            }
                        }
                        _ = cancel_token.cancelled() => {
                            (false, false)
                        }
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

            if self.is_cancelled() {
                break;
            }

            if !approved_calls.is_empty() {
                for (_, tc) in &approved_calls {
                    let _ = tx
                        .send(AgentEvent::ToolStart {
                            name: tc.function.name.clone(),
                            args: tc.function.arguments.clone(),
                        })
                        .await;
                }

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

                let results: Vec<(usize, Result<String>, Vec<UndoAction>)> = if tool_inputs.len()
                    == 1
                {
                    let (name, args) = tool_inputs
                        .into_iter()
                        .next()
                        .expect("single tool input must exist");
                    let mut temp_undo = Vec::new();

                    if name == "execute_shell_command" {
                        if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                            if let Some(stripped) = cmd.strip_prefix("cd ") {
                                let new_dir = stripped.trim().trim_matches('"').trim_matches('\'');
                                let target_path = self.cwd.join(new_dir);
                                if let Ok(validated) = crate::tools::base::validate_path(
                                    target_path.to_str().unwrap_or("."),
                                ) {
                                    if validated.exists() && validated.is_dir() {
                                        self.cwd = validated.clone();
                                        let _ = std::env::set_current_dir(&self.cwd);
                                    }
                                }
                            }
                        }
                    }

                    let has_traversal = crate::agent::security::is_path_traversal_arg(&args);
                    let _guard = crate::tools::base::PathTraversalGuard::new(has_traversal);
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
                    let has_traversal = tool_inputs
                        .iter()
                        .any(|(_, args)| crate::agent::security::is_path_traversal_arg(args));
                    let _guard = crate::tools::base::PathTraversalGuard::new(has_traversal);
                    let res = execute_tools_parallel(&tool_inputs, Some(self.cwd.clone())).await;
                    res
                };

                for (tool_idx, result, undo_actions) in results {
                    self.undo_stack.extend(undo_actions);

                    let (_orig_idx, tc) = &approved_calls[tool_idx];
                    let result_str = match result {
                        Ok(res) => res,
                        Err(e) => format!("Error: {}", e),
                    };

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

                    if self.is_cancelled() {
                        break;
                    }

                    let mut stored_content = result_str;
                    if stored_content.len() > self.config.max_tool_output_chars {
                        let trunc: String = stored_content
                            .chars()
                            .take(self.config.max_tool_output_chars)
                            .collect();
                        stored_content = format!(
                            "{}\n\n... [Output Truncated to {} chars (total {} chars) to save \
                             tokens. Use specific tools or grep/read_local_file with line ranges \
                             if you need to read more.] ...",
                            trunc,
                            self.config.max_tool_output_chars,
                            stored_content.len()
                        );
                    }

                    self.messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(stored_content),
                        reasoning_content: None,
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
            }

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

            if self.is_cancelled() {
                break;
            }
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
}
