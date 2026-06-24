use std::fs;

pub fn get_project_context() -> String {
    let mut context = String::new();

    if let Ok(cwd) = std::env::current_dir() {
        context.push_str(&format!(
            "### Current Working Directory:\n{}\n\n",
            cwd.display()
        ));
    }

    // Inject local memory (only — project structure listing removed to save tokens)
    if let Ok(memory) = fs::read_to_string(".deep/memory.md") {
        if !memory.trim().is_empty() {
            context.push_str("### Local Memory:\n");
            context.push_str(&memory);
        }
    }

    context
}
