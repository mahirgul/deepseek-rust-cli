use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub fn highlight_code(code: &str, lang: &str) -> String {
    let ps = SyntaxSet::load_defaults_nonewlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_token(lang)
        .or_else(|| ps.find_syntax_by_extension(lang))
        .unwrap_or_else(|| ps.find_syntax_plain_text());
    
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let mut output = String::new();

    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        output.push_str(&escaped);
    }
    output.push_str("\x1b[0m"); // Reset at end
    output
}

pub fn print_highlighted_markdown(text: &str) {
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut lang = String::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                println!("{}", highlight_code(&code_buffer, &lang));
                code_buffer.clear();
                in_code_block = false;
            } else {
                // Start of code block
                lang = line.trim_start_matches('`').trim().to_string();
                if lang.is_empty() { lang = "text".to_string(); }
                in_code_block = true;
            }
        } else if in_code_block {
            code_buffer.push_str(line);
            code_buffer.push('\n');
        } else {
            // Normal markdown line
            println!("{}", line);
        }
    }
}
