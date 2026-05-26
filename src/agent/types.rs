use crate::api::types::TokenUsage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApprovalResult {
    Yes,
    No,
    Always,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Reasoning {
        content: String,
    },
    Content {
        content: String,
    },
    ToolStart {
        name: String,
        args: String,
    },
    ToolEnd {
        name: String,
        /// Truncated result output for display (colorized by TUI)
        result: Option<String>,
    },
    Error {
        content: String,
    },
    ApprovalRequest {
        name: String,
        args: String,
    },
    Done {
        token_usage: TokenUsage,
    },
    Aborted {
        token_usage: TokenUsage,
    },
}

#[derive(Debug, Clone)]
pub struct UndoAction {
    pub r#type: String,
    pub path: String,
    /// For 'write', 'replace', 'delete': contains file content.
    /// For 'rename': contains the original source path as bytes.
    pub backup: Option<Vec<u8>>,
}
