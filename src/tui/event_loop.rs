use std::{
    io::{self, Write},
    sync::Arc,
    time::Instant,
};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, DisableBracketedPaste, EnableBracketedPaste, KeyCode, KeyEventKind},
    execute,
    style::{self, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use tokio::sync::{mpsc, Mutex};

use crate::{
    agent::agent::{AgentEvent, ApprovalResult, DeepSeekAgent},
    api::types::TokenUsage,
    tui::colorizer::{CodeColorizer, CodeLang, StreamColorizer},
};

pub enum TuiEvent {
    Input(event::KeyEvent),
    Mouse(event::MouseEvent),
    /// Bracketed paste content (multi-line preserved)
    Paste(String),
    Tick,
    Agent(AgentEvent),
    Abort,
}

fn load_global_history() -> Vec<String> {
    let path = std::path::PathBuf::from(".deep/input_history.json");
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(history) = serde_json::from_str::<Vec<String>>(&content) {
            return history;
        }
    }
    Vec::new()
}

fn save_global_history(history: &[String]) {
    let path = std::path::PathBuf::from(".deep/input_history.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = std::fs::write(path, json);
    }
}

struct App {
    input: String,
    /// Byte position of cursor within input (0 = start)
    cursor_pos: usize,
    awaiting_approval: bool,
    spinner_frame: usize,
    current_task: Option<String>,
    /// When the current *task label* last changed (for display only)
    task_start_time: Option<Instant>,
    /// When the entire agent job started (never reset until finish_task)
    job_start_time: Option<Instant>,
    cwd: String,
    model: String,
    history: Vec<String>,
    history_index: Option<usize>,
    /// Footer is always 4 lines: status, folder+token, input, queue
    footer_height: u16,
    token_usage: TokenUsage,
    /// When true, ignore incoming AgentEvents (after abort)
    aborted: bool,
    /// Pending commands that were sent but not yet completed.
    /// Index 0 is the currently-running command; the rest are queued.
    queued_commands: Vec<String>,
}

impl App {
    fn new() -> Self {
        let history = load_global_history();
        Self {
            input: String::new(),
            cursor_pos: 0,
            awaiting_approval: false,
            spinner_frame: 0,
            current_task: None,
            task_start_time: None,
            job_start_time: None,
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            model: String::from("unknown"),
            history,
            history_index: None,
            footer_height: 4, // Fixed: Status, Folder+Token, Input, Queue
            token_usage: TokenUsage::default(),
            aborted: false,
            queued_commands: Vec::new(),
        }
    }

    fn next_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(0)
                }
            }
            None => Some(self.history.len().saturating_sub(1)),
        };
        if let Some(idx) = next_index {
            self.history_index = Some(idx);
            self.input = self.history[idx].clone();
            self.cursor_pos = self.input.len();
        }
    }

    fn prev_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(i) => {
                if i < self.history.len().saturating_sub(1) {
                    Some(i + 1)
                } else {
                    self.input.clear();
                    self.cursor_pos = 0;
                    None
                }
            }
            None => None,
        };
        self.history_index = next_index;
        if let Some(idx) = self.history_index {
            self.input = self.history[idx].clone();
            self.cursor_pos = self.input.len();
        }
    }

    fn start_task(&mut self, task: String) {
        // Set job-wide timer once at the beginning of the agent job
        if self.job_start_time.is_none() {
            self.job_start_time = Some(Instant::now());
        }
        if self.current_task.as_ref() != Some(&task) {
            self.current_task = Some(task);
            self.task_start_time = Some(Instant::now());
        }
    }

    fn finish_task(&mut self) {
        self.current_task = None;
        self.task_start_time = None;
        self.job_start_time = None;
        self.awaiting_approval = false;
        self.aborted = false;
        // Pop the just-completed command from queue
        if !self.queued_commands.is_empty() {
            self.queued_commands.remove(0);
        }
    }

    fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Total tokens used so far
    fn total_tokens(&self) -> u64 {
        self.token_usage.prompt_tokens + self.token_usage.completion_tokens
    }
}

pub struct EventLoop {
    rx: mpsc::Receiver<TuiEvent>,
    rx_tx: mpsc::Sender<TuiEvent>,
    app_tx: mpsc::Sender<ApprovalResult>,
    cmd_tx: mpsc::Sender<String>,
    agent: Arc<Mutex<DeepSeekAgent>>,
    /// Shared cancel token — can be cancelled without locking the agent mutex
    cancel_token: Arc<std::sync::Mutex<tokio_util::sync::CancellationToken>>,
}

