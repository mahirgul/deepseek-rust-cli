//! Streaming colorizer for reasoning/content output.
//!
//! Maintains parse state across chunks to color:
//! - File paths (e.g. `src/main.rs`, `/etc/nginx/conf.d/default.conf`)
//! - Inline code (`code`)
//! - Code blocks (```lang ... ```)
//! - URLs
//!
//! Also provides `CodeColorizer` for syntax-highlighting code snippets
//! (tool outputs, file contents, shell command results).

use std::fmt::Write as FmtWrite;

/// Parse state carried across chunks
#[derive(Debug, Clone, PartialEq)]
enum State {
    Normal,
    /// Inside inline code `` ` ``
    InlineCode,
    /// Inside a fenced code block (language tag stored)
    FencedBlock {
        lang: String,
    },
}

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

        // Save any trailing content that might be incomplete
        // (e.g., a backtick at the very end that could start inline code)
        // For now, we rely on the next chunk to complete it.
        // We keep nothing pending — the state handles partial constructs.
        if self.state == State::Normal {
            // No pending needed
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
                    // Scan ahead to see if this looks like a path
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
                        // Find end of extension
                        while end < len && chars[end].is_alphanumeric() {
                            end += 1;
                        }
                        // Also include trailing / if present
                        if end < len && chars[end] == '/' {
                            end += 1;
                            // Continue matching after /
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
            // Check if it has a path separator or extension
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

// ── CodeColorizer: Syntax highlighting for tool outputs ──────────

/// Detected language for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodeLang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Shell,
    Json,
    Toml,
    Yaml,
    Html,
    Css,
    Sql,
    Markdown,
    /// Fallback: highlight strings, comments, numbers
    Generic,
}

impl CodeLang {
    /// Detect language from a filename or extension
    pub fn from_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".rs") {
            CodeLang::Rust
        } else if lower.ends_with(".py") || lower.ends_with(".pyw") {
            CodeLang::Python
        } else if lower.ends_with(".js") || lower.ends_with(".mjs") || lower.ends_with(".cjs") {
            CodeLang::JavaScript
        } else if lower.ends_with(".ts") || lower.ends_with(".tsx") || lower.ends_with(".mts") {
            CodeLang::TypeScript
        } else if lower.ends_with(".go") {
            CodeLang::Go
        } else if lower.ends_with(".java") || lower.ends_with(".kt") || lower.ends_with(".scala") {
            CodeLang::Java
        } else if lower.ends_with(".c") || lower.ends_with(".h") {
            CodeLang::C
        } else if lower.ends_with(".cpp")
            || lower.ends_with(".cc")
            || lower.ends_with(".cxx")
            || lower.ends_with(".hpp")
            || lower.ends_with(".hh")
        {
            CodeLang::Cpp
        } else if lower.ends_with(".sh")
            || lower.ends_with(".bash")
            || lower.ends_with(".zsh")
            || lower.ends_with(".fish")
            || lower.ends_with(".ps1")
            || lower.ends_with(".bat")
        {
            CodeLang::Shell
        } else if lower.ends_with(".json") {
            CodeLang::Json
        } else if lower.ends_with(".toml") {
            CodeLang::Toml
        } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
            CodeLang::Yaml
        } else if lower.ends_with(".html")
            || lower.ends_with(".htm")
            || lower.ends_with(".xml")
            || lower.ends_with(".svg")
        {
            CodeLang::Html
        } else if lower.ends_with(".css") || lower.ends_with(".scss") || lower.ends_with(".less") {
            CodeLang::Css
        } else if lower.ends_with(".sql") || lower.ends_with(".psql") {
            CodeLang::Sql
        } else if lower.ends_with(".md") || lower.ends_with(".mdx") {
            CodeLang::Markdown
        } else {
            CodeLang::Generic
        }
    }

    /// Detect language from a tool name
    pub fn from_tool(tool_name: &str) -> Self {
        match tool_name {
            "run_python_code" => CodeLang::Python,
            "execute_shell_command" => CodeLang::Shell,
            "read_local_file" => CodeLang::Generic, // determined later from path
            "github_get_file" => CodeLang::Generic,
            _ => CodeLang::Generic,
        }
    }
}

/// Syntax highlighter for code snippets.
///
/// Applies ANSI color codes for:
/// - Keywords (language-specific)
/// - Strings (single, double, backtick)
/// - Comments (line and block)
/// - Numbers
/// - Booleans / nil / null
pub struct CodeColorizer;

impl CodeColorizer {
    /// Highlight a code snippet. Returns ANSI-colored string.
    /// `lang` determines the syntax rules; `max_lines` truncates output.
    pub fn highlight(code: &str, lang: CodeLang, max_lines: Option<usize>) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let total = lines.len();
        let show = max_lines.unwrap_or(total).min(total);
        let truncated = total > show;

