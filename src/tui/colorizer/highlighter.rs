use crate::tui::colorizer::types::CodeLang;
use std::fmt::Write as FmtWrite;

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
            let mut matched_bool = false;
            for kw in &["true", "false", "null", "nil", "None", "True", "False"] {
                if remaining.starts_with(kw)
                    && (i + kw.len() >= len
                        || !chars[i + kw.len()].is_alphanumeric() && chars[i + kw.len()] != '_')
                {
                    out.push_str("\x1b[35m"); // magenta
                    out.push_str(kw);
                    out.push_str("\x1b[0m");
                    i += kw.len();
                    matched_bool = true;
                    break;
                }
            }
            if matched_bool {
                continue;
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

        for kw in crate::tui::keywords::get_keywords(lang) {
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