impl EventLoop {
    pub fn new(
        rx: mpsc::Receiver<TuiEvent>,
        rx_tx: mpsc::Sender<TuiEvent>,
        app_tx: mpsc::Sender<ApprovalResult>,
        cmd_tx: mpsc::Sender<String>,
        agent: Arc<Mutex<DeepSeekAgent>>,
    ) -> Self {
        // Clone the cancel token from the agent (brief lock)
        let cancel_token = agent
            .try_lock()
            .map(|a| a.cancel_token.clone())
            .unwrap_or_else(|_| {
                Arc::new(std::sync::Mutex::new(
                    tokio_util::sync::CancellationToken::new(),
                ))
            });
        Self {
            rx,
            rx_tx,
            app_tx,
            cmd_tx,
            agent,
            cancel_token,
        }
    }

    pub async fn run(mut self) -> Result<String> {
        let mut full_message = String::new();
        let mut app = App::new();
        let mut reasoning_colorizer = StreamColorizer::new();
        let mut content_colorizer = StreamColorizer::new();

        {
            if let Ok(agent) = self.agent.try_lock() {
                app.model = agent.model.clone();
                app.token_usage = agent.token_usage.clone();
            }
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        // Enable bracketed paste so multi-line pastes come as a single event
        execute!(stdout, EnableBracketedPaste)?;

        // Initial setup: Clear and set scrolling region
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
        let log_height = term_height.saturating_sub(app.footer_height);
        execute!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
            // CSI <top>;<bottom>r set scrolling region (1-indexed)
            style::Print(format!("\x1b[1;{}r", log_height)),
            cursor::MoveTo(0, 0),
            cursor::SavePosition // Initial Log Position
        )?;

        let mut last_size = (term_width, term_height);
        let mut last_footer_height = app.footer_height; // always 4
        render_footer(&mut stdout, &app)?;

        while let Some(event) = self.rx.recv().await {
            match event {
                TuiEvent::Abort => {
                    // Cancel via shared token — no agent lock needed, avoids deadlock
                    self.cancel_token.lock().unwrap().cancel();
                    // Set aborted flag so we ignore any in-flight AgentEvents
                    app.aborted = true;
                    app.current_task = None;
                    app.task_start_time = None;
                    app.job_start_time = None;
                    app.awaiting_approval = false;
                    write_to_output(&mut stdout, "🛑 Operation aborted by user.\n".to_string())?;
                }
                TuiEvent::Paste(text) => {
                    // Insert pasted text at cursor position (preserving newlines)
                    if !text.is_empty() {
                        let byte_pos = app.cursor_pos.min(app.input.len());
                        app.input.insert_str(byte_pos, &text);
                        app.cursor_pos = byte_pos + text.len();
                    }
                }
                TuiEvent::Mouse(_) => {}
                TuiEvent::Input(key) => {
                    if key.kind == KeyEventKind::Press {
                        if app.awaiting_approval {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    app.finish_task();
                                    write_to_output(
                                        &mut stdout,
                                        "✅ Approved\n".green().to_string(),
                                    )?;
                                    let _ = self.app_tx.send(ApprovalResult::Yes).await;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    app.finish_task();
                                    write_to_output(
                                        &mut stdout,
                                        "❌ Rejected\n".red().to_string(),
                                    )?;
                                    let _ = self.app_tx.send(ApprovalResult::No).await;
                                }
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    app.finish_task();
                                    write_to_output(
                                        &mut stdout,
                                        "🛡️ Always Approved\n".blue().to_string(),
                                    )?;
                                    let _ = self.app_tx.send(ApprovalResult::Always).await;
                                }
                                KeyCode::Esc => {
                                    let _ = self.rx_tx.send(TuiEvent::Abort).await;
                                }
                                _ => {}
                            }
                            continue;
                        }
                        match key.code {
                            KeyCode::Enter if !app.input.is_empty() => {
                                let cmd = app.input.clone();
                                write_to_output(
                                    &mut stdout,
                                    format!("> {}\n", cmd).cyan().to_string(),
                                )?;

                                if cmd == "exit"
                                    || cmd == "quit"
                                    || cmd == "/exit"
                                    || cmd == "/quit"
                                {
                                    break;
                                }
                                if app.history.last() != Some(&cmd) {
                                    app.history.push(cmd.clone());
                                    if app.history.len() > 1000 {
                                        app.history.remove(0);
                                    }
                                    save_global_history(&app.history);
                                }
                                app.history_index = None;
                                // Reset aborted flag for new command
                                app.aborted = false;
                                // Track in queue
                                app.queued_commands.push(cmd.clone());
                                let _ = self.cmd_tx.send(cmd).await;
                                app.input.clear();
                                app.cursor_pos = 0;
                            }
                            KeyCode::Char('c') | KeyCode::Char('C')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                            {
                                break;
                            }
                            KeyCode::Char(c) => {
                                let byte_pos = app.cursor_pos.min(app.input.len());
                                app.input.insert(byte_pos, c);
                                app.cursor_pos = byte_pos + c.len_utf8();
                            }
                            KeyCode::Backspace if app.cursor_pos > 0 => {
                                // Find the byte start of the char before cursor
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
                            KeyCode::Esc => {
                                let _ = self.rx_tx.send(TuiEvent::Abort).await;
                            }
                            _ => {}
                        }
                    }
                }
                TuiEvent::Agent(agent_event) => {
                    // If aborted, ignore all agent events except Aborted/Done which confirm
                    // termination
                    if app.aborted {
                        match &agent_event {
                            AgentEvent::Aborted { token_usage }
                            | AgentEvent::Done { token_usage } => {
                                // Flush colorizers
                                let flush = reasoning_colorizer.finish();
                                if !flush.is_empty() {
                                    write_to_output(&mut stdout, flush)?;
                                }
                                let flush = content_colorizer.finish();
                                if !flush.is_empty() {
                                    write_to_output(&mut stdout, flush)?;
                                }
                                // Update token usage from event
                                app.token_usage = token_usage.clone();
                                app.finish_task();
                                if matches!(agent_event, AgentEvent::Aborted { .. }) {
                                    write_to_output(
                                        &mut stdout,
                                        "🛑 Operation aborted by user.\n".to_string(),
                                    )?;
                                }
                            }
                            _ => {
                                // Silently ignore in-flight events after abort
                                continue;
                            }
                        }
                        render_footer(&mut stdout, &app)?;
                        continue;
                    }

                    match agent_event {
                        AgentEvent::Reasoning { content } => {
                            app.start_task("Reasoning".to_string());
                            if !content.is_empty() {
                                let colored = reasoning_colorizer.feed(&content);
                                write_to_output(&mut stdout, colored)?;
                            }
                        }
                        AgentEvent::Content { content } => {
                            app.start_task("Generating".to_string());
                            full_message.push_str(&content);
                            let colored = content_colorizer.feed(&content);
                            write_to_output(&mut stdout, colored)?;
                        }
                        AgentEvent::ToolStart { name, args } => {
                            app.start_task(format!("Tool: {}", name));
                            // Pretty-print JSON args
                            let formatted_args = format_tool_args(&name, &args);
                            write_to_output(
                                &mut stdout,
                                format!("🔧 {} \n{}\n", name.cyan().bold(), formatted_args.dim())
                                    .to_string(),
                            )?;
                        }
                        AgentEvent::ToolEnd { name, result } => {
                            // Display result with syntax highlighting if present
                            if let Some(ref res) = result {
                                let lang = detect_lang_for_result(&name, res);
                                let max_lines = if name == "read_local_file"
                                    || name == "execute_shell_command"
                                {
                                    Some(20)
                                } else {
                                    Some(10)
                                };
                                let colored_result = CodeColorizer::highlight(res, lang, max_lines);
                                write_to_output(
                                    &mut stdout,
                                    format!(
                                        "✅ {} executed:\n{}\n",
                                        name.green().bold(),
                                        colored_result
                                    ),
                                )?;
                            } else {
                                write_to_output(
                                    &mut stdout,
                                    format!("✅ {} executed.\n", name).green().to_string(),
                                )?;
                            }
                        }
                        AgentEvent::ApprovalRequest { name, args } => {
                            app.start_task("Awaiting Approval".to_string());
                            app.awaiting_approval = true;
                            write_to_output(
                                &mut stdout,
                                format!("⚠️ Approval Required for tool: {}\n", name)
                                    .yellow()
                                    .to_string(),
                            )?;
                            write_to_output(
                                &mut stdout,
                                format!("Arguments: {}\n", args).dim().to_string(),
                            )?;
                            write_to_output(
                                &mut stdout,
                                "? Press 'y' to approve, 'n' to reject, 'a' to allow all.\n"
                                    .red()
                                    .to_string(),
                            )?;
                        }
                        AgentEvent::Error { content } => {
                            // Flush any pending colorizer output
                            let flush = reasoning_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            let flush = content_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            app.finish_task();
                            write_to_output(
                                &mut stdout,
                                format!("❌ Error: {}\n", content).red().to_string(),
                            )?;
                        }
                        AgentEvent::Done { token_usage } => {
                            // Flush any pending colorizer output
                            let flush = reasoning_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            let flush = content_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            // Update token usage from event
                            app.token_usage = token_usage;
                            app.finish_task();
                            write_to_output(
                                &mut stdout,
                                "✅ Operation Complete\n".green().to_string(),
                            )?;
                            full_message.clear();
                        }
                        AgentEvent::Aborted { token_usage } => {
                            // Flush any pending colorizer output
                            let flush = reasoning_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            let flush = content_colorizer.finish();
                            if !flush.is_empty() {
                                write_to_output(&mut stdout, flush)?;
                            }
                            // Update token usage from event
                            app.token_usage = token_usage;
                            app.finish_task();
                            write_to_output(
                                &mut stdout,
                                "🛑 Operation aborted by user.\n".to_string(),
                            )?;
                        }
                    }
                }
                TuiEvent::Tick => {
                    app.tick();
                    let (w, h) = terminal::size().unwrap_or((80, 24));
                    if (w, h) != last_size {
                        last_size = (w, h);
                        // Force scrolling region update below
                        last_footer_height = 0;
                    }
                }
            }
            // Footer is always 4 lines; update scrolling region once if terminal resized
            let new_fh = 4u16;
            if new_fh != last_footer_height {
                let (_w, h) = terminal::size().unwrap_or((80, 24));
                let log_h = h.saturating_sub(new_fh);
                execute!(stdout, style::Print(format!("\x1b[1;{}r", log_h)))?;
                last_footer_height = new_fh;
            }
            render_footer(&mut stdout, &app)?;
        }

        // Cleanup: Reset scrolling region and clear
        let (_, _h) = terminal::size().unwrap_or((80, 24));
        execute!(
            stdout,
            style::Print("\x1b[r"), // Reset scrolling region to full screen
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            DisableBracketedPaste,
        )?;

        disable_raw_mode()?;
        println!();
        Ok(full_message)
    }
}

