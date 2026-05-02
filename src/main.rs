use anyhow::Result;
use clap::Parser;
use colored::*;
use crossterm::{cursor, execute, terminal};
use deepseek_rust_cli::agent::agent::DeepSeekAgent;
use deepseek_rust_cli::agent::mentions::process_mentions;
use deepseek_rust_cli::cli::Args;
use deepseek_rust_cli::config::{get_api_key, init_workspace, load_config};
use deepseek_rust_cli::logger::init_logger;
use deepseek_rust_cli::tui::event_loop::EventLoop;
use deepseek_rust_cli::tui::input::{InputHandler, InputResult};
use deepseek_rust_cli::version::VERSION;
use std::io;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Clear screen at startup
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    // Initialize workspace (.deep directory, config.json, memory.md)
    init_workspace();

    let config = load_config();
    init_logger(args.debug || config.debug);

    let api_key = get_api_key()?;
    let mut agent = DeepSeekAgent::new(api_key, config, args.session);
    agent.auto_approve = args.auto_approve;
    let mut input_handler = InputHandler::new();

    // Start background update check
    deepseek_rust_cli::updater::check_for_updates_background();

    print_welcome_banner(&agent);

    while let Ok(res) = input_handler.read_line() {
        match res {
            InputResult::Text(text) => {
                if text.trim().is_empty() {
                    continue;
                }
                if text == "/exit" || text == "/quit" {
                    break;
                }

                if text.starts_with('/') {
                    if let Some(res) =
                        deepseek_rust_cli::agent::commands::process_command(&mut agent, &text)
                            .await?
                    {
                        if res == "RETRY" {
                            while agent.messages.len() > 1
                                && agent
                                    .messages
                                    .last()
                                    .map(|m| m.role != "user")
                                    .unwrap_or(false)
                            {
                                agent.messages.pop();
                            }
                        } else {
                            println!("{}", res.cyan());
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                let processed_input = if text.starts_with('/') {
                    String::new()
                } else {
                    process_mentions(&text)
                };
                let (tx, rx) = mpsc::channel(100);
                let (app_tx, app_rx) = mpsc::channel(1);
                let cancel_token = agent.cancel_token.clone();

                let chat_future = agent.chat_stream(processed_input, tx, app_rx);
                let event_loop = EventLoop::new(rx, app_tx);

                // Cancellation listener task
                let cancel_token_task = cancel_token.clone();
                let cancel_handle = tokio::spawn(async move {
                    use crossterm::event::{Event, KeyCode, KeyModifiers, poll, read};
                    use std::time::Duration;

                    loop {
                        if cancel_token_task.is_cancelled() {
                            break;
                        }
                        // Use a slightly longer poll to be more CPU efficient
                        if let Ok(true) = poll(Duration::from_millis(50))
                            && let Ok(Event::Key(key)) = read()
                            && (key.code == KeyCode::Esc
                                || (key.code == KeyCode::Char('c')
                                    && key.modifiers.contains(KeyModifiers::CONTROL)))
                        {
                            cancel_token_task.cancel();
                            break;
                        }
                    }
                });

                let (chat_res, _) = tokio::join!(chat_future, event_loop.run());
                cancel_handle.abort(); // Ensure the listener stops immediately

                if let Err(e) = chat_res
                    && !e.to_string().contains("cancelled")
                {
                    println!("\n❌ Agent error: {}", e);
                }
                agent.reset_cancel();
            }
            InputResult::Exit | InputResult::Eof => break,
        }
    }

    Ok(())
}

fn print_welcome_banner(agent: &DeepSeekAgent) {
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
        "/help".cyan().bold(),
        "/model".cyan().bold(),
        "/clear".cyan().bold(),
        "/retry".cyan().bold(),
        "/save".cyan().bold(),
        "/sessions".cyan().bold(),
        "/exit".cyan().bold()
    );

    println!("{}", line);
    println!(
        "Session: {} | Model: {}",
        agent.session_id.bright_cyan(),
        agent.model.bright_cyan()
    );

    // Show update notification if available
    if let Some(latest) = deepseek_rust_cli::updater::get_latest_available_version()
        && latest != VERSION
    {
        println!(
            "\n✨ {} {} {} {}",
            "New version".bright_green(),
            latest.bold().bright_green(),
            "available! Type".bright_green(),
            "/update".bold().yellow()
        );
    }
}
