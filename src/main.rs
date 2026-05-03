use anyhow::Result;
use clap::Parser;
use colored::*;
use crossterm::event::{self, Event};
use crossterm::{cursor, execute, terminal};
use deepseek_rust_cli::agent::agent::{AgentEvent, DeepSeekAgent};
use deepseek_rust_cli::agent::commands::process_command;
use deepseek_rust_cli::agent::mentions::process_mentions;
use deepseek_rust_cli::cli::Args;
use deepseek_rust_cli::config::{get_api_key, init_workspace, load_config};
use deepseek_rust_cli::logger::init_logger;
use deepseek_rust_cli::tui::event_loop::{EventLoop, TuiEvent};
use deepseek_rust_cli::version::VERSION;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    init_workspace();

    let config = load_config();
    init_logger(args.debug || config.debug);

    let api_key = get_api_key()?;
    let mut agent = DeepSeekAgent::new(api_key, config, args.session);
    agent.auto_approve = args.auto_approve;

    deepseek_rust_cli::updater::check_for_updates_background();

    let agent = Arc::new(Mutex::new(agent));

    let (tui_tx, tui_rx) = mpsc::channel(100);
    let (app_tx, mut app_rx) = mpsc::channel(1);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(100);

    // Input loop
    let tui_tx_for_input = tui_tx.clone();
    tokio::spawn(async move {
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_secs(0));

            if event::poll(timeout).unwrap_or(false)
                && let Event::Key(key) = event::read().unwrap()
            {
                let _ = tui_tx_for_input.send(TuiEvent::Input(key)).await;
            }
            if last_tick.elapsed() >= tick_rate {
                let _ = tui_tx_for_input.send(TuiEvent::Tick).await;
                last_tick = Instant::now();
            }
        }
    });

    // Agent processing task
    let agent_clone = agent.clone();
    let tui_tx_for_agent = tui_tx.clone();

    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let (agent_event_tx, mut agent_event_rx) = mpsc::channel(100);

            let tui_tx_inner = tui_tx_for_agent.clone();
            tokio::spawn(async move {
                while let Some(ev) = agent_event_rx.recv().await {
                    let _ = tui_tx_inner.send(TuiEvent::Agent(ev)).await;
                }
            });

            let mut agent_lock = agent_clone.lock().await;

            // Handle slash commands
            if cmd.starts_with('/') {
                match process_command(&mut agent_lock, &cmd).await {
                    Ok(Some(response)) => {
                        let _ = agent_event_tx
                            .send(AgentEvent::Content { content: response })
                            .await;
                        let _ = agent_event_tx.send(AgentEvent::Done).await;
                        continue;
                    }
                    Ok(None) => {
                        // Not a recognized command, proceed to chat
                    }
                    Err(e) => {
                        let _ = agent_event_tx
                            .send(AgentEvent::Error {
                                content: format!("Command error: {}", e),
                            })
                            .await;
                        let _ = agent_event_tx.send(AgentEvent::Done).await;
                        continue;
                    }
                }
            }

            let processed_cmd = process_mentions(&cmd);
            let _ = agent_lock
                .chat_stream(processed_cmd, agent_event_tx.clone(), &mut app_rx)
                .await;
            let _ = agent_event_tx.send(AgentEvent::Done).await;
            agent_lock.reset_cancel();
        }
    });

    // Start TUI
    let event_loop = EventLoop::new(tui_rx, tui_tx.clone(), app_tx, cmd_tx, agent.clone());

    let res = event_loop.run().await;

    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    if let Err(e) = res {
        println!("\n❌ UI error: {}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn _print_welcome_banner(agent: &DeepSeekAgent) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let w = width as usize;

    let line = "#".repeat(w).bright_blue();
    println!("{}", line);

    let title_part = format!(
        "🚀 {} {}",
        "DeepSeek CLI".bold().bright_yellow(),
        VERSION.cyan()
    );
    let time_part = format!(
        "📅 {}",
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .dimmed()
    );
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default()
        .bright_magenta();
    let dir = std::env::current_dir()
        .unwrap_or_default()
        .display()
        .to_string()
        .bright_white();

    let host_str = format!("💻 {}", host);
    let dir_str = format!("📂 {}", dir);

    let plain_len = format!(" {}  {} │ {} │ {}", title_part, time_part, host, dir).len();

    if plain_len > w + 20 {
        println!(" {} │ {}", title_part, time_part);
        println!("  {} {} {}", host_str, "│".dimmed(), dir_str);
    } else {
        println!(
            " {} │ {} │ {} {} {}",
            title_part,
            time_part,
            host_str,
            "│".dimmed(),
            dir_str
        );
    }

    println!();
    println!(
        "  📌 {}  🔧 {}  🗑️ {}  🔁 {}  💾 {}  📂 {}  🚪 {}",
        "Model".dimmed(),
        "Tools".dimmed(),
        "Clear".dimmed(),
        "Undo".dimmed(),
        "Save".dimmed(),
        "Load".dimmed(),
        "Exit".dimmed()
    );
    println!(
        "  ✨ {}  🛠️ {}  🧹 {}  🔙 {}  📁 {}  📖 {}  🚪 {}",
        agent.model.bright_cyan(),
        "Enabled".bright_green(),
        "/clear".bright_yellow(),
        "/undo".bright_yellow(),
        "/savemem".bright_yellow(),
        "/resume".bright_yellow(),
        "exit/quit".bright_red()
    );
    println!();
    println!("{}", line);
    println!();
}