/// Renders the fixed 4-line footer at the very bottom with dark background.
///
/// Layout (from bottom):
///   Line 4 (bottom):  Queue (q1: cmd1  q2: cmd2  ...) — horizontal
///   Line 3:           Input prompt "> "
///   Line 2:           Folder path + Token usage info
///   Line 1 (top of ft): Spinner + Status message
fn render_footer(stdout: &mut io::Stdout, app: &App) -> io::Result<()> {
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
        " ⚠️ AWAITING APPROVAL (y/n/a) ".red().to_string()
    } else if let Some(task) = &app.current_task {
        let elapsed = app
            .job_start_time
            .map(|s| format!(" ({:.1}s)", s.elapsed().as_secs_f32()))
            .unwrap_or_default();
        format!(" {}...{} ", task, elapsed).blue().to_string()
    } else {
        format!(" {} ", app.model).magenta().to_string()
    };

    let line1 = format!("{}{}", spinner, status);
    stdout.queue(style::Print(line1))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // ── Line 2: Folder + Token info ─────────────────────────────────
    stdout.queue(cursor::MoveTo(0, term_height.saturating_sub(fh - 1)))?;

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
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // ── Line 3: Input prompt ────────────────────────────────────────
    let line3_y = term_height.saturating_sub(2);
    stdout.queue(cursor::MoveTo(0, line3_y))?;

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
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // Cursor X: prompt width + cursor char offset (relative to visible portion)
    let visible_input_chars = app.input.chars().count();
    let visible_offset = if visible_input_chars > avail && avail > 0 {
        visible_input_chars.saturating_sub(avail)
    } else {
        0
    };
    let cursor_char = app.input[..app.cursor_pos.min(app.input.len())]
        .chars()
        .count();
    let cursor_x = 2 + ((cursor_char.saturating_sub(visible_offset)) as u16);

    // ── Line 4 (bottom): Queue entries horizontal ───────────────────
    let line4_y = term_height.saturating_sub(1);
    stdout.queue(cursor::MoveTo(0, line4_y))?;

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

