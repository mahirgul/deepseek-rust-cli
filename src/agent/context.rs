use std::fs;

pub fn get_project_context() -> String {
    let mut context = String::new();
    if let Ok(entries) = fs::read_dir(".") {
        context.push_str("\n### Project Structure:\n");
        for entry in entries.flatten() {
            if let Some(name) = entry
                .file_name()
                .into_string()
                .ok()
                .filter(|n| !n.starts_with('.') && n != "target")
            {
                context.push_str(&format!("- {}\n", name));
            }
        }
    }

    // Inject local memory
    if let Ok(memory) = fs::read_to_string(".deep/memory.md") {
        context.push_str("\n### Local Memory:\n");
        context.push_str(&memory);
    }

    context
}
