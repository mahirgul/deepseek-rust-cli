use crate::agent::agent::{AgentEvent, ApprovalResult, DeepSeekAgent};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, KeyCode, KeyEventKind},
    execute,
    style::{self, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

pub enum TuiEvent {
    Input(event::KeyEvent),
    Mouse(event::MouseEvent),
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
    awaiting_approval: bool,
    spinner_frame: usize,
    current_task: Option<String>,
    task_start_time: Option<Instant>,
    cwd: String,
    model: String,
    history: Vec<String>,
    history_index: Option<usize>,
    footer_height: u16,
}

impl App {
    fn new() -> Self {
        let history = load_global_history();
        Self {
            input: String::new(),
            awaiting_approval: false,
            spinner_frame: 0,
            current_task: None,
            task_start_time: None,
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            model: String::from("unknown"),
            history,
            history_index: None,
            footer_height: 2, // Line 1: Status, Line 2: Input
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
                    None
                }
            }
            None => None,
        };
        self.history_index = next_index;
        if let Some(idx) = self.history_index {
            self.input = self.history[idx].clone();
        }
    }

    fn start_task(&mut self, task: String) {
        if self.current_task.as_ref() != Some(&task) {
            self.current_task = Some(task);
            self.task_start_time = Some(Instant::now());
        }
    }

    fn finish_task(&mut self) {
        self.current_task = None;
        self.task_start_time = None;
        self.awaiting_approval = false;
    }

    fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }
}

pub struct EventLoop {
    rx: mpsc::Receiver<TuiEvent>,
    rx_tx: mpsc::Sender<TuiEvent>,
    app_tx: mpsc::Sender<ApprovalResult>,
    cmd_tx: mpsc::Sender<String>,
    agent: Arc<Mutex<DeepSeekAgent>>,
}

impl EventLoop {
    pub fn new(
        rx: mpsc::Receiver<TuiEvent>,
        rx_tx: mpsc::Sender<TuiEvent>,
        app_tx: mpsc::Sender<ApprovalResult>,
        cmd_tx: mpsc::Sender<String>,
        agent: Arc<Mutex<DeepSeekAgent>>,
    ) -> Self {
        Self {
            rx,
            rx_tx,
            app_tx,
            cmd_tx,
            agent,
        }
    }