/// Truncate a string to fit within max_width chars, adding "…" if cut.
fn truncate_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if s.chars().count() > max_width {
        let truncated: String = s.chars().take(max_width.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        s.to_string()
    }
}

/// Truncate a string containing ANSI escape codes to visible width limit.
fn truncate_ansi_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let re = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
    let mut visible = 0usize;
    let mut result = String::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(m) = re.find(remaining) {
            if m.start() == 0 {
                result.push_str(m.as_str());
                remaining = &remaining[m.end()..];
                continue;
            }
            // Text before the escape
            let text = &remaining[..m.start()];
            for ch in text.chars() {
                if visible >= max_width {
                    result.push('…');
                    return result;
                }
                result.push(ch);
                visible += 1;
            }
            remaining = &remaining[m.start()..];
        } else {
            // No more escapes
            for ch in remaining.chars() {
                if visible >= max_width {
                    result.push('…');
                    return result;
                }
                result.push(ch);
                visible += 1;
            }
            break;
        }
    }
    result
}

/// Strips ANSI escape codes from a string to get visible length.
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

/// Erases the footer area, prints the new text chunk, and prepares space for the footer.
fn write_to_output(stdout: &mut io::Stdout, text: String) -> io::Result<()> {
    // 1. Jump to the current log position
    stdout.queue(cursor::RestorePosition)?;

    // 2. Print the NEW text chunk
    let safe_text = text.replace("\r\n", "\n").replace("\n", "\r\n");
    execute!(stdout, style::Print(safe_text))?;

    // 3. Save the NEW log position
    stdout.queue(cursor::SavePosition)?;

    Ok(())
}

