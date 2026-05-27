use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThinkingConfig {
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFormat {
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    pub temperature: f32,
    pub top_p: f32,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseChunk {
    pub choices: Vec<ChatChoiceChunk>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoiceChunk {
    pub delta: DeltaMessage,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaMessage {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaToolCall {
    pub index: usize,
    pub id: Option<String>,
    pub function: Option<DeltaFunctionCall>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
    pub max_tokens: Option<u32>,
    pub thinking_enabled: bool,
    pub reasoning_effort: Option<String>,
    pub json_mode: bool,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            max_tokens: Some(16_384),
            thinking_enabled: true,
            reasoning_effort: Some("high".to_string()),
            json_mode: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default)]
    pub prompt_cache_hit_tokens: u64,
    #[serde(default)]
    pub prompt_cache_miss_tokens: u64,
}
