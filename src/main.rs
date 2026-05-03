use anyhow::Result;
use clap::{CommandFactory, Parser};
use crossterm::event::{self, Event};
use crossterm::{cursor, execute, terminal};
use deepseek_rust_cli::agent::agent::{AgentEvent, DeepSeekAgent};
use deepseek_rust_cli::agent::commands::process_command;
use deepseek_rust_cli::agent::mentions::process_mentions;
use deepseek_rust_cli::cli::{Args, ShellType};
use deepseek_rust_cli::config::{get_api_key, init_workspace, load_config};
use deepseek_rust_cli::logger::init_logger;
use deepseek_rust_cli::tui::event_loop::{EventLoop, TuiEvent};
use std::io::{self};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --generate-completion early (no TUI needed)
    if let Some(shell) = &args.generate_completion {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();
        let mut stdout = io::stdout();
        match shell {
            ShellType::Bash => {
                clap_complete::generate(clap_complete::shells::Bash, &mut cmd, name, &mut stdout)
            }
            ShellType::Zsh => {
                clap_complete::generate(clap_complete::shells::Zsh, &mut cmd, name, &mut stdout)
            }
            ShellType::Fish => {
                clap_complete::generate(clap_complete::shells::Fish, &mut cmd, name, &mut stdout)
            }
            ShellType::PowerShell => clap_complete::generate(
                clap_complete::shells::PowerShell,
                &mut cmd,
                name,
                &mut stdout,
            ),
            ShellType::Elvish => {
                clap_complete::generate(clap_complete::shells::Elvish, &mut cmd, name, &mut stdout)
            }
        }
        return Ok(());
    }

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

            if event::poll(timeout).unwrap_or(false) {
                match event::read().unwrap() {
                    Event::Key(key) => {
                        let _ = tui_tx_for_input.send(TuiEvent::Input(key)).await;
                    }
                    Event::Mouse(mouse) => {
                        let _ = tui_tx_for_input.send(TuiEvent::Mouse(mouse)).await;
                    }
                    Event::Paste(text) => {
                        let _ = tui_tx_for_input.send(TuiEvent::Paste(text)).await;
                    }
                    _ => {}
                }
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
                        let tu = agent_lock.token_usage.clone();
                        let _ = agent_event_tx
                            .send(AgentEvent::Content { content: response })
                            .await;
                        let _ = agent_event_tx
                            .send(AgentEvent::Done { token_usage: tu })
                            .await;
                        continue;
                    }
                    Ok(None) => {
                        // Not a recognized command, proceed to chat
                    }
                    Err(e) => {
                        let tu = agent_lock.token_usage.clone();
                        let _ = agent_event_tx
                            .send(AgentEvent::Error {
                                content: format!("Command error: {}", e),
                            })
                            .await;
                        let _ = agent_event_tx
                            .send(AgentEvent::Done { token_usage: tu })
                            .await;
                        continue;
                    }
                }
            }

            let processed_cmd = process_mentions(&cmd);
            let _ = agent_lock
                .chat_stream(processed_cmd, agent_event_tx.clone(), &mut app_rx)
                .await;
            // Only send Done if not aborted (Aborted event already sent by chat_stream)
            if !agent_lock.cancel_token.lock().unwrap().is_cancelled() {
                let tu = agent_lock.token_usage.clone();
                let _ = agent_event_tx
                    .send(AgentEvent::Done { token_usage: tu })
                    .await;
            }
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
