use crate::agent::agent::{AgentEvent, ApprovalResult, DeepSeekAgent};
use anyhow::Result;
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use std::io;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub enum TuiEvent {
    Input(event::KeyEvent),
    Tick,
    Agent(AgentEvent),
    Abort,
}

use std::time::Instant;

struct App {
    input: String,
    messages: Vec<String>,
    list_state: ListState,
    scrollbar_state: ScrollbarState,
    awaiting_approval: bool,
    spinner_frame: usize,
    current_task: Option<String>,
    task_start_time: Option<Instant>,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            awaiting_approval: false,
            spinner_frame: 0,
            current_task: None,
            task_start_time: None,
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

    fn push_message(&mut self, msg: String) {
        if msg.is_empty() {
            self.messages.push(String::new());
        } else {
            for line in msg.lines() {
                self.messages.push(line.to_string());
            }
        }
        self.scrollbar_state = self.scrollbar_state.content_length(self.messages.len());
        if !self.messages.is_empty() {
            let last_idx = self.messages.len().saturating_sub(1);
            self.list_state.select(Some(last_idx));
        }
    }

    fn scroll_up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn scroll_down(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.messages.len().saturating_sub(1) {
                    self.messages.len().saturating_sub(1)
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
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
        let mut content_buffer = String::new();
        let mut app = App::new();

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        app.push_message("🚀 DeepSeek CLI Started".to_string());

        while let Some(event) = self.rx.recv().await {
            terminal.draw(|f| render_ui(f, &mut app))?;

            match event {
                TuiEvent::Abort => {
                    let agent = self.agent.lock().await;
                    agent.abort();
                    app.finish_task();
                    app.push_message("🛑 Operation aborted by user.".to_string());
                }
                TuiEvent::Input(key) => {
                    if key.kind == KeyEventKind::Press {
                        if app.awaiting_approval {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    app.finish_task();
                                    app.push_message("✅ Approved".to_string());
                                    let _ = self.app_tx.send(ApprovalResult::Yes).await;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    app.finish_task();
                                    app.push_message("❌ Rejected".to_string());
                                    let _ = self.app_tx.send(ApprovalResult::No).await;
                                }
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    app.finish_task();
                                    app.push_message("🛡️ Always Approved".to_string());
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
                                app.push_message(format!("> {}", cmd));
                                if cmd == "exit" || cmd == "quit" || cmd == "/exit" || cmd == "/quit" {
                                    break;
                                }
                                let _ = self.cmd_tx.send(cmd).await;
                                app.input.clear();
                            }
                            KeyCode::Char('c') | KeyCode::Char('C')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                            {
                                break;
                            }
                            KeyCode::Char(c) => app.input.push(c),
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Up => app.scroll_up(),
                            KeyCode::Down => app.scroll_down(),
                            KeyCode::Esc => {
                                let _ = self.rx_tx.send(TuiEvent::Abort).await;
                            }
                            _ => {}
                        }
                    }
                }
                TuiEvent::Agent(agent_event) => {
                    match agent_event {
                        AgentEvent::Reasoning { content: _ } => {
                            app.start_task("Reasoning".to_string());
                        }
                        AgentEvent::Content { content } => {
                            app.start_task("Generating".to_string());
                            content_buffer.push_str(&content);
                            full_message.push_str(&content);
                            if content.contains('\n') {
                                app.push_message(content_buffer.clone());
                                content_buffer.clear();
                            }
                        }
                        AgentEvent::ToolStart { name, args } => {
                            app.start_task(format!("Tool: {}", name));
                            if !content_buffer.is_empty() {
                                app.push_message(content_buffer.clone());
                                content_buffer.clear();
                            }
                            app.push_message(format!(
                                "🔧 Executing tool: {} (args: {})",
                                name, args
                            ));
                        }
                        AgentEvent::ToolEnd { name } => {
                            app.push_message(format!("✅ {} executed.", name));
                        }
                        AgentEvent::ApprovalRequest { name, args } => {
                            app.start_task("Awaiting Approval".to_string());
                            app.awaiting_approval = true;
                            app.push_message(format!("⚠️ Approval Required for tool: {}", name));
                            app.push_message(format!("Arguments: {}", args));
                            app.push_message(
                                "? Press 'y' to approve, 'n' to reject, 'a' to allow all."
                                    .to_string(),
                            );
                        }
                        AgentEvent::Error { content } => {
                            app.finish_task();
                            app.push_message(format!("❌ Error: {}", content));
                        }
                        AgentEvent::Done => {
                            app.finish_task();
                            if !content_buffer.is_empty() {
                                app.push_message(content_buffer.clone());
                                content_buffer.clear();
                            }
                            app.push_message("✅ Operation Complete".to_string());
                        }
                        AgentEvent::Aborted => {
                            app.finish_task();
                            app.push_message("🛑 Operation aborted by user.".to_string());
                        }
                    }
                }
                TuiEvent::Tick => {
                    app.tick();
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(full_message)
    }
}

fn render_ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    let mut text_lines = Vec::new();
    for m in &app.messages {
        let style = if m.starts_with("⚠️") {
            Style::default().fg(Color::Yellow)
        } else if m.starts_with("❌") {
            Style::default().fg(Color::Red)
        } else if m.starts_with("✅") {
            Style::default().fg(Color::Green)
        } else if m.starts_with("🔧") {
            Style::default().fg(Color::Blue)
        } else if m.starts_with("?") {
            Style::default().fg(Color::LightRed)
        } else if m.starts_with("Arguments:") {
            Style::default().fg(Color::LightYellow)
        } else if m.starts_with(">") {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        text_lines.push(Line::from(Span::styled(m, style)));
    }

    let scroll = app.list_state.selected().unwrap_or(0) as u16;

    let output_area = Paragraph::new(Text::from(text_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Output Area (Arrows to scroll) "),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(output_area, chunks[0]);

    let scroll_pos = scroll as usize;
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chunks[0],
        &mut app.scrollbar_state.position(scroll_pos),
    );

    let spinner_chars = vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];

    let input_title = if app.awaiting_approval {
        format!(" {} ⚠️ AWAITING APPROVAL (y/n/a) ", spinner)
    } else if let Some(task) = &app.current_task {
        let elapsed = app
            .task_start_time
            .map(|s| format!(" ({:.1}s)", s.elapsed().as_secs_f32()))
            .unwrap_or_default();
        format!(" {} {}...{} ", spinner, task, elapsed)
    } else {
        " Input Command ".to_string()
    };

    let input_area = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_title)
                .border_style(if app.awaiting_approval {
                    Style::default().fg(Color::Red)
                } else if app.current_task.is_some() {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::Yellow)
                }),
        );

    f.render_widget(input_area, chunks[1]);

    if !app.awaiting_approval {
        f.set_cursor_position((
            chunks[1].x + app.input.chars().count() as u16 + 1,
            chunks[1].y + 1,
        ));
    }
}
