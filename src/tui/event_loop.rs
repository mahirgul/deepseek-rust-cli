use crate::agent::agent::{AgentEvent, ApprovalResult};
use crate::tui::highlight::print_highlighted_markdown;
use crate::tui::spinner::Spinner;
use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use tokio::sync::mpsc;

pub struct EventLoop {
    rx: mpsc::Receiver<AgentEvent>,
    app_tx: mpsc::Sender<ApprovalResult>,
}

impl EventLoop {
    pub fn new(rx: mpsc::Receiver<AgentEvent>, app_tx: mpsc::Sender<ApprovalResult>) -> Self {
        Self { rx, app_tx }
    }

    pub async fn run(mut self) -> Result<String> {
        let mut full_message = String::new();
        let mut content_buffer = String::new();
        let mut is_reasoning = false;
        let (mut spinner, _spinner_handle) = Spinner::new();

        // Initial start
        spinner.start("Thinking").await;

        while let Some(event) = self.rx.recv().await {
            match event {
                AgentEvent::Reasoning { content } => {
                    if !spinner.is_running() {
                        spinner.start("Reasoning").await;
                    }
                    is_reasoning = true;
                    tracing::debug!("Agent Reasoning: {}", content);
                }
                AgentEvent::Content { content } => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    if is_reasoning {
                        println!();
                        is_reasoning = false;
                    }
                    content_buffer.push_str(&content);
                    full_message.push_str(&content);
                    if content.contains('\n') {
                        print!("{}", content_buffer);
                        content_buffer.clear();
                        io::stdout().flush().unwrap_or(());
                    }
                }
                AgentEvent::ToolStart { name, args } => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    if !content_buffer.is_empty() {
                        println!("{}", content_buffer);
                        content_buffer.clear();
                    }
                    println!("\n🔧 {} {}", "Executing tool:".yellow(), name.bold());
                    if args.len() < 200 {
                        println!("  {} {}", "Args:".dimmed(), args.dimmed());
                    }
                    spinner.start(&format!("Running {}", name)).await;
                }
                AgentEvent::ToolEnd { name } => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    println!("✅ {} {}", name.bold(), "executed.".green());
                }
                AgentEvent::ApprovalRequest { name, args } => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    if !content_buffer.is_empty() {
                        println!("{}", content_buffer);
                        content_buffer.clear();
                    }
                    println!(
                        "\n⚠️  {} {}",
                        "Approval Required for tool:".yellow().bold(),
                        name.bold().red()
                    );
                    println!("   {} {}", "Arguments:".dimmed(), args.dimmed());
                    print!("   {} [y/n/a]: ", "Approve?".yellow().bold());
                    io::stdout().flush().unwrap_or(());

                    let res = tokio::task::spawn_blocking(|| {
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap_or(0);
                        let choice = input.trim().to_lowercase();
                        if choice == "y" {
                            ApprovalResult::Yes
                        } else if choice == "a" {
                            ApprovalResult::Always
                        } else {
                            ApprovalResult::No
                        }
                    })
                    .await
                    .unwrap_or(ApprovalResult::No);

                    let _ = self.app_tx.send(res).await;
                }
                AgentEvent::Error { content } => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    if !content_buffer.is_empty() {
                        println!("{}", content_buffer);
                        content_buffer.clear();
                    }
                    tracing::error!("Agent Error: {}", content);
                    println!("\n❌ {} {}", "Error:".red(), content);
                }
                AgentEvent::Done => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    if !content_buffer.is_empty() {
                        println!("{}", content_buffer);
                        full_message.push_str(&content_buffer);
                        content_buffer.clear();
                    }

                    if full_message.contains("```")
                        || full_message.contains("**")
                        || full_message.contains('#')
                    {
                        println!("\n--- {} ---", "Formatted View".dimmed());
                        print_highlighted_markdown(&full_message);
                        println!("---");
                    }

                    println!();
                }
                AgentEvent::Aborted => {
                    if spinner.is_running() {
                        spinner.stop().await;
                    }
                    println!("\n🛑 {}", "Operation aborted by user.".yellow());
                }
            }
        }
        Ok(full_message)
    }
}
