//! Streaming colorizer for reasoning/content output.

pub mod highlighter;
pub mod types;
pub mod utils;

use crate::tui::colorizer::types::State;
pub use highlighter::CodeColorizer;
use std::fmt::Write as FmtWrite;
pub use types::CodeLang;
pub use utils::truncate_result;

pub struct StreamColorizer {
    state: State,
    /// Pending output that might be part of an unclosed construct
    pending: String,
    dimmed: bool,
    first_feed: bool,
}

impl Default for StreamColorizer {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamColorizer {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            pending: String::new(),
            dimmed: false,
            first_feed: true,
        }
    }

    pub fn set_dimmed(&mut self, dimmed: bool) {
        self.dimmed = dimmed;
    }

    fn reset_code(&self) -> String {
        if self.dimmed {
            "\x1b[0m\x1b[2m".to_string()
        } else {
            "\x1b[0m".to_string()
        }
    }

    /// Feed a chunk of text, return colored output ready to print.
    /// Call `finish()` at the end to flush remaining buffer.
    pub fn feed(&mut self, chunk: &str) -> String {
        let input = format!("{}{}", self.pending, chunk);
        self.pending.clear();

        let mut out = String::new();
        if self.first_feed && self.dimmed {
            out.push_str("\x1b[2m");
            self.first_feed = false;
        }
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            match self.state {
                State::Normal => {
                    // Look for backtick or file path
                    if chars[i] == '`' {
                        // Check for fenced block (```)
                        if i + 2 < len && chars[i + 1] == '`' && chars[i + 2] == '`' {
                            // Opening fenced block
                            let _ = write!(out, "\x1b[33m```{}", self.reset_code()); // yellow backticks
                            i += 3;
                            // Read language tag
                            let mut lang = String::new();
                            while i < len && chars[i] != '\n' && chars[i] != '\r' {
                                lang.push(chars[i]);
                                i += 1;
                            }
                            if !lang.is_empty() {
                                let _ = write!(out, "\x1b[36m{}{}", lang, self.reset_code());
                                // cyan lang
                            }
                            // Skip newline after lang tag
                            if i < len && chars[i] == '\r' {
                                i += 1;
                            }
                            if i < len && chars[i] == '\n' {
                                out.push('\n');
                                i += 1;
                            }
                            self.state = State::FencedBlock {
                                lang: lang.trim().to_string(),
                            };
                        } else {
                            // Start inline code
                            let _ = write!(out, "\x1b[32m`{}", self.reset_code()); // green backtick
                            i += 1;
                            self.state = State::InlineCode;
                        }
                    } else if self.is_path_boundary(&chars, i, len) {
                        // Try to match a file path
                        let path_end = self.match_path(&chars, i, len);
                        if path_end > i {
                            let path: String = chars[i..path_end].iter().collect();
                            let _ = write!(out, "\x1b[34m{}{}", path, self.reset_code()); // blue file path
                            i = path_end;
                        } else {
                            out.push(chars[i]);
                            i += 1;
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                State::InlineCode => {
                    if chars[i] == '`' {
                        let _ = write!(out, "\x1b[32m`{}", self.reset_code()); // closing green backtick
                        i += 1;
                        self.state = State::Normal;
                    } else {
                        // Content inside inline code - keep green
                        out.push_str("\x1b[32m");
                        while i < len && chars[i] != '`' {
                            out.push(chars[i]);
                            i += 1;
                        }
                        out.push_str(&self.reset_code());
                    }
                }
                State::FencedBlock { ref lang } => {
                    // Look for closing ```
                    if chars[i] == '`' && i + 2 < len && chars[i + 1] == '`' && chars[i + 2] == '`'
                    {
                        let _ = write!(out, "\x1b[33m```{}", self.reset_code()); // yellow closing
                        i += 3;
                        self.state = State::Normal;
                    } else {
                        // Content inside code block - dim white
                        let lang_clone = lang.clone();
                        out.push_str("\x1b[37m");
                        while i < len {
                            if chars[i] == '`'
                                && i + 2 < len
                                && chars[i + 1] == '`'
                                && chars[i + 2] == '`'
                            {
                                break;
                            }
                            out.push(chars[i]);
                            i += 1;
                        }
                        out.push_str(&self.reset_code());
                        self.state = State::FencedBlock { lang: lang_clone };
                    }
                }
            }
        }

        out
    }

    /// Call when streaming is done to flush any remaining state
    pub fn finish(&mut self) -> String {
        let mut out = String::new();

        // Close any open constructs
        match self.state {
            State::InlineCode => {
                let _ = write!(out, "\x1b[32m`{}", self.reset_code()); // close inline
            }
            State::FencedBlock { .. } => {
                let _ = write!(out, "\x1b[33m```{}", self.reset_code()); // close block
            }
            _ => {}
        }

        if !self.pending.is_empty() {
            out.push_str(&self.pending);
            self.pending.clear();
        }

        if self.dimmed {
            out.push_str("\x1b[0m");
        }

        self.state = State::Normal;
        self.first_feed = true;
        out
    }

    /// Check if we're at a potential file path boundary
    fn is_path_boundary(&self, chars: &[char], i: usize, _len: usize) -> bool {
        let c = chars[i];
        // Path must start with ./ or ../ or / or a word char that looks like a file
        if c == '.' || c == '/' || c == '~' {
            return true;
        }
        if c.is_alphanumeric() {
            // Check if previous char is whitespace or boundary
            if i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(' || chars[i - 1] == '['
            {
                // Look ahead for path-like patterns
                return true;
            }
        }
        false
    }

    /// Try to match a file path starting at position i
    fn match_path(&self, chars: &[char], start: usize, len: usize) -> usize {
        let mut end = start;

        // Common path prefixes
        if start < len {
            match chars[start] {
                '/' => {
                    end += 1;
                }
                '~' if start + 1 < len && chars[start + 1] == '/' => {
                    end += 2;
                }
                '.' if start + 1 < len && chars[start + 1] == '/' => {
                    end += 2;
                }
                c if c.is_alphanumeric() => {
                    // Must contain a path separator or known extension
                }
                _ => return start,
            }
        }

        // Continue matching path characters
        while end < len {
            let c = chars[end];
            if c.is_alphanumeric()
                || c == '/'
                || c == '.'
                || c == '-'
                || c == '_'
                || c == ' '
                || c == '~'
                || c == '+'
                || c == '@'
                || c == '#'
                || c == ':'
            {
                // Check for common file extensions to stop at
                if c == '.' && end + 1 < len {
                    // Look for known extensions
                    let remaining = &chars[end + 1..];
                    let remaining_str: String = remaining.iter().collect();
                    let ext_candidates = [
                        "rs",
                        "py",
                        "js",
                        "ts",
                        "go",
                        "java",
                        "c",
                        "cpp",
                        "h",
                        "hpp",
                        "rb",
                        "php",
                        "swift",
                        "kt",
                        "scala",
                        "sh",
                        "bash",
                        "zsh",
                        "fish",
                        "ps1",
                        "toml",
                        "yaml",
                        "yml",
                        "json",
                        "xml",
                        "html",
                        "css",
                        "scss",
                        "md",
                        "txt",
                        "log",
                        "csv",
                        "env",
                        "cfg",
                        "conf",
                        "lock",
                        "gitignore",
                        "dockerfile",
                        "nix",
                        "lua",
                        "vim",
                        "el",
                        "ex",
                        "exs",
                        "erl",
                        "hrl",
                        "sql",
                        "graphql",
                        "proto",
                        "vue",
                        "svelte",
                        "tsx",
                        "jsx",
                        "mjs",
                        "wasm",
                        "wat",
                        "bc",
                        "dc",
                        "awk",
                        "sed",
                    ];
                    let matched = ext_candidates.iter().any(|ext| {
                        remaining_str.len() >= ext.len()
                            && remaining_str[..ext.len()].eq_ignore_ascii_case(ext)
                            && (remaining_str.len() == ext.len()
                                || remaining_str
                                    .as_bytes()
                                    .get(ext.len())
                                    .is_none_or(|&b| !b.is_ascii_alphanumeric() && b != b'_'))
                    });
                    if matched {
                        // Include the dot
                        end += 1;
                        // Include the extension
                        while end < len && chars[end].is_alphanumeric() {
                            end += 1;
                        }
                        // Also include trailing / if present
                        if end < len && chars[end] == '/' {
                            end += 1;
                            continue;
                        }
                        break;
                    }
                }

                // Stop at whitespace if we have a valid path
                if c.is_whitespace() {
                    break;
                }

                // Stop at certain punctuation
                if c == ','
                    || c == ';'
                    || c == ')'
                    || c == ']'
                    || c == '}'
                    || c == '"'
                    || c == '\''
                    || c == '>'
                    || c == '`'
                {
                    break;
                }

                end += 1;
            } else {
                break;
            }
        }

        // Must have at least 3 chars and look like a path
        if end - start >= 3 {
            let segment: String = chars[start..end].iter().collect();
            if segment.contains('/')
                || segment.contains('.')
                || segment.ends_with("rc")
                || segment.ends_with("file")
            {
                return end;
            }
        }

        start // no match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_code() {
        let mut c = StreamColorizer::new();
        let out = c.feed("Use `cargo build` to compile.");
        assert!(out.contains("\x1b[32m"));
    }

    #[test]
    fn test_file_path() {
        let mut c = StreamColorizer::new();
        let out = c.feed("Edit src/main.rs and Cargo.toml");
        assert!(out.contains("\x1b[34m"));
    }

    #[test]
    fn test_fenced_block() {
        let mut c = StreamColorizer::new();
        let out = c.feed("```rust\nlet x = 1;\n```");
        assert!(out.contains("\x1b[33m"));
        assert!(out.contains("\x1b[36mrust\x1b[0m"));
    }

    #[test]
    fn test_code_rust_keywords() {
        let code = "fn main() {\n    let x = 42;\n}";
        let colored = CodeColorizer::highlight(code, CodeLang::Rust, None);
        assert!(colored.contains("\x1b[34mfn\x1b[0m"));
        assert!(colored.contains("\x1b[34mlet\x1b[0m"));
        assert!(colored.contains("\x1b[35m42\x1b[0m"));
    }

    #[test]
    fn test_lang_from_path() {
        assert_eq!(CodeLang::from_path("src/main.rs"), CodeLang::Rust);
        assert_eq!(CodeLang::from_path("app.py"), CodeLang::Python);
        assert_eq!(CodeLang::from_path("unknown.xyz"), CodeLang::Generic);
    }

    #[test]
    fn test_truncate_result() {
        let short = "hello";
        assert_eq!(truncate_result(short, 100), "hello");

        let long = "a".repeat(200);
        let truncated = truncate_result(&long, 50);
        assert!(truncated.len() <= 120);
    }
}
