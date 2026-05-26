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
