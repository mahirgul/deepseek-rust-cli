use std::{sync::Arc, time::Instant};

use crate::api::types::TokenUsage;

#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub width: u16,
    pub height: u16,
}

pub fn load_global_history() -> Vec<String> {
    let path = std::path::PathBuf::from(".deep/input_history.json");
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(history) = serde_json::from_str::<Vec<String>>(&content) {
            return history;
        }
    }
    Vec::new()
}

pub fn save_global_history(history: &[String]) {
    let path = std::path::PathBuf::from(".deep/input_history.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = std::fs::write(path, json);
    }
}

pub struct App {
    pub input: String,
    /// Byte position of cursor within input (0 = start)
    pub cursor_pos: usize,
    pub awaiting_approval: bool,
    pub spinner_frame: usize,
    pub current_task: Option<String>,
    /// When the current *task label* last changed (for display only)
    pub task_start_time: Option<Instant>,
    /// When the entire agent job started (never reset until finish_task)
    pub job_start_time: Option<Instant>,
    pub cwd: String,
    pub model: String,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    /// Footer is always 4 lines: status, folder+token, input, queue
    pub footer_height: u16,
    pub token_usage: TokenUsage,
    /// When true, ignore incoming AgentEvents (after abort)
    pub aborted: bool,
    /// Pending commands that were sent but not yet completed.
    /// Index 0 is the currently-running command; the rest are queued.
    pub queued_commands: Vec<String>,
    pub log_x: u16,
    pub log_y: u16,
    pub reasoning_started: bool,
    pub content_started: bool,
    pub is_path_traversal_warning: bool,
    pub terminal_size: Arc<std::sync::RwLock<TerminalSize>>,
    pub output_buffer: String,
    pub last_key_time: Option<Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let history = load_global_history();
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        Self {
            input: String::new(),
            cursor_pos: 0,
            awaiting_approval: false,
            spinner_frame: 0,
            current_task: None,
            task_start_time: None,
            job_start_time: None,
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            model: String::from("unknown"),
            history,
            history_index: None,
            footer_height: 4, // Fixed: Status, Folder+Token, Input, Queue
            token_usage: TokenUsage::default(),
            aborted: false,
            queued_commands: Vec::new(),
            log_x: 0,
            log_y: 0,
            reasoning_started: false,
            content_started: false,
            is_path_traversal_warning: false,
            terminal_size: Arc::new(std::sync::RwLock::new(TerminalSize { width, height })),
            output_buffer: String::new(),
            last_key_time: None,
        }
    }

    pub fn next_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(i) => {
                if i > 0 {
                    Some(i - 1)
                } else {
                    Some(0)
                }
            }
            None => Some(self.history.len().saturating_sub(1)),
        };
        if let Some(idx) = next_index {
            self.history_index = Some(idx);
            self.input = self.history[idx].clone();
            self.cursor_pos = self.input.len();
        }
    }

    pub fn prev_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(i) => {
                if i < self.history.len().saturating_sub(1) {
                    Some(i + 1)
                } else {
                    self.input.clear();
                    self.cursor_pos = 0;
                    None
                }
            }
            None => None,
        };
        self.history_index = next_index;
        if let Some(idx) = self.history_index {
            self.input = self.history[idx].clone();
            self.cursor_pos = self.input.len();
        }
    }

    pub fn start_task(&mut self, task: String) {
        // Set job-wide timer once at the beginning of the agent job
        if self.job_start_time.is_none() {
            self.job_start_time = Some(Instant::now());
        }
        if self.current_task.as_ref() != Some(&task) {
            self.current_task = Some(task);
            self.task_start_time = Some(Instant::now());
        }
    }

    pub fn finish_task(&mut self) {
        self.current_task = None;
        self.task_start_time = None;
        self.job_start_time = None;
        self.awaiting_approval = false;
        self.is_path_traversal_warning = false;
        self.aborted = false;
        // Pop the just-completed command from queue
        if !self.queued_commands.is_empty() {
            self.queued_commands.remove(0);
        }
    }

    pub fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Total tokens used so far
    pub fn total_tokens(&self) -> u64 {
        self.token_usage.prompt_tokens + self.token_usage.completion_tokens
    }
}
