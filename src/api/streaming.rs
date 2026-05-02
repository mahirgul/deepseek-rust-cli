use crate::api::types::ChatResponseChunk;

pub struct StreamParser;

impl StreamParser {
    pub fn parse_chunk(chunk: &str) -> Vec<ChatResponseChunk> {
        let mut results = Vec::new();
        for line in chunk.lines() {
            if let Some(stripped) = line.strip_prefix("data: ") {
                let data = stripped.trim();
                if data == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<ChatResponseChunk>(data) {
                    results.push(parsed);
                }
            }
        }
        results
    }
}
