use crate::api::types::ChatResponseChunk;

pub struct StreamParser {
    buffer: String,
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn parse_chunk(&mut self, chunk: &str) -> Vec<ChatResponseChunk> {
        self.buffer.push_str(chunk);
        let mut results = Vec::new();

        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer.drain(..=newline_pos);

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

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