    pub async fn run(mut self) -> Result<String> {
        let mut full_message = String::new();
        let mut app = App::new();

        {
            let agent = self.agent.lock().await;
            app.model = agent.model.clone();
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();

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
        render_footer(&mut stdout, &app)?;

        while let Some(event) = self.rx.recv().await {
            match event {
                TuiEvent::Abort => {
                    let agent = self.agent.lock().await;
                    agent.abort();
                    app.finish_task();
                    write_to_output(&mut stdout, "🛑 Operation aborted by user.\n".to_string())?;
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
                                let _ = self.cmd_tx.send(cmd).await;
                                app.input.clear();
                            }
                            KeyCode::Char('c') | KeyCode::Char('C')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                            {
                                break;
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
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
                TuiEvent::Agent(agent_event) => match agent_event {
                    AgentEvent::Reasoning { content } => {
                        app.start_task("Reasoning".to_string());
                        if !content.is_empty() {
                            write_to_output(&mut stdout, content)?;
                        }
                    }
                    AgentEvent::Content { content } => {
                        app.start_task("Generating".to_string());
                        full_message.push_str(&content);
                        write_to_output(&mut stdout, content)?;
                    }
                    AgentEvent::ToolStart { name, args } => {
                        app.start_task(format!("Tool: {}", name));
                        write_to_output(
                            &mut stdout,
                            format!("🔧 Executing tool: {} (args: {})\n", name, args)
                                .blue()
                                .to_string(),
                        )?;
                    }
                    AgentEvent::ToolEnd { name } => {
                        write_to_output(
                            &mut stdout,
                            format!("✅ {} executed.\n", name).green().to_string(),
                        )?;
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
                        app.finish_task();
                        write_to_output(
                            &mut stdout,
                            format!("❌ Error: {}\n", content).red().to_string(),
                        )?;
                    }
                    AgentEvent::Done => {
                        app.finish_task();
                        write_to_output(
                            &mut stdout,
                            "✅ Operation Complete\n".green().to_string(),
                        )?;
                        full_message.clear();
                        let agent = self.agent.lock().await;
                        app.model = agent.model.clone();
                    }
                    AgentEvent::Aborted => {
                        app.finish_task();
                        write_to_output(
                            &mut stdout,
                            "🛑 Operation aborted by user.\n".to_string(),
                        )?;
                    }
                },
                TuiEvent::Tick => {
                    app.tick();
                    let (w, h) = terminal::size().unwrap_or((80, 24));
                    if (w, h) != last_size {
                        let log_h = h.saturating_sub(app.footer_height);
                        execute!(stdout, style::Print(format!("\x1b[1;{}r", log_h)))?;
                        last_size = (w, h);
                    }
                }
            }
            render_footer(&mut stdout, &app)?;
        }

        // Cleanup: Reset scrolling region and clear
        let (_, _h) = terminal::size().unwrap_or((80, 24));
        execute!(
            stdout,
            style::Print("\x1b[r"), // Reset scrolling region to full screen
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        disable_raw_mode()?;
        println!();
        Ok(full_message)
    }
}

/// Renders the 2-line footer at the very bottom with dark background.
fn render_footer(stdout: &mut io::Stdout, app: &App) -> io::Result<()> {
    // Get terminal dimensions
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    // 1. Move to the absolute bottom area for the footer
    stdout.queue(cursor::Hide)?;
    stdout.queue(cursor::MoveTo(
        0,
        term_height.saturating_sub(app.footer_height),
    ))?;

    // 2. Set black background and clear the footer area
    stdout.queue(style::SetBackgroundColor(style::Color::Black))?;
    stdout.queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;

    // 3. Line 1: Status Line
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
            .task_start_time
            .map(|s| format!(" ({:.1}s)", s.elapsed().as_secs_f32()))
            .unwrap_or_default();
        format!(" {}...{} ", task, elapsed).blue().to_string()
    } else {
        format!(" {} ", app.model).magenta().to_string()
    };

    // Emoji-safe visible length calculation
    let spinner_visible_len = 2; // ✨ is 2 cells
    let status_visible_len = strip_ansi(&status).chars().count() + 2; // +2 for potential emojis
    let fixed_len = spinner_visible_len + status_visible_len + 10; // Extra margin

    // Truncate CWD if necessary
    let mut cwd_text = app.cwd.clone();
    let max_cwd_len = (term_width as usize).saturating_sub(fixed_len);
    if cwd_text.len() > max_cwd_len && max_cwd_len > 0 {
        cwd_text = format!("...{}", &cwd_text[cwd_text.len() - max_cwd_len + 3..]);
    }
    let cwd = format!(" 📂 {} ", cwd_text).blue();

    // Print line 1
    let line1 = format!("{}{}|{}", spinner, status, cwd);
    stdout.queue(style::Print(line1))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // 4. Line 2: Input Prompt
    stdout.queue(cursor::MoveTo(0, term_height.saturating_sub(1)))?;
    let prompt_prefix = "> ";
    let mut input_text = app.input.clone();
    let max_input_len = (term_width as usize).saturating_sub(prompt_prefix.len() + 10);
    let mut display_offset = 0;
    if input_text.chars().count() > max_input_len && max_input_len > 0 {
        display_offset = input_text.chars().count() - max_input_len;
        input_text = input_text.chars().skip(display_offset).collect();
    }

    let line2 = format!("{}{}", prompt_prefix, input_text)
        .cyan()
        .to_string();
    stdout.queue(style::Print(line2))?;
    stdout.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;

    // 5. Reset styles
    stdout.queue(style::SetBackgroundColor(style::Color::Reset))?;
    stdout.queue(style::ResetColor)?;

    // 6. Position cursor for the user (blinking at input prompt)
    let cursor_pos = if display_offset > 0 {
        max_input_len + prompt_prefix.len()
    } else {
        app.input.chars().count() + prompt_prefix.len()
    };
    stdout.queue(cursor::MoveTo(
        cursor_pos as u16,
        term_height.saturating_sub(1),
    ))?;
    stdout.queue(cursor::Show)?;
    stdout.flush()?;

    Ok(())
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
