use std::{fs, path::PathBuf};

use anyhow::Result;
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub model: String,
    pub base_url: String,
    pub request_timeout: u64,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub proxy_username: Option<String>,
    #[serde(default)]
    pub proxy_password: Option<String>,
    #[serde(default)]
    pub danger_accept_invalid_certs: bool,
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
    #[serde(default = "default_max_tool_output_chars")]
    pub max_tool_output_chars: usize,
    #[serde(default = "default_max_context_chars")]
    pub max_context_chars: usize,
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
            model: "deepseek-v4-pro".to_string(), /* Reverting to deepseek-v4-pro as requested by
                                                   * user */
            base_url: "https://api.deepseek.com".to_string(),
            request_timeout: 6000, // 100 minutes
            proxy_url: None,
            proxy_username: None,
            proxy_password: None,
            danger_accept_invalid_certs: false,
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            max_tokens: 16_384, // 16K — sufficient for practical use; saves completion tokens
            max_iterations: 500,
            show_token_usage: true,
            concise_reasoning: true,
            debug: false,
            system_prompt: prompt,
            max_tool_output_chars: 15000,
            max_context_chars: 100000,
        }
    }
}

fn default_max_tool_output_chars() -> usize {
    15000
}

fn default_max_context_chars() -> usize {
    100000
}
const DEFAULT_SYSTEM_PROMPT: &str = "You are a terminal-based AI coding assistant running on {os} \
                                     via {shell}.
Be concise and practical. You have full access to the workspace to read/write files and execute \
                                     commands.
Explain your actions briefly and always verify file contents before modification.";

/// Initialize the .deep directory structure in the current workspace.
/// Creates .deep/ folder with config.json, memory.md, and history/ subdirectory
/// if they don't already exist.
pub fn init_workspace() {
    let deep_dir = PathBuf::from(".deep");

    // Create .deep directory if it doesn't exist
    if !deep_dir.exists() {
        let _ = fs::create_dir_all(&deep_dir);
    }

    // Create history subdirectory
    let history_dir = deep_dir.join("history");
    if !history_dir.exists() {
        let _ = fs::create_dir_all(&history_dir);
    }

    // Create config.json if it doesn't exist
    let config_path = deep_dir.join("config.json");
    if !config_path.exists() {
        let config = Config::default();
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&config_path, json);
        }
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
        let _ = fs::write(&memory_path, default_memory);
    }
}

pub fn load_config() -> Config {
    // 1. Try workspace .deep/config.json (primary)
    if let Ok(content) = fs::read_to_string(".deep/config.json") {
        if let Ok(loaded) = serde_json::from_str::<Config>(&content) {
            return loaded;
        }
    }

    // 2. Fallback: Try global ~/.deep/config.json
    if let Some(mut home) = dirs::home_dir() {
        home.push(".deep/config.json");
        if let Some(loaded) = fs::read_to_string(home)
            .ok()
            .and_then(|c| serde_json::from_str::<Config>(&c).ok())
        {
            return loaded;
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
        "DEEPSEEK_API_KEY not found.\nPlease create ~/.deep/.env or workspace .env \
         with:\nDEEPSEEK_API_KEY=your_api_key_here"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model, "deepseek-v4-pro");
        assert!(config.temperature >= 0.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config.model, decoded.model);
        assert_eq!(config.max_tool_output_chars, decoded.max_tool_output_chars);
        assert_eq!(config.max_context_chars, decoded.max_context_chars);
    }

    #[test]
    fn test_config_backward_compatibility() {
        let json = r#"{"model":"test-model","base_url":"http://test","request_timeout":10,"temperature":0.5,"top_p":0.9,"presence_penalty":0.0,"frequency_penalty":0.0,"max_tokens":1000,"max_iterations":5,"show_token_usage":false,"concise_reasoning":false,"debug":false,"system_prompt":"sys"}"#;
        let decoded: Config = serde_json::from_str(json).unwrap();
        assert_eq!(decoded.max_tool_output_chars, 15000);
        assert_eq!(decoded.max_context_chars, 100000);
    }
}