        let mut out = String::with_capacity(code.len() + (code.len() / 4));
        for (i, line) in lines.iter().take(show).enumerate() {
            if i > 0 {
                out.push('\n');
            }
            Self::highlight_line(line, lang, &mut out);
        }
        if truncated {
            let _ = write!(out, "\n\x1b[2m... ({} more lines)\x1b[0m", total - show);
        }
        out
    }

    /// Highlight a single line
    fn highlight_line(line: &str, lang: CodeLang, out: &mut String) {
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];

            // ── Strings ────────────────────────────────────
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                out.push_str("\x1b[33m"); // yellow
                out.push(quote);
                i += 1;
                // Consume until matching close quote (handle escapes)
                while i < len {
                    if chars[i] == '\\' && i + 1 < len {
                        out.push(chars[i]);
                        out.push(chars[i + 1]);
                        i += 2;
                    } else if chars[i] == quote {
                        out.push(chars[i]);
                        i += 1;
                        break;
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                out.push_str("\x1b[0m");
                continue;
            }

            // ── Comments ───────────────────────────────────
            // Line comment: // or #
            if (c == '/' && i + 1 < len && chars[i + 1] == '/')
                || (c == '#' && lang != CodeLang::Toml)
            {
                out.push_str("\x1b[2m\x1b[37m"); // dim white
                while i < len {
                    out.push(chars[i]);
                    i += 1;
                }
                out.push_str("\x1b[0m");
                break;
            }
            // Block comment: /*
            if c == '/' && i + 1 < len && chars[i + 1] == '*' {
                out.push_str("\x1b[2m\x1b[37m"); // dim white
                out.push_str("/*");
                i += 2;
                while i < len {
                    if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                        out.push_str("*/");
                        i += 2;
                        break;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                out.push_str("\x1b[0m");
                continue;
            }
            // HTML comment: <!--
            if c == '<'
                && i + 3 < len
                && chars[i + 1] == '!'
                && chars[i + 2] == '-'
                && chars[i + 3] == '-'
            {
                out.push_str("\x1b[2m\x1b[37m");
                while i < len {
                    out.push(chars[i]);
                    i += 1;
                }
                out.push_str("\x1b[0m");
                break;
            }

            // ── Numbers ────────────────────────────────────
            if c.is_ascii_digit()
                && (i == 0 || !chars[i - 1].is_alphanumeric() || chars[i - 1] == '.')
            {
                out.push_str("\x1b[35m"); // magenta
                                          // Hex prefix
                if c == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                    out.push_str("0x");
                    i += 2;
                }
                while i < len
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || chars[i].is_ascii_alphabetic()
                            && i > 0
                            && chars[i - 1] == '0'
                            && (chars[i] == 'x' || chars[i] == 'X'))
                {
                    if chars[i].is_ascii_alphabetic()
                        && !(i > 0 && chars[i - 1] == '0' && (chars[i] == 'x' || chars[i] == 'X'))
                        && !(chars[i] == 'e' || chars[i] == 'E')
                    {
                        break;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                out.push_str("\x1b[0m");
                continue;
            }

            // ── Booleans / null / nil ──────────────────────
            let remaining: String = chars[i..].iter().collect();
            for kw in &["true", "false", "null", "nil", "None", "True", "False"] {
                if remaining.starts_with(kw)
                    && (i + kw.len() >= len
                        || !chars[i + kw.len()].is_alphanumeric() && chars[i + kw.len()] != '_')
                {
                    out.push_str("\x1b[35m"); // magenta
                    out.push_str(kw);
                    out.push_str("\x1b[0m");
                    i += kw.len();
                    // skip the outer increment
                    // we already advanced i, need to signal continue
                    // use a flag... instead just break and let loop continue
                    break;
                }
            }
            if i < len
                && (remaining.starts_with("true")
                    || remaining.starts_with("false")
                    || remaining.starts_with("null")
                    || remaining.starts_with("nil")
                    || remaining.starts_with("None")
                    || remaining.starts_with("True")
                    || remaining.starts_with("False"))
                && (i > 0 && !chars[i - 1].is_alphanumeric() || i == 0)
            {
                // already handled by logic above, but the break broke us out of the inner loop
                // We need to detect if we consumed chars. Simpler: just check and consume manually.
                continue; // already handled
            }

            // ── Keywords (language-specific) ───────────────
            let kw_matched = Self::try_keyword(&chars, i, len, lang);
            if let Some(kw_len) = kw_matched {
                out.push_str("\x1b[34m"); // blue
                if let Some(slice) = chars.get(i..i + kw_len) {
                    for &ch in slice {
                        out.push(ch);
                    }
                }
                out.push_str("\x1b[0m");
                i += kw_len;
                continue;
            }

            // ── Plain char ─────────────────────────────────
            out.push(c);
            i += 1;
        }
    }

    /// Try to match a keyword at position. Returns length if matched.
    fn try_keyword(chars: &[char], i: usize, len: usize, lang: CodeLang) -> Option<usize> {
        let remaining: String = chars[i..].iter().collect();

        for kw in super::keywords::get_keywords(lang) {
            if remaining.starts_with(*kw) {
                let kw_len = kw.len();
                // Must be at word boundary
                if i + kw_len >= len
                    || !chars[i + kw_len].is_alphanumeric() && chars[i + kw_len] != '_'
                {
                    // Must start at word boundary too
                    if i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_' {
                        return Some(kw_len);
                    }
                }
            }
        }
        None
    }
}

