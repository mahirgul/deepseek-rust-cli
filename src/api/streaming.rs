use crate::api::types::ChatResponseChunk;

pub struct StreamParser {
    buffer: Vec<u8>,
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamParser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn parse_chunk(&mut self, chunk: &[u8]) -> Vec<ChatResponseChunk> {
        self.buffer.extend_from_slice(chunk);
        let mut results = Vec::new();

        while let Some(newline_pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line_bytes = self.buffer[..newline_pos].to_vec();
            self.buffer.drain(..=newline_pos);

            if let Ok(line_str) = String::from_utf8(line_bytes) {
                let line = line_str.trim();
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
        }
        results
    }
}
