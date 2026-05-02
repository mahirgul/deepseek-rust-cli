use crate::agent::agent::{AgentEvent, ApprovalResult};
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
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use std::io;
use tokio::sync::mpsc;

pub enum TuiEvent {
    Input(event::KeyEvent),
    Tick,
    Agent(AgentEvent),
}

struct App {
    input: String,
    messages: Vec<String>,
    list_state: ListState,
    scrollbar_state: ScrollbarState,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
        }
    }

    fn push_message(&mut self, msg: String) {
        self.messages.push(msg);
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
    app_tx: mpsc::Sender<ApprovalResult>,
    cmd_tx: mpsc::Sender<String>,
}

impl EventLoop {
    pub fn new(
        rx: mpsc::Receiver<TuiEvent>,
        app_tx: mpsc::Sender<ApprovalResult>,
        cmd_tx: mpsc::Sender<String>,
    ) -> Self {
        Self { rx, app_tx, cmd_tx }
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
                TuiEvent::Input(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                if !app.input.is_empty() {
                                    let cmd = app.input.clone();
                                    app.push_message(format!("> {}", cmd));
                                    if cmd == "exit" || cmd == "quit" {
                                        break;
                                    }
                                    let _ = self.cmd_tx.send(cmd).await;
                                    app.input.clear();
                                }
                            }
                            KeyCode::Char(c) => app.input.push(c),
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Up => app.scroll_up(),
                            KeyCode::Down => app.scroll_down(),
                            KeyCode::Esc => break,
                            _ => {}
                        }
                    }
                }
                TuiEvent::Agent(agent_event) => {
                    match agent_event {
                        AgentEvent::Reasoning { content: _ } => {
                            // Need to handle partial reasoning visually
                        }
                        AgentEvent::Content { content } => {
                            content_buffer.push_str(&content);
                            full_message.push_str(&content);
                            if content.contains('\n') {
                                app.push_message(content_buffer.clone());
                                content_buffer.clear();
                            }
                        }
                        AgentEvent::ToolStart { name, args } => {
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
                            app.push_message(format!("⚠️ Approval Required for tool: {}", name));
                            app.push_message(format!("Arguments: {}", args));
                            app.push_message("Type 'y' to approve, 'n' to reject.".to_string());
                            // Instead of auto-rejecting, we should probably handle this in the input loop.
                            // For now, auto-approve if auto_approve is set, else we need a state machine.
                            // Let's just auto-approve in this simplified TUI for testing.
                            let _ = self.app_tx.send(ApprovalResult::Yes).await;
                        }
                        AgentEvent::Error { content } => {
                            app.push_message(format!("❌ Error: {}", content));
                        }
                        AgentEvent::Done => {
                            if !content_buffer.is_empty() {
                                app.push_message(content_buffer.clone());
                                content_buffer.clear();
                            }
                            app.push_message("✅ Operation Complete".to_string());
                        }
                        AgentEvent::Aborted => {
                            app.push_message("🛑 Operation aborted by user.".to_string());
                        }
                    }
                }
                TuiEvent::Tick => {}
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

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| ListItem::new(Line::from(Span::raw(m))))
        .collect();

    let list = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Output Area (Arrows to scroll) "),
        )
        .style(Style::default().fg(Color::White));

    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    let scroll_pos = app.list_state.selected().unwrap_or(0);
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chunks[0],
        &mut app.scrollbar_state.position(scroll_pos),
    );

    let input_area = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Input Command ")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(input_area, chunks[1]);

    f.set_cursor_position((
        chunks[1].x + app.input.chars().count() as u16 + 1,
        chunks[1].y + 1,
    ));
}