// ── Helper: Shorten a string for display ────────────────────────

/// Truncate a result string for TUI display, keeping it readable.
pub fn truncate_result(result: &str, max_chars: usize) -> String {
    let result = result.trim();
    if result.len() <= max_chars {
        return result.to_string();
    }
    // Try to break at a newline
    let truncate_at = result
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(result.len());
    let truncated = &result[..truncate_at];
    if let Some(last_nl) = truncated.rfind('\n') {
        if last_nl > max_chars / 2 {
            return format!(
                "{}\n\x1b[2m... (truncated, {} total chars)\x1b[0m",
                &result[..last_nl],
                result.len()
            );
        }
    }
    format!(
        "{}\x1b[2m... ({} total chars)\x1b[0m",
        truncated,
        result.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_code() {
        let mut c = StreamColorizer::new();
        let out = c.feed("Use `cargo build` to compile.");
        // Should have green backticks around cargo build
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

    // ── CodeColorizer tests ──

    #[test]
    fn test_code_rust_keywords() {
        let code = "fn main() {\n    let x = 42;\n}";
        let colored = CodeColorizer::highlight(code, CodeLang::Rust, None);
        // fn and let should be blue
        assert!(colored.contains("\x1b[34mfn\x1b[0m"));
        assert!(colored.contains("\x1b[34mlet\x1b[0m"));
        // 42 should be magenta (number)
        assert!(colored.contains("\x1b[35m42\x1b[0m"));
    }

    #[test]
    fn test_code_string() {
        let code = r#"let s = "hello world";"#;
        let colored = CodeColorizer::highlight(code, CodeLang::Rust, None);
        // String should be yellow
        assert!(colored.contains("\x1b[33m\"hello world\"\x1b[0m"));
    }

    #[test]
    fn test_code_comment() {
        let code = "// this is a comment\nlet x = 1;";
        let colored = CodeColorizer::highlight(code, CodeLang::Rust, None);
        // Comment should be dim
        assert!(colored.contains("\x1b[2m\x1b[37m"));
        assert!(colored.contains("this is a comment"));
    }

    #[test]
    fn test_code_python() {
        let code = "def hello():\n    return 'world'";
        let colored = CodeColorizer::highlight(code, CodeLang::Python, None);
        assert!(colored.contains("\x1b[34mdef\x1b[0m"));
        assert!(colored.contains("\x1b[34mreturn\x1b[0m"));
        assert!(colored.contains("\x1b[33m'world'\x1b[0m"));
    }

    #[test]
    fn test_code_shell() {
        let code = "echo 'hello' && ls -la";
        let colored = CodeColorizer::highlight(code, CodeLang::Shell, None);
        assert!(colored.contains("\x1b[33m'hello'\x1b[0m"));
    }

    #[test]
    fn test_code_truncation() {
        let code = "line1\nline2\nline3\nline4\nline5";
        let colored = CodeColorizer::highlight(code, CodeLang::Generic, Some(3));
        assert!(colored.contains("line1"));
        assert!(colored.contains("line3"));
        assert!(colored.contains("more lines"));
        assert!(!colored.contains("line5"));
    }

    #[test]
    fn test_lang_from_path() {
        assert_eq!(CodeLang::from_path("src/main.rs"), CodeLang::Rust);
        assert_eq!(CodeLang::from_path("app.py"), CodeLang::Python);
        assert_eq!(CodeLang::from_path("script.sh"), CodeLang::Shell);
        assert_eq!(CodeLang::from_path("config.json"), CodeLang::Json);
        assert_eq!(CodeLang::from_path("Cargo.toml"), CodeLang::Toml);
        assert_eq!(CodeLang::from_path("app.js"), CodeLang::JavaScript);
        assert_eq!(CodeLang::from_path("app.ts"), CodeLang::TypeScript);
        assert_eq!(CodeLang::from_path("main.go"), CodeLang::Go);
        assert_eq!(CodeLang::from_path("unknown.xyz"), CodeLang::Generic);
    }

    #[test]
    fn test_truncate_result() {
        let short = "hello";
        assert_eq!(truncate_result(short, 100), "hello");

        let long = "a".repeat(200);
        let truncated = truncate_result(&long, 50);
        assert!(truncated.len() <= 120); // Should be roughly 50 chars + marker
    }
}
