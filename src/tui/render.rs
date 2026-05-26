use crate::{
    tui::{
        app::App,
        utils::{strip_ansi, truncate_ansi_str, truncate_str},
    },
    version::VERSION,
};
use crossterm::{
    cursor, execute,
    style::{self, Stylize},
    terminal, QueueableCommand,
};
use std::io::{self, Write};

pub fn render_footer(stdout: &mut io::Stdout, app: &App) -> io::Result<()> {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
    let fh = app.footer_height; // always 4

    stdout.queue(cursor::Hide)?;

    // ── Line 1 (top of footer): Status ──────────────────────────────
    let line1_y = term_height.saturating_sub(fh);
    stdout.queue(cursor::MoveTo(0, line1_y))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;

    let spinner_chars = vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner = if app.current_task.is_some() || app.awaiting_approval {
        spinner_chars[app.spinner_frame % spinner_chars.len()]
            .to_string()
            .yellow()
            .to_string()
    } else {
        "✨".to_string()
    };

    let status = if app.awaiting_approval {
        if app.is_path_traversal_warning {
            " ⚠️ AWAITING APPROVAL (y/n) ".red().to_string()
        } else {
            " ⚠️ AWAITING APPROVAL (y/n/a) ".red().to_string()
        }
    } else if let Some(task) = &app.current_task {
        let elapsed = app
            .job_start_time
            .map(|s| format!(" ({:.1}s)", s.elapsed().as_secs_f32()))
            .unwrap_or_default();
        format!(" {}...{} ", task, elapsed).blue().to_string()
    } else {
        format!(" {} ", app.model).magenta().to_string()
    };

    let line1 = format!("v{} {}{}", VERSION, spinner, status);
    stdout.queue(style::Print(line1))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // ── Line 2: Folder + Token info ─────────────────────────────────
    stdout.queue(cursor::MoveTo(0, term_height.saturating_sub(fh - 1)))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;

    let total_tokens = app.total_tokens();
    let token_str = if total_tokens > 0 {
        format!(
            " | 📊 {} prompt · {} comp · {} total",
            app.token_usage.prompt_tokens, app.token_usage.completion_tokens, total_tokens
        )
    } else {
        String::new()
    };

    let cwd_visible = format!("📂 {} ", app.cwd);
    let token_visible_len = strip_ansi(&token_str).chars().count();
    let cwd_visible_len = cwd_visible.chars().count();
    let max_cwd_len = (term_width as usize).saturating_sub(token_visible_len + 2);

    let cwd_display = if cwd_visible_len > max_cwd_len && max_cwd_len > 3 {
        format!(
            "📂 ...{} ",
            &app.cwd[app.cwd.len().saturating_sub(max_cwd_len - 6)..]
        )
    } else {
        cwd_visible
    };

    let line2 = format!("{}{}", cwd_display.blue(), token_str.dim());
    stdout.queue(style::Print(line2))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // ── Line 3: Input prompt ────────────────────────────────────────
    let line3_y = term_height.saturating_sub(2);
    stdout.queue(cursor::MoveTo(0, line3_y))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;

    let prompt = "> ";
    let avail = (term_width as usize).saturating_sub(3); // "> " + 1 char margin
    let input_display = if app.input.chars().count() <= avail || avail == 0 {
        app.input.clone()
    } else {
        // Show tail portion near cursor
        let skip = app.input.chars().count().saturating_sub(avail);
        app.input.chars().skip(skip).collect()
    };
    let line3 = format!("{}{}", prompt.cyan(), input_display);
    stdout.queue(style::Print(line3))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // Cursor X: prompt width + cursor char offset (relative to visible portion)
    let visible_input_chars = app.input.chars().count();
    let visible_offset = if visible_input_chars > avail && avail > 0 {
        visible_input_chars.saturating_sub(avail)
    } else {
        0
    };
    let cursor_byte_pos = app.cursor_pos.min(app.input.len());
    let safe_cursor_pos = if app.input.is_char_boundary(cursor_byte_pos) {
        cursor_byte_pos
    } else {
        let mut p = cursor_byte_pos;
        while p > 0 && !app.input.is_char_boundary(p) {
            p -= 1;
        }
        p
    };
    let cursor_char = app.input[..safe_cursor_pos].chars().count();
    let cursor_x = 2 + ((cursor_char.saturating_sub(visible_offset)) as u16);

    // ── Line 4 (bottom): Queue entries horizontal ───────────────────
    let line4_y = term_height.saturating_sub(1);
    stdout.queue(cursor::MoveTo(0, line4_y))?;
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;

    if !app.queued_commands.is_empty() {
        // Build queue display: "q1: cmd1  q2: cmd2  ..."
        let mut parts: Vec<String> = Vec::new();
        let separator = "  ";

        // Estimate max entries that fit on one line
        let max_entries = (term_width as usize / 15).max(1);

        for i in 0..app.queued_commands.len().min(max_entries) {
            if i > 0 {
                parts.push(separator.to_string());
            }
            let cmd = &app.queued_commands[i];
            let prefix = if i == 0 && app.current_task.is_some() {
                format!("▶ q{}:", i + 1)
            } else if i == 0 {
                format!("✓ q{}:", i + 1)
            } else {
                format!("q{}:", i + 1)
            };
            let prefix_len = prefix.chars().count();
            let cmd_max = 30usize.saturating_sub(prefix_len);
            let truncated_cmd = truncate_str(cmd, cmd_max);

            // Styled entry
            let entry: String = if i == 0 && app.current_task.is_some() {
                format!("{}{}", prefix.green(), truncated_cmd)
            } else if i == 0 {
                format!("{}{}", prefix.dim(), truncated_cmd.dim())
            } else {
                format!("{}{}", prefix.yellow(), truncated_cmd.dim())
            };
            parts.push(entry);
        }

        let queue_line = parts.join("");
        // Truncate to terminal width (account for ANSI codes properly)
        let truncated = truncate_ansi_str(&queue_line, term_width as usize);
        stdout.queue(style::Print(truncated))?;
    }

    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // Reset styles
    stdout.queue(style::SetBackgroundColor(style::Color::Reset))?;
    stdout.queue(style::ResetColor)?;

    // Position cursor on the input line
    stdout.queue(cursor::MoveTo(cursor_x, line3_y))?;
    stdout.queue(cursor::Show)?;
    stdout.flush()?;

    Ok(())
}

