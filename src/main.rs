mod cli;
mod models;
mod utils;

use clap::Parser;
use cli::Args;
use colored::*;
use dotenvy::dotenv;
use futures::stream::StreamExt;
use models::{ChatRequest, Message, StreamResponse};
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::env;
use std::io::{self, Write};
use std::process::Command;
use utils::{is_safe_command, load_history, save_history};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv();
    let args = Args::parse();
    let config = utils::load_config();

    if let Some(dirs) = &config.init_directories {
        for dir in dirs {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!(
                    "{} Failed to create directory ({}): {}",
                    "⚠️ Warning:".yellow().bold(),
                    dir,
                    e
                );
            }
        }
    }

    let critical_files = vec![".deep/memory.md", ".deep/plan.md"];
    for file in &critical_files {
        if !std::path::Path::new(file).exists() {
            let _ = std::fs::write(file, format!("# {}\n\n(This file is empty.)\n", file));
        }
    }

    let mut model = args.model.unwrap_or_else(|| {
        config
            .model
            .clone()
            .unwrap_or_else(|| "deepseek-chat".to_string())
    });

    let base_url = config
        .base_url
        .unwrap_or_else(|| "https://api.deepseek.com".to_string());
    let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| {
        anyhow::anyhow!("DEEPSEEK_API_KEY must be set in the environment or .env file")
    })?;

    let client = Client::new();
    let mut rl = DefaultEditor::new()?;
    
    // ... (rest of setup)


    let default_prompt = "You are a helpful AI assistant acting as a CLI agent.
You can execute tools using the following formats:

1. BASH: ```bash\n<command>\n``` (Safe commands only)
2. FETCH: ```fetch\n<url>\n``` (Reads and cleans HTML content)
3. READ: ```read\n<file> <start_line> <end_line>\n```
4. PATCH:
```patch
FILE: <path>
<<<<
<exact old text to replace>
====
<new text>
>>>>
```

