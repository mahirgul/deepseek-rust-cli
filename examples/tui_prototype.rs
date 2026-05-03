use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

enum TuiEvent {
    Input(event::KeyEvent),
    Tick,
    Log(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channels for events
    let (tx, mut rx) = mpsc::channel(100);

    // Background task simulator
    let log_tx = tx.clone();
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            count += 1;
            let _ = log_tx
                .send(TuiEvent::Log(format!("Background log #{}", count)))
                .await;
        }
    });

    // Event loop task
    let input_tx = tx.clone();
    tokio::spawn(async move {
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_secs(0));

            if event::poll(timeout).unwrap_or(false)
                && let Event::Key(key) = event::read().unwrap() {
                let _ = input_tx.send(TuiEvent::Input(key)).await;
            }
            if last_tick.elapsed() >= tick_rate {
                let _ = input_tx.send(TuiEvent::Tick).await;
                last_tick = Instant::now();
            }
        }
    });

    // App state
    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(f.area());

            // Output area
            let items: Vec<ListItem> = messages.iter().map(|m| ListItem::new(m.as_str())).collect();
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Output Log (Scrolls)"),
            );
            // In a real app, you'd handle scrolling here.
            // For this demo, we just show the last N lines that fit.
            f.render_widget(list, chunks[0]);

            // Input area
            let input_widget = Paragraph::new(input.as_str())
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_widget, chunks[1]);

            // Set cursor position for input
            f.set_cursor_position((
                chunks[1].x + input.chars().count() as u16 + 1,
                chunks[1].y + 1,
            ));
        })?;

        if let Some(event) = rx.recv().await {
            match event {
                TuiEvent::Input(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter if !input.is_empty() => {
                                messages.push(format!("You: {}", input));
                                // Process command...
                                if input == "exit" || input == "quit" {
                                    break;
                                }
                                input.clear();
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                            }
                            KeyCode::Backspace => {
                                input.pop();
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                TuiEvent::Log(msg) => {
                    messages.push(msg);
                }
                TuiEvent::Tick => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
