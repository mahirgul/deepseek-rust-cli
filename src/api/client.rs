use crate::api::types::{ChatRequest, Message, Tool};
use anyhow::Result;
use reqwest::{Client, Response};
use std::time::Duration;

pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String, base_url: String, timeout_secs: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key,
            base_url,
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
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
            }

            let response_res = self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&request)
                .send()
                .await;

            match response_res {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    }
                    let status = response.status();
                    let err_text = response.text().await.unwrap_or_default();

                    if status.is_server_error() || status.as_u16() == 429 {
                        last_err = Some(anyhow::anyhow!("API Error ({}): {}", status, err_text));
                        continue;
                    } else {
                        anyhow::bail!("API Error ({}): {}", status, err_text);
                    }
                }
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("Network Error: {}", e));
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("API Request failed after retries")))
    }
}
