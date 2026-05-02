use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

pub enum InputResult {
    Text(String),
    Exit,
    EOF,
}

pub struct InputHandler {
    prompt: String,
    history: Vec<String>,
    history_index: usize,
    history_path: PathBuf,
}

impl InputHandler {
    pub fn new() -> Self {
        let history_path = PathBuf::from(".deep/input_history.json");
        let history = if let Ok(content) = fs::read_to_string(&history_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            prompt: ">> ".to_string(),
            history_index: history.len(),
            history,
            history_path,
        }
    }

    fn save_history(&self) {
        if let Ok(json) = serde_json::to_string(&self.history) {
            let _ = fs::write(&self.history_path, json);
        }
    }

    pub fn read_line(&mut self) -> io::Result<InputResult> {
        terminal::enable_raw_mode()?;
        let result = self.read_raw();
        terminal::disable_raw_mode()?;

        if let Ok(InputResult::Text(ref text)) = result {
            if !text.trim().is_empty()
                && (self.history.is_empty() || self.history.last().unwrap() != text)
            {
                self.history.push(text.clone());
                self.save_history();
            }
        }
        self.history_index = self.history.len();

        result
    }

    fn read_raw(&mut self) -> io::Result<InputResult> {
        let mut buffer = String::new();
        let mut cursor_pos = 0; // Measured in CHARACTERS
        let mut stdout = io::stdout();

        print!("\x1b[1;36m{}\x1b[0m", self.prompt);
        stdout.flush()?;

        loop {
            if event::poll(Duration::from_millis(10))? {
                if let Event::Key(KeyEvent {
                    code, modifiers, ..
                }) = event::read()?
                {
                    match code {
                        KeyCode::Enter => {
                            if modifiers.contains(KeyModifiers::ALT) {
                                let char_idx = buffer
                                    .char_indices()
                                    .nth(cursor_pos)
                                    .map(|(i, _)| i)
                                    .unwrap_or(buffer.len());
                                buffer.insert(char_idx, '\n');
                                cursor_pos += 1;
                                self.redraw(&buffer, cursor_pos)?;
                            } else {
                                print!("\r\n");
                                return Ok(InputResult::Text(buffer));
                            }
                        }
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                            println!("^C");
                            return Ok(InputResult::Exit);
                        }
                        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                            println!();
                            return Ok(InputResult::EOF);
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let char_idx = buffer
                                    .char_indices()
                                    .nth(cursor_pos)
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                buffer.remove(char_idx);
                                self.redraw(&buffer, cursor_pos)?;
                            }
                        }
                        KeyCode::Left => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                execute!(stdout, cursor::MoveLeft(1))?;
                            }
                        }
                        KeyCode::Right => {
                            if cursor_pos < buffer.chars().count() {
                                cursor_pos += 1;
                                execute!(stdout, cursor::MoveRight(1))?;
                            }
                        }
                        KeyCode::Up => {
                            if self.history_index > 0 {
                                self.history_index -= 1;
                                buffer = self.history[self.history_index].clone();
                                cursor_pos = buffer.chars().count();
                                self.redraw(&buffer, cursor_pos)?;
                            }
                        }
                        KeyCode::Down => {
                            if self.history_index < self.history.len() {
                                self.history_index += 1;
                                if self.history_index < self.history.len() {
                                    buffer = self.history[self.history_index].clone();
                                } else {
                                    buffer.clear();
                                }
                                cursor_pos = buffer.chars().count();
                                self.redraw(&buffer, cursor_pos)?;
                            }
                        }
                        KeyCode::Char(c) => {
                            let char_idx = buffer
                                .char_indices()
                                .nth(cursor_pos)
                                .map(|(i, _)| i)
                                .unwrap_or(buffer.len());
                            buffer.insert(char_idx, c);
                            cursor_pos += 1;
                            self.redraw(&buffer, cursor_pos)?;
                        }
                        _ => {}
                    }
                    stdout.flush()?;
                }
            }
        }
    }

    fn redraw(&self, buffer: &str, cursor_pos: usize) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::UntilNewLine)
        )?;
        print!("\x1b[1;36m{}\x1b[0m{}", self.prompt, buffer);
        let char_count = buffer.chars().count();
        let move_left = char_count - cursor_pos;
        if move_left > 0 {
            execute!(stdout, cursor::MoveLeft(move_left as u16))?;
        }
        Ok(())
    }
}