/// Pretty-print tool arguments for display.
/// For simple tools, extracts the most relevant arg (e.g., command, path).
fn format_tool_args(name: &str, args: &str) -> String {
    // Try to parse as JSON and prettify
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(args) {
        match name {
            "execute_shell_command" => {
                if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
                    return format!("  $ {}", cmd);
                }
            }
            "read_local_file" | "write_local_file" | "delete_file" => {
                if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    return format!("  📄 {}", path);
                }
            }
            "replace_text_in_file" => {
                if let Some(path) = obj.get("file_path").and_then(|v| v.as_str()) {
                    let old = obj
                        .get("old_text")
                        .and_then(|v| v.as_str())
                        .map(|s| {
                            if s.len() > 60 {
                                format!("{}...", &s[..60])
                            } else {
                                s.to_string()
                            }
                        })
                        .unwrap_or_default();
                    return format!("  📄 {} | replace: \"{}\"", path, old);
                }
            }
            "list_directory" | "tree_view" => {
                if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                    return format!("  📂 {}", path);
                }
            }
            "fetch_url" => {
                if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                    return format!("  🌐 {}", url);
                }
            }
            "diff_files" => {
                if let (Some(f1), Some(f2)) = (
                    obj.get("file1").and_then(|v| v.as_str()),
                    obj.get("file2").and_then(|v| v.as_str()),
                ) {
                    return format!("  📄 {} ↔ {}", f1, f2);
                }
            }
            "search_code" | "search_repos" => {
                if let Some(q) = obj.get("query").and_then(|v| v.as_str()) {
                    return format!("  🔍 {}", q);
                }
            }
            _ => {}
        }
        // Fallback: compact JSON
        serde_json::to_string_pretty(&obj).unwrap_or_else(|_| args.to_string())
    } else {
        args.to_string()
    }
}

/// Detect the best language for syntax-highlighting a tool result.
fn detect_lang_for_result(tool_name: &str, result: &str) -> CodeLang {
    match tool_name {
        "read_local_file" | "write_local_file" | "replace_text_in_file" => {
            // Try to detect from the first line or content patterns
            if result.trim_start().starts_with("#!/") {
                if result.contains("python") {
                    return CodeLang::Python;
                }
                if result.contains("bash") || result.contains("sh") {
                    return CodeLang::Shell;
                }
            }
            if result.trim_start().starts_with("<?xml")
                || result.trim_start().starts_with("<!DOCTYPE html")
                || result.trim_start().starts_with("<html")
            {
                return CodeLang::Html;
            }
            if result.trim_start().starts_with("{") || result.trim_start().starts_with("[") {
                return CodeLang::Json;
            }
            if result.contains("fn ") && result.contains("->") {
                return CodeLang::Rust;
            }
            if result.contains("def ") && result.contains("return ") {
                return CodeLang::Python;
            }
            CodeLang::Generic
        }
        "execute_shell_command" => CodeLang::Shell,
        "run_python_code" => CodeLang::Python,
        "github_get_file" => CodeLang::Generic,
        _ => CodeLang::Generic,
    }
}