pub fn write_to_output(stdout: &mut io::Stdout, app: &mut App, text: String) -> io::Result<()> {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
    let log_height = term_height.saturating_sub(app.footer_height);
    let max_cols = term_width;

    // Move to the current log position
    stdout.queue(cursor::MoveTo(app.log_x, app.log_y))?;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // It's an ANSI escape sequence! Print it directly and skip.
            let mut seq = String::new();
            seq.push('\x1b');
            seq.push('[');
            i += 2;
            while i < chars.len() {
                let c = chars[i];
                seq.push(c);
                i += 1;
                // ANSI escape sequences end with a letter in the range @ to ~ (ASCII 64-126)
                if (c as u32) >= 64 && (c as u32) <= 126 {
                    break;
                }
            }
            execute!(stdout, style::Print(seq))?;
        } else if chars[i] == '\n' {
            // Newline: print CR LF to move to start of next line
            execute!(stdout, style::Print("\r\n"))?;
            app.log_x = 0;
            if app.log_y < log_height.saturating_sub(1) {
                app.log_y += 1;
            }
            i += 1;
        } else if chars[i] == '\r' {
            // Carriage return: reset log_x
            app.log_x = 0;
            i += 1;
        } else {
            // Normal character
            if app.log_x >= max_cols {
                execute!(stdout, style::Print("\r\n"))?;
                app.log_x = 0;
                if app.log_y < log_height.saturating_sub(1) {
                    app.log_y += 1;
                }
            }
            execute!(stdout, style::Print(chars[i]))?;
            app.log_x += 1;
            i += 1;
        }
    }

    stdout.flush()?;
    Ok(())
}
