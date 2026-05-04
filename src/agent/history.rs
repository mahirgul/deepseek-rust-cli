use std::{fs, path::PathBuf};

use crate::api::types::Message;

pub fn load_history(session_id: &str) -> Vec<Message> {
    let path = get_history_path(session_id);
    if let Some(msgs) = fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<Vec<Message>>(&c).ok())
    {
        return msgs;
    }
    Vec::new()
}

pub fn save_history(session_id: &str, messages: &[Message]) {
    let path = get_history_path(session_id);
    let _ = fs::create_dir_all(path.parent().unwrap());
    if let Ok(json) = serde_json::to_string_pretty(messages) {
        let _ = fs::write(path, json);
    }
}

fn get_history_path(session_id: &str) -> PathBuf {
    let mut path = PathBuf::from(".deep/history");
    path.push(format!("{}.json", session_id));
    path
}
