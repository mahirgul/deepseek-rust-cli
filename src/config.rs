use anyhow::Result;
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub model: String,
    pub base_url: String,
    pub request_timeout: u64,
    pub temperature: f32,
    pub top_p: f32,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
    pub max_tokens: u32,
    pub max_iterations: usize,
    pub show_token_usage: bool,
    pub concise_reasoning: bool,
    pub debug: bool,
    pub system_prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        let os = std::env::consts::OS;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if os == "windows" {
                "cmd/powershell".to_string()
            } else {
                "sh".to_string()
            }
        });

        let prompt = DEFAULT_SYSTEM_PROMPT
            .replace("{os}", os)
            .replace("{shell}", &shell);

        Self {
            model: "deepseek-v4-pro".to_string(), // Reverting to deepseek-v4-pro as requested by user
            base_url: "https://api.deepseek.com".to_string(),
            request_timeout: 6000,
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            max_tokens: 200000,
            max_iterations: 500,
            show_token_usage: true,
            concise_reasoning: true,
            debug: false,
            system_prompt: prompt,
        }
    }
}
const DEFAULT_SYSTEM_PROMPT: &str = "You are a terminal-based AI coding assistant running on {os} via {shell}.
Be concise and practical. You have full access to the workspace to read/write files and execute commands.
Explain your actions briefly and always verify file contents before modification.";

/// Initialize the .deep directory structure in the current workspace.
/// Creates .deep/ folder with config.json, memory.md, and history/ subdirectory
/// if they don't already exist.
pub fn init_workspace() {
    let deep_dir = PathBuf::from(".deep");

    // Create .deep directory if it doesn't exist
    if !deep_dir.exists() {
        fs::create_dir_all(&deep_dir).expect("Failed to create .deep directory");
        println!("📁 Created .deep/ workspace directory");
    }

    // Create history subdirectory
    let history_dir = deep_dir.join("history");
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir).expect("Failed to create .deep/history directory");
    }

    // Create config.json if it doesn't exist
    let config_path = deep_dir.join("config.json");
    if !config_path.exists() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_path, json).expect("Failed to write config.json");
        println!("📝 Created .deep/config.json with default settings");
    }

    // Create memory.md if it doesn't exist
    let memory_path = deep_dir.join("memory.md");
    if !memory_path.exists() {
        let default_memory = r#"# Local Memory

This file serves as the agent's persistent memory for this project.
You can update this file to store important context, decisions, and notes
that the AI agent should remember across sessions.

## Project Notes
- 

## Decisions
- 

## Important Context
- 
"#;
        fs::write(&memory_path, default_memory).expect("Failed to write memory.md");
        println!("📝 Created .deep/memory.md for persistent memory");
    }
}

pub fn load_config() -> Config {
    // 1. Try workspace .deep/config.json (primary)
    if let Ok(content) = fs::read_to_string(".deep/config.json") {
        if let Ok(loaded) = serde_json::from_str::<Config>(&content) {
            return loaded;
        }
        eprintln!("⚠️  Invalid config.json, using defaults");
    }

    // 2. Fallback: Try global ~/.deep/config.json
    if let Some(mut home) = dirs::home_dir() {
        home.push(".deep/config.json");
        if let Ok(content) = fs::read_to_string(home) {
            if let Ok(loaded) = serde_json::from_str::<Config>(&content) {
                return loaded;
            }
        }
    }

    Config::default()
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let path = PathBuf::from(".deep/config.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

pub fn get_api_key() -> Result<String> {
    dotenv().ok();

    // 1. Check current environment (includes workspace .env if Step 1 succeeded)
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        return Ok(key);
    }

    // 2. Check user's home directory .deep/.env
    if let Some(mut home) = dirs::home_dir() {
        home.push(".deep/.env");
        if home.exists() {
            let _ = dotenvy::from_path(&home);
            if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
                return Ok(key);
            }
        }
    }

    anyhow::bail!(
        "DEEPSEEK_API_KEY not found.\n\
         Please create ~/.deep/.env or workspace .env with:\n\
         DEEPSEEK_API_KEY=your_api_key_here"
    )
}
