use std::{sync::Arc, time::Duration};

use anyhow::Result;
use reqwest::{Client, Response};

use crate::api::types::{ChatRequest, Message, Tool};

/// High-performance HTTP client with connection pooling, HTTP/2, and retry logic.
pub struct DeepSeekClient {
    client: Client,
    api_key: Arc<str>,
    base_url: Arc<str>,
}

impl DeepSeekClient {
    pub fn new(api_key: String, base_url: String, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            // Connection pooling — reuse connections for multiple requests
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .pool_max_idle_per_host(4)
            // TCP keep-alive for persistent connections
            .tcp_keepalive(Some(Duration::from_secs(60)))
            // Use HTTP/2 via ALPN negotiation (works behind proxies/firewalls)
            .http2_adaptive_window(true)
            // Auto-decompress responses
            .gzip(true)
            .brotli(true)
            .zstd(true)
            // Connection timeout
            .connect_timeout(Duration::from_secs(10))
            // User agent
            .user_agent(concat!(
                "deepseek-rust-cli/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_key: Arc::from(api_key),
            base_url: Arc::from(base_url),
        }
    }

    pub async fn chat_completions(
        &self,
        model: &str,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
        options: crate::api::types::ChatOptions,
    ) -> Result<Response> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        tracing::info!("API request to: {}", url);
        tracing::info!("Model: {}, Messages count: {}", model, messages.len());

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            tools,
            tool_choice: Some("auto".to_string()),
            temperature: options.temperature,
            top_p: options.top_p,
            presence_penalty: options.presence_penalty,
            frequency_penalty: options.frequency_penalty,
            max_tokens: options.max_tokens,
        };

        let mut last_err = None;
        // Exponential backoff: 500ms, 1s, 2s
        for attempt in 0..3 {
            if attempt > 0 {
                tracing::info!("Retry attempt {}...", attempt + 1);
                tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
            }

            let response_res = self
                .client
                .post(&url)
                .bearer_auth(self.api_key.as_ref())
                .json(&request)
                .send()
                .await;

            match response_res {
                Ok(response) => {
                    let status = response.status();
                    tracing::info!("API response status: {}", status);
                    if response.status().is_success() {
                        return Ok(response);
                    }
                    let err_text = response.text().await.unwrap_or_default();
                    tracing::error!("API error response: {}", err_text);

                    if status.is_server_error() || status.as_u16() == 429 {
                        last_err = Some(anyhow::anyhow!("API Error ({}): {}", status, err_text));
                        continue;
                    } else {
                        anyhow::bail!("API Error ({}): {}", status, err_text);
                    }
                }
                Err(e) => {
                    tracing::error!("Network error on attempt {}: {}", attempt + 1, e);
                    last_err = Some(anyhow::anyhow!("Network Error: {}", e));
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("API Request failed after retries")))
    }
}