Be concise. When suggested a bash command, ensure it's safe. Use PATCH for surgical file edits."
        .to_string();

    let mut messages: Vec<Message> = load_history();
    if messages.is_empty() {
        messages.push(Message {
            role: "system".to_string(),
            content: config.system_prompt.clone().unwrap_or(default_prompt.clone()),
        });
    }

    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap_or(());


    println!("{}", "🚀 DeepSeek CLI Agent Pro Started!".green().bold());
    println!("Model: {} | API: {}", model.cyan(), base_url.cyan());
    println!(
        "{}",
        "Type '/help' for commands. Use up/down arrows for history.".dimmed()
    );

    let mut auto_approve = args.auto_approve;
    let mut autonomous_turn = false;
    let mut iteration_count = 0;
    let max_iterations = 5;

    loop {
        // Refresh context every turn
        let mut current_dir_context = String::new();
        if let Ok(cwd) = std::env::current_dir() {
            current_dir_context.push_str(&format!(
                "\n\n### CURRENT WORKING DIRECTORY:\n{}\n",
                cwd.display()
            ));
        }

        if let Ok(entries) = std::fs::read_dir(".") {
            current_dir_context.push_str("\n\n### PROJECT STRUCTURE:\n");
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if !name.starts_with('.') && name != "target" {
                        current_dir_context.push_str(&format!("- {}\n", name));
                    }
                }
            }
        }

        let git_status = Command::new("git").arg("status").arg("-s").output();
        if let Ok(output) = git_status {
            let status_str = String::from_utf8_lossy(&output.stdout);
            if !status_str.trim().is_empty() {
                current_dir_context.push_str("\n\n### GIT STATUS:\n");
                current_dir_context.push_str(&status_str);
            }
        }

        for file in &critical_files {
            if let Ok(content) = std::fs::read_to_string(file) {
                current_dir_context.push_str(&format!(
                    "\n\n### CONTENTS OF {}:\n{}\n",
                    file, content
                ));
            }
        }

        let dynamic_system_prompt = format!("{}{}", config.system_prompt.as_ref().unwrap_or(&default_prompt), current_dir_context);
        if let Some(first) = messages.first_mut() {
            if first.role == "system" {
                first.content = dynamic_system_prompt;
            }
        }

        if !autonomous_turn {
            iteration_count = 0; // Reset count on user turn
            let readline = if let Some(prompt_text) = &args.prompt {
                if messages.iter().any(|m| m.role == "user") {
                    Ok(String::new())
                } else {
                    println!("{} {}", "🟢 >>".green().bold(), prompt_text);
                    Ok(prompt_text.clone())
                }
            } else {
                let p = format!("{} ", "🟢 >>".green().bold());
                rl.readline(&p)
            };

            let text = match readline {
                Ok(line) => {
                    let _ = rl.add_history_entry(line.as_str());
                    line.trim().to_string()
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            };

            if text.is_empty() && args.prompt.is_none() {
                continue;
            }
            if text.eq_ignore_ascii_case("exit") || text.eq_ignore_ascii_case("quit") {
                break;
            }

            if text.starts_with('/') {
                match text.split_whitespace().next().unwrap_or("") {
                    "/help" => {
                        println!("\n{}", "🛠️  Help Menu".cyan().bold());
                        println!("/model <name> - Change AI model");
                        println!("/clear        - Clear screen");
                        println!("/forget       - Wipe history");
                        println!("/config       - Show settings");
                        println!("/status       - Show session info");
                        continue;
                    }
                    "/clear" => {
                        print!("\x1B[2J\x1B[1;1H");
                        io::stdout().flush()?;
                        continue;
                    }
                    "/forget" => {
                        messages.truncate(1);
                        save_history(&messages);
                        println!("🗑️ History wiped.");
                        continue;
                    }
                    "/model" => {
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        if parts.len() > 1 {
                            model = parts[1].to_string();
                            println!("Model: {}", model.cyan());
                        }
                        continue;
                    }
                    _ => {}
                }
            }

            if !text.is_empty() {
                messages.push(Message {
                    role: "user".to_string(),
                    content: text.clone(),
                });
            }
        }

        autonomous_turn = false;

        // Context Management
        let total_tokens = utils::get_total_tokens(&messages);
        if total_tokens > 10000 && messages.len() > 10 {
            let keep_count = 5; // Keep last 5 messages + system prompt
            let split_index = messages.len() - keep_count;
            let system_msg = messages[0].clone();
            let to_summarize = messages[1..split_index].to_vec();
            
            let history_tokens: usize = to_summarize.iter().map(|m| utils::count_tokens(&m.content)).sum();
            
            // Only summarize if history is significant enough to matter
            if history_tokens > 1500 {
                println!("🔄 Context usage ({} tokens) high. Summarizing {} tokens of history...", total_tokens, history_tokens);
                if let Some(summary) = summarize_context(&client, &api_key, &base_url, &model, &to_summarize).await {
                    let mut new_messages = Vec::new();
                    new_messages.push(system_msg);
                    new_messages.push(Message { 
                        role: "system".to_string(), 
                        content: format!("Summary of previous context to save tokens:\n{}", summary) 
                    });
                    new_messages.extend(messages.drain(split_index..));
                    messages = new_messages;
                    println!("✅ Context compressed successfully.");
                }
            }
        }

        let request_body = ChatRequest {
            model: model.clone(),
            messages: messages.clone(),
            stream: true,
        };
        let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let req = client
            .post(&endpoint)
            .bearer_auth(&api_key)
            .json(&request_body);
        let mut es = match EventSource::new(req) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        print!("{} ", "🤖 DeepSeek:".magenta().bold());
        io::stdout().flush()?;

        let mut full_response = String::new();
        let mut reasoning = String::new();
        let mut showed_reasoning_header = false;

        println!("{}", "---".dimmed());
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" {
                        break;
                    }
                    if let Ok(resp) = serde_json::from_str::<StreamResponse>(&message.data) {
                        if let Some(choice) = resp.choices.first() {
                            if let Some(reason) = &choice.delta.reasoning_content {
                                if !showed_reasoning_header {
                                    print!("{}", "🧠 Thinking: ".dimmed());
                                    showed_reasoning_header = true;
                                }
                                print!("{}", reason.dimmed());
                                io::stdout().flush().unwrap_or(());
                                reasoning.push_str(reason);
                            }
                            if let Some(content) = &choice.delta.content {
                                if showed_reasoning_header {
                                    println!("\n");
                                    showed_reasoning_header = false;
                                }
                                print!("{}", content);
                                io::stdout().flush().unwrap_or(());
                                full_response.push_str(content);
                            }
                        }
                    }
                }
                Err(_) => break,
                _ => {}
            }
        }
        println!("\n{}", "---".dimmed());

        let assistant_msg = if !reasoning.is_empty() {
            format!("<thought>\n{}\n</thought>\n\n{}", reasoning, full_response)
        } else {
            full_response.clone()
        };

        messages.push(Message {
            role: "assistant".to_string(),
            content: assistant_msg,
        });

        match utils::extract_action(&full_response) {
            utils::Action::Bash(cmd) => {
                let (safe, reason) = is_safe_command(&cmd);
                if !safe {
                    let msg = format!(
                        "❌ Security Alert: Dangerous command blocked. Reason: {}",
                        reason.unwrap_or_default()
                    );
                    println!("{}", msg.red().bold());
                    messages.push(Message {
                        role: "system".to_string(),
                        content: msg,
                    });
                } else {
                    let mut execute_cmd = auto_approve;
                    if !auto_approve {
                        print!(
                            "\n{} {}\nRun command? [y/N/a]: ",
                            "⚠️ Suggested:".yellow().bold(),
                            cmd.yellow()
                        );
                        io::stdout().flush()?;
                        let mut confirm = String::new();
                        io::stdin().read_line(&mut confirm)?;
                        match confirm.trim().to_lowercase().as_str() {
                            "y" => execute_cmd = true,
                            "a" => {
                                auto_approve = true;
                                execute_cmd = true;
                            }
                            _ => execute_cmd = false,
                        }
                    }
                    if execute_cmd {
                        println!("⚙️  Executing...");
                        let output = if cfg!(target_os = "windows") {
                            Command::new("cmd").arg("/C").arg(&cmd).output()
                        } else {
                            Command::new("sh").arg("-c").arg(&cmd).output()
                        };
                        let result_text = match output {
                            Ok(o) => {
                                let res = format!(
                                    "{}{}",
                                    String::from_utf8_lossy(&o.stdout),
                                    String::from_utf8_lossy(&o.stderr)
                                );
                                println!("{}\n{}", "✅ Output:".green().bold(), res.trim());
                                format!("Output:\n{}", res)
                            }
                            Err(e) => format!("Error: {}", e),
                        };
                        messages.push(Message {
                            role: "system".to_string(),
                            content: result_text,
                        });
                        autonomous_turn = true;
                    }
                }
            }
            utils::Action::Patch(filepath, old, new) => {
                println!("\n🩹 Patching file: {}", filepath.cyan());
                let result_text = match std::fs::read_to_string(&filepath) {
                    Ok(content) => {
                        if content.contains(&old) {
                            let new_content = content.replace(&old, &new);
                            match std::fs::write(&filepath, new_content) {
                                Ok(_) => {
                                    println!("✅ File patched successfully.");
                                    format!("Successfully patched {}", filepath)
                                }
                                Err(e) => format!("Failed to write to file: {}", e),
                            }
                        } else {
                            format!(
                                "Error: Could not find exact match for the 'old' content in {}",
                                filepath
                            )
                        }
                    }
                    Err(e) => format!("Failed to read file: {}", e),
                };
                messages.push(Message {
                    role: "system".to_string(),
                    content: result_text,
                });
                autonomous_turn = true;
            }
            utils::Action::Fetch(url) => {
                println!("\n🌐 Fetching: {}", url.cyan());
                let result_text = match reqwest::get(&url).await {
                    Ok(resp) => {
                        let text = resp.text().await.unwrap_or_default();
                        let clean_text =
                            html2text::from_read(text.as_bytes(), 80).unwrap_or_else(|_| text);
                        let snippet = if clean_text.len() > 3000 {
                            format!("{}... (truncated)", &clean_text[..3000])
                        } else {
                            clean_text
                        };
                        println!("✅ Fetched {} characters.", snippet.len());
                        format!("Web content (cleaned):\n{}", snippet)
                    }
                    Err(e) => format!("Fetch error: {}", e),
                };
                messages.push(Message {
                    role: "system".to_string(),
                    content: result_text,
                });
                autonomous_turn = true;
            }
            utils::Action::Read(filepath, s, e) => {
                println!("\n📖 Reading file: {}", filepath.cyan());
                let result_text = match std::fs::read_to_string(&filepath) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().collect();
                        let start = s.saturating_sub(1);
                        let mut end = e;
                        if end == 0 || end > lines.len() {
                            end = lines.len();
                        }
                        if start < lines.len() && start < end {
                            println!("✅ Read {} lines.", end - start);
                            lines[start..end].join("\n")
                        } else {
                            format!("Invalid range. File has {} lines.", lines.len())
                        }
                    }
                    Err(e) => format!("Error: {}", e),
                };
                messages.push(Message {
                    role: "system".to_string(),
                    content: result_text,
                });
                autonomous_turn = true;
            }
            utils::Action::None => {}
        }

        if autonomous_turn {
            iteration_count += 1;
            if iteration_count >= max_iterations {
                println!("\n{}", "⚠️ [Max iterations reached - Stopping for safety]".red().bold());
                autonomous_turn = false;
            } else {
                println!(
                    "\n{}",
                    format!(
                        "🔄 [Autonomous Turn {}/{} - Continuing...]",
                        iteration_count, max_iterations
                    )
                    .blue()
                    .italic()
                );
            }
        }
        save_history(&messages);
        if args.prompt.is_some() && !autonomous_turn {
            break;
        }
    }
    Ok(())
}

async fn summarize_context(
    client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    messages_to_summarize: &[Message],
) -> Option<String> {
    let mut prompt = String::from("Please summarize the following chat history concisely:\n\n");
    for m in messages_to_summarize {
        prompt.push_str(&format!("{}: {}\n", m.role, m.content));
    }
    let request_body = ChatRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
        stream: false,
    };
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    if let Ok(resp) = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
    {
        if let Ok(sync_resp) = resp.json::<models::SyncChatResponse>().await {
            if let Some(choice) = sync_resp.choices.first() {
                return Some(choice.message.content.clone());
            }
        }
    }
    None
}
