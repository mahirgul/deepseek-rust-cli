use colored::*;
use std::io::{self, Write};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum SpinnerCmd {
    Start(String),
    Stop,
}

pub struct Spinner {
    tx: mpsc::Sender<SpinnerCmd>,
    running: bool,
}

impl Spinner {
    pub fn new() -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel::<SpinnerCmd>(10);

        let handle = tokio::spawn(async move {
            let spinner_chars = vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            let mut current_status = String::from("Thinking");
            let mut active = true;
            let mut start_time = tokio::time::Instant::now();

            loop {
                tokio::select! {
                    cmd = rx.recv() => {
                        match cmd {
                            Some(SpinnerCmd::Start(status)) => {
                                active = true;
                                current_status = status;
                                start_time = tokio::time::Instant::now();
                            }
                            Some(SpinnerCmd::Stop) => {
                                if active {
                                    print!("\r\x1b[K");
                                    io::stdout().flush().unwrap_or(());
                                }
                                active = false;
                            }
                            None => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(80)), if active => {
                        let elapsed = start_time.elapsed().as_secs_f32();
                        print!("\r{} {}... {:.1}s",
                            spinner_chars[i % spinner_chars.len()].to_string().cyan(),
                            current_status.dimmed(),
                            elapsed
                        );
                        io::stdout().flush().unwrap_or(());
                        i += 1;
                    }
                }
            }
        });

        (Self { tx, running: false }, handle)
    }

    pub async fn start(&mut self, status: &str) {
        let _ = self.tx.send(SpinnerCmd::Start(status.to_string())).await;
        self.running = true;
    }

    pub async fn stop(&mut self) {
        if self.running {
            let _ = self.tx.send(SpinnerCmd::Stop).await;
            self.running = false;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}
