use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct StreamResponse {
    pub choices: Vec<StreamChoice>,
}

#[derive(Deserialize, Debug)]
pub struct StreamChoice {
    pub delta: DeltaMessage,
}

#[derive(Deserialize, Debug)]
pub struct DeltaMessage {
    pub content: Option<String>,
    #[allow(dead_code)]
    pub reasoning_content: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SyncChatResponse {
    pub choices: Vec<SyncChoice>,
}

#[derive(Deserialize, Debug)]
pub struct SyncChoice {
    pub message: Message,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub system_prompt: Option<String>,
    pub init_directories: Option<Vec<String>>,
}
