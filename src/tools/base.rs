use crate::agent::agent::UndoAction;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(
        &self,
        args: &HashMap<String, Value>,
        undo_stack: &mut Vec<UndoAction>,
        cwd: Option<&Path>,
    ) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub async fn execute(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
        undo_stack: &mut Vec<UndoAction>,
        cwd: Option<&Path>,
    ) -> Result<String> {
        if let Some(tool) = self.tools.get(name) {
            tool.execute(args, undo_stack, cwd).await
        } else {
            Err(anyhow::anyhow!("Tool '{}' not found", name))
        }
    }
}

pub fn validate_path(path: &str) -> Result<std::path::PathBuf> {
    let p = std::path::PathBuf::from(path);
    let abs = if p.is_absolute() {
        p
    } else {
        let mut a = std::env::current_dir()?;
        a.push(p);
        a
    };

    // Canonicalize to resolve '..' and symlinks
    let canonical = match std::fs::canonicalize(&abs) {
        Ok(c) => c,
        Err(_) => {
            // If it doesn't exist, we still want to check the parent
            if let Some(parent) = abs.parent() {
                let can_parent = std::fs::canonicalize(parent)?;
                can_parent.join(abs.file_name().unwrap_or_default())
            } else {
                abs
            }
        }
    };

    let cwd = std::env::current_dir()?;
    if !canonical.starts_with(&cwd) && !path.is_empty() {
        anyhow::bail!("Path traversal detected: access to '{}' is denied", path);
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockTool;
    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }
        async fn execute(
            &self,
            args: &HashMap<String, Value>,
            _undo: &mut Vec<UndoAction>,
            _cwd: Option<&Path>,
        ) -> Result<String> {
            let val = args
                .get("val")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            Ok(format!("mock: {}", val))
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool));

        let mut args = HashMap::new();
        args.insert("val".to_string(), json!("hello"));

        let mut undo = Vec::new();
        let res = registry
            .execute("mock_tool", &args, &mut undo, None)
            .await
            .unwrap();
        assert_eq!(res, "mock: hello");

        let res_err = registry.execute("unknown", &args, &mut undo, None).await;
        assert!(res_err.is_err());
    }

    #[test]
    fn test_validate_path() {
        let p = validate_path("test.txt").unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with("test.txt"));
    }
}
