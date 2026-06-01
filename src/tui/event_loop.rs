use std::{
    io::{self},
    sync::Arc,
};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, DisableBracketedPaste, EnableBracketedPaste},
    execute,
    style::{self, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use tokio::sync::{mpsc, Mutex};

use crate::{
    agent::{agent::DeepSeekAgent, types::ApprovalResult},
    tui::{
        app::App,
        colorizer::StreamColorizer,
        render::{render_footer, write_to_output},
    },
};

pub enum TuiEvent {
    Input(event::KeyEvent),
    Mouse(event::MouseEvent),
    /// Bracketed paste content (multi-line preserved)
    Paste(String),
    Tick,
    Agent(crate::agent::types::AgentEvent),
    Abort,
}

#[cfg(windows)]
extern "system" {
    fn GetConsoleCP() -> u32;
    fn GetConsoleOutputCP() -> u32;
    fn SetConsoleCP(wCodePageID: u32) -> i32;
    fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
}

struct TerminalGuard {
    #[cfg(windows)]
    orig_cp: Option<(u32, u32)>,
}

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;

        #[cfg(windows)]
        let orig_cp = unsafe {
            let cp = GetConsoleCP();
            let ocp = GetConsoleOutputCP();
            if SetConsoleCP(65001) != 0 && SetConsoleOutputCP(65001) != 0 {
                Some((cp, ocp))
            } else {
                None
            }
        };

        Ok(Self {
            #[cfg(windows)]
            orig_cp,
        })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            style::Print("\x1b[r"), // Reset scrolling region to full screen
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            DisableBracketedPaste,
            cursor::Show,
        );

        #[cfg(windows)]
        if let Some((cp, ocp)) = self.orig_cp {
            unsafe {
                let _ = SetConsoleCP(cp);
                let _ = SetConsoleOutputCP(ocp);
            }
        }
    }
}

pub struct EventLoop {
    pub rx: mpsc::Receiver<TuiEvent>,
    pub app_tx: mpsc::Sender<ApprovalResult>,
    pub cmd_tx: mpsc::Sender<(usize, String)>,
    pub agent: Arc<Mutex<DeepSeekAgent>>,
    /// Shared cancel token — can be cancelled without locking the agent mutex
    pub cancel_token: Arc<std::sync::Mutex<tokio_util::sync::CancellationToken>>,
    pub run_id: Arc<std::sync::atomic::AtomicUsize>,
}

impl EventLoop {
    pub fn new(
        rx: mpsc::Receiver<TuiEvent>,
        _rx_tx: mpsc::Sender<TuiEvent>,
        app_tx: mpsc::Sender<ApprovalResult>,
        cmd_tx: mpsc::Sender<(usize, String)>,
        agent: Arc<Mutex<DeepSeekAgent>>,
        cancel_token: Arc<std::sync::Mutex<tokio_util::sync::CancellationToken>>,
        run_id: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        Self {
            rx,
            app_tx,
            cmd_tx,
            agent,
            cancel_token,
            run_id,
        }
    }

    fn handle_abort(&self, app: &mut App, stdout: &mut io::Stdout) -> Result<()> {
        if app.queued_commands.is_empty() {
            return Ok(());
        }
        // Cancel via shared token — no agent lock needed, avoids deadlock
        if let Ok(token) = self.cancel_token.lock() {
            token.cancel();
        } else {
            tracing::warn!("Cancel token mutex poisoned during abort");
        }
        // Increment run_id to discard any queued operations in cmd_rx
        self.run_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Set aborted flag so we ignore any in-flight AgentEvents
        app.aborted = true;
        app.current_task = None;
        app.task_start_time = None;
        app.job_start_time = None;
        app.awaiting_approval = false;
        app.queued_commands.clear();
        write_to_output(stdout, app, "🛑 Operation aborted by user.\n".to_string())?;
        Ok(())
    }

    pub async fn run(mut self) -> Result<String> {
        let mut full_message = String::new();
        let mut app = App::new();
        let mut reasoning_colorizer = StreamColorizer::new();
        reasoning_colorizer.set_dimmed(true);
        let mut content_colorizer = StreamColorizer::new();

        {
            if let Ok(agent) = self.agent.try_lock() {
                app.model = agent.model.clone();
                app.token_usage = agent.token_usage.clone();
            }
        }

        let _guard = TerminalGuard::new()?;
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
        )?;

        // Print beautiful startup logo
        let logo_lines = vec![
            format!(
                "  {}   {}   {}",
                "██████╗ ".cyan().bold(),
                "  ██████╗".magenta().bold(),
                "DeepSeek CLI Agent".cyan().bold()
            ),
            format!(
                "  {}   {}   {}",
                "██╔══██╗".cyan().bold(),
                " ██╔════╝".magenta().bold(),
                "Autonomous Terminal System".dim()
            ),
            format!(
                "  {}   {}   {}",
                "██║  ██║".cyan().bold(),
                " ██║     ".magenta().bold(),
                format!("Version {}", crate::version::VERSION).dim()
            ),
            format!(
                "  {}   {}   {}",
                "██║  ██║".cyan().bold(),
                " ██║     ".magenta().bold(),
                "Status: Ready".dim()
            ),
            format!(
                "  {}   {}   {}",
                "██████╔╝".cyan().bold(),
                " ╚██████╗".magenta().bold(),
                "Type /help for command list".dim()
            ),
            format!(
                "  {}   {}   {}",
                "╚═════╝ ".cyan().bold(),
                "  ╚═════╝".magenta().bold(),
                ""
            ),
        ];

        for line in logo_lines {
            write_to_output(&mut stdout, &mut app, format!("{}\n", line))?;
        }
        write_to_output(&mut stdout, &mut app, "\n".to_string())?;

        let mut last_size = (term_width, term_height);
        let mut last_footer_height = app.footer_height; // always 4
        render_footer(&mut stdout, &app)?;

        while let Some(event) = self.rx.recv().await {
            match event {
                TuiEvent::Abort => {
                    if !app.queued_commands.is_empty() {
                        self.handle_abort(&mut app, &mut stdout)?;
                    }
                }
                TuiEvent::Paste(text) => {
                    if !text.is_empty() {
                        let byte_pos = app.cursor_pos.min(app.input.len());
                        app.input.insert_str(byte_pos, &text);
                        app.cursor_pos = byte_pos + text.len();
                    }
                }
                TuiEvent::Mouse(_) => {}
                TuiEvent::Input(key) => {
                    if self.handle_input(&mut app, &mut stdout, key)? {
                        break;
                    }
                }
                TuiEvent::Agent(agent_event) => {
                    self.handle_agent_event(
                        &mut app,
                        &mut stdout,
                        agent_event,
                        &mut full_message,
                        &mut reasoning_colorizer,
                        &mut content_colorizer,
                    )?;
                }
                TuiEvent::Tick => {
                    app.tick();
                    if let Ok(p) = std::env::current_dir() {
                        app.cwd = p.display().to_string();
                    }
                    let (w, h) = terminal::size().unwrap_or((80, 24));
                    if (w, h) != last_size {
                        last_size = (w, h);
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
                if app.log_y >= log_h {
                    app.log_y = log_h.saturating_sub(1);
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
            cursor::MoveTo(0, 0),
            DisableBracketedPaste,
        )?;

        disable_raw_mode()?;
        println!();
        Ok(full_message)
    }
}
