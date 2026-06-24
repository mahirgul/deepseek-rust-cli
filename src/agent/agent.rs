use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    agent::{history::load_history, types::UndoAction},
    api::{
        client::DeepSeekClient,
        types::{Message, TokenUsage},
    },
    config::Config,
};

pub struct DeepSeekAgent {
    pub client: Arc<DeepSeekClient>,
    /// Tool result cache to avoid redundant operations
    pub tool_cache: crate::agent::executor::ToolCache,
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
    /// Cached context detection for dynamic tool filtering
    pub is_git_repo: bool,
    pub has_github_token: bool,
}

impl DeepSeekAgent {
    pub fn new(api_key: String, config: Config, session_id: Option<String>) -> Self {
        let sid = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut messages = load_history(&sid);

        if messages.is_empty() {
            let cwd_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();
            let mut base_prompt = config.system_prompt.replace("{cwd}", &cwd_path);
            if config.concise_reasoning {
                base_prompt.push_str(
                    "\nKeep your internal reasoning/thinking process very short and concise.",
                );
            }
            let full_sys = format!(
                "{}\n{}",
                base_prompt,
                crate::agent::context::get_project_context()
            );
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
                config.proxy_url.clone(),
                config.proxy_username.clone(),
                config.proxy_password.clone(),
                config.danger_accept_invalid_certs,
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
            tool_cache: std::collections::HashMap::new(),
            is_git_repo: std::path::Path::new(".git").exists(),
            has_github_token: std::env::var("GITHUB_TOKEN").is_ok(),
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
            tracing::warn!("Cancel token mutex poisoned during abort");
        }
    }

    /// Check if cancelled (lock-free clone for use in hot loops)
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_agent_new_context_chars() {
        let config = Config {
            max_context_chars: 500,
            max_tool_output_chars: 300,
            ..Default::default()
        };
        let agent = DeepSeekAgent::new("dummy_key".to_string(), config, None);
        assert_eq!(agent.config.max_context_chars, 500);
        assert_eq!(agent.config.max_tool_output_chars, 300);
    }
}
