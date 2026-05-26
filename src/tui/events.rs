use std::io;

use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyEventKind},
    style::Stylize,
};

use crate::{
    agent::types::{AgentEvent, ApprovalResult},
    tui::{
        app::{save_global_history, App},
        colorizer::{CodeColorizer, StreamColorizer},
        event_loop::EventLoop,
        render::write_to_output,
        utils::{detect_lang_for_result, format_tool_args},
    },
};

impl EventLoop {
    pub fn handle_input(
        &self,
        app: &mut App,
        stdout: &mut io::Stdout,
        key: crossterm::event::KeyEvent,
    ) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }

        if app.awaiting_approval {
            if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('C'))
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                return Ok(true);
            }
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.awaiting_approval = false;
                    app.current_task = None;
                    write_to_output(stdout, app, "✅ Approved\n".green().to_string())?;
                    let _ = self.app_tx.try_send(ApprovalResult::Yes);
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.awaiting_approval = false;
                    app.current_task = None;
                    write_to_output(stdout, app, "❌ Rejected\n".red().to_string())?;
                    let _ = self.app_tx.try_send(ApprovalResult::No);
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if app.is_path_traversal_warning {
                        return Ok(false);
                    }
                    app.awaiting_approval = false;
                    app.current_task = None;
                    write_to_output(stdout, app, "🛡️ Always Approved\n".blue().to_string())?;
                    let _ = self.app_tx.try_send(ApprovalResult::Always);
                }
                _ => {}
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Enter if !app.input.is_empty() => {
                let cmd = app.input.clone();
                app.reasoning_started = false;
                app.content_started = false;
                let separator = format!("\n{}\n", "────────────────────────────────────────────────────────────────────────────────".dim());
                let prompt = format!("> {}\n", cmd).cyan().to_string();
                write_to_output(stdout, app, format!("{}{}", separator, prompt))?;

                if cmd == "exit" || cmd == "quit" || cmd == "/exit" || cmd == "/quit" {
                    return Ok(true);
                }
                if app.history.last() != Some(&cmd) {
                    app.history.push(cmd.clone());
                    if app.history.len() > 1000 {
                        app.history.remove(0);
                    }
                    save_global_history(&app.history);
                }
                app.history_index = None;
                app.aborted = false;
                app.queued_commands.push(cmd.clone());
                let current_run_id = self.run_id.load(std::sync::atomic::Ordering::SeqCst);
                let _ = self.cmd_tx.try_send((current_run_id, cmd));
                app.input.clear();
                app.cursor_pos = 0;
            }
            KeyCode::Char('c') | KeyCode::Char('C')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                return Ok(true);
            }
            KeyCode::Char(c) => {
                let byte_pos = app.cursor_pos.min(app.input.len());
                app.input.insert(byte_pos, c);
                app.cursor_pos = byte_pos + c.len_utf8();
            }
            KeyCode::Backspace if app.cursor_pos > 0 => {
                let mut prev = app.cursor_pos - 1;
                while prev > 0 && !app.input.is_char_boundary(prev) {
                    prev -= 1;
                }
                app.input.replace_range(prev..app.cursor_pos, "");
                app.cursor_pos = prev;
            }
            KeyCode::Delete if app.cursor_pos < app.input.len() => {
                let mut next = app.cursor_pos + 1;
                while next < app.input.len() && !app.input.is_char_boundary(next) {
                    next += 1;
                }
                app.input.replace_range(app.cursor_pos..next, "");
            }
            KeyCode::Left if app.cursor_pos > 0 => {
                let mut prev = app.cursor_pos - 1;
                while prev > 0 && !app.input.is_char_boundary(prev) {
                    prev -= 1;
                }
                app.cursor_pos = prev;
            }
            KeyCode::Right if app.cursor_pos < app.input.len() => {
                let mut next = app.cursor_pos + 1;
                while next < app.input.len() && !app.input.is_char_boundary(next) {
                    next += 1;
                }
                app.cursor_pos = next;
            }
            KeyCode::Home => {
                app.cursor_pos = 0;
            }
            KeyCode::End => {
                app.cursor_pos = app.input.len();
            }
            KeyCode::Up => {
                app.next_history();
            }
            KeyCode::Down => {
                app.prev_history();
            }
            _ => {}
        }
        Ok(false)
    }

    pub fn handle_agent_event(
        &self,
        app: &mut App,
        stdout: &mut io::Stdout,
        agent_event: AgentEvent,
        full_message: &mut String,
        reasoning_colorizer: &mut StreamColorizer,
        content_colorizer: &mut StreamColorizer,
    ) -> Result<()> {
        if app.aborted {
            match &agent_event {
                AgentEvent::Aborted { token_usage } | AgentEvent::Done { token_usage } => {
                    let flush = reasoning_colorizer.finish();
                    if !flush.is_empty() {
                        write_to_output(stdout, app, flush)?;
                    }
                    let flush = content_colorizer.finish();
                    if !flush.is_empty() {
                        write_to_output(stdout, app, flush)?;
                    }
                    app.token_usage = token_usage.clone();
                    app.finish_task();
                }
                _ => return Ok(()),
            }
            return Ok(());
        }

        match agent_event {
            AgentEvent::Reasoning { content } => {
                app.start_task("Reasoning".to_string());
                if !content.is_empty() {
                    if !app.reasoning_started {
                        let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                        let header = "🧠 Thinking Process:\n".yellow().italic().to_string();
                        write_to_output(stdout, app, format!("\n{}\n{}", separator, header))?;
                        app.reasoning_started = true;
                        app.content_started = false;
                    }
                    let colored = reasoning_colorizer.feed(&content);
                    write_to_output(stdout, app, colored)?;
                }
            }
            AgentEvent::Content { content } => {
                app.start_task("Generating".to_string());
                full_message.push_str(&content);
                if !content.is_empty() {
                    if !app.content_started {
                        let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                        let header = "💬 Response:\n".cyan().bold().to_string();
                        write_to_output(stdout, app, format!("\n{}\n{}", separator, header))?;
                        app.content_started = true;
                        app.reasoning_started = false;
                    }
                    let colored = content_colorizer.feed(&content);
                    write_to_output(stdout, app, colored)?;
                }
            }
            AgentEvent::ToolStart { name, args } => {
                app.start_task(format!("Tool: {}", name));
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                let formatted_args = format_tool_args(&name, &args);
                write_to_output(
                    stdout,
                    app,
                    format!(
                        "\n{}\n🔧 {} \n{}\n",
                        separator,
                        name.cyan().bold(),
                        formatted_args.dim()
                    ),
                )?;
            }
            AgentEvent::ToolEnd { name, result } => {
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                if let Some(ref res) = result {
                    let lang = detect_lang_for_result(&name, res);
                    let max_lines = if name == "read_local_file" || name == "execute_shell_command"
                    {
                        Some(20)
                    } else {
                        Some(10)
                    };
                    let colored_result = CodeColorizer::highlight(res, lang, max_lines);
                    write_to_output(
                        stdout,
                        app,
                        format!(
                            "\n{}\n✅ {} executed:\n{}\n",
                            separator,
                            name.green().bold(),
                            colored_result
                        ),
                    )?;
                } else {
                    write_to_output(
                        stdout,
                        app,
                        format!("\n{}\n✅ {} executed.\n", separator, name.green().bold()),
                    )?;
                }
            }
            AgentEvent::ApprovalRequest { name, args } => {
                app.start_task("Awaiting Approval".to_string());
                app.awaiting_approval = true;
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();

                let (display_name, is_traversal) =
                    if let Some(stripped) = name.strip_prefix("path_traversal_warning:") {
                        (stripped.to_string(), true)
                    } else {
                        (name.clone(), false)
                    };

                app.is_path_traversal_warning = is_traversal;
                let header = if is_traversal {
                    format!(
                        "⚠️ WARNING: Path traversal detected for tool: {}\n",
                        display_name
                    )
                    .red()
                    .bold()
                    .to_string()
                } else {
                    format!("⚠️ Approval Required for tool: {}\n", display_name)
                        .yellow()
                        .to_string()
                };

                write_to_output(stdout, app, format!("\n{}\n{}", separator, header))?;
                write_to_output(
                    stdout,
                    app,
                    format!("Arguments: {}\n", args).dim().to_string(),
                )?;

                let prompt_str = if is_traversal {
                    "? Press 'y' to approve this path traversal, 'n' to reject. (Always Approve is \
                     disabled for security)\n"
                        .red()
                        .to_string()
                } else {
                    "? Press 'y' to approve, 'n' to reject, 'a' to allow all.\n"
                        .red()
                        .to_string()
                };
                write_to_output(stdout, app, prompt_str)?;
            }
            AgentEvent::Error { content } => {
                let flush = reasoning_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                let flush = content_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                app.finish_task();
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                write_to_output(
                    stdout,
                    app,
                    format!("\n{}\n❌ Error: {}\n", separator, content)
                        .red()
                        .to_string(),
                )?;
            }
            AgentEvent::Done { token_usage } => {
                let flush = reasoning_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                let flush = content_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                app.token_usage = token_usage;
                app.finish_task();
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                write_to_output(
                    stdout,
                    app,
                    format!("\n{}\n✅ Operation Complete\n", separator)
                        .green()
                        .to_string(),
                )?;
                full_message.clear();
            }
            AgentEvent::Aborted { token_usage } => {
                let flush = reasoning_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                let flush = content_colorizer.finish();
                if !flush.is_empty() {
                    write_to_output(stdout, app, flush)?;
                }
                app.token_usage = token_usage;
                app.finish_task();
                app.reasoning_started = false;
                app.content_started = false;
                let separator = "────────────────────────────────────────────────────────────────────────────────".dim().to_string();
                write_to_output(
                    stdout,
                    app,
                    format!("\n{}\n🛑 Operation aborted by user.\n", separator).to_string(),
                )?;
            }
        }
        Ok(())
    }
}
