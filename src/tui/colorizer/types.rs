/// Detected language for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodeLang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Shell,
    Json,
    Toml,
    Yaml,
    Html,
    Css,
    Sql,
    Markdown,
    /// Fallback: highlight strings, comments, numbers
    Generic,
}

impl CodeLang {
    /// Detect language from a filename or extension
    pub fn from_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".rs") {
            CodeLang::Rust
        } else if lower.ends_with(".py") || lower.ends_with(".pyw") {
            CodeLang::Python
        } else if lower.ends_with(".js") || lower.ends_with(".mjs") || lower.ends_with(".cjs") {
            CodeLang::JavaScript
        } else if lower.ends_with(".ts") || lower.ends_with(".tsx") || lower.ends_with(".mts") {
            CodeLang::TypeScript
        } else if lower.ends_with(".go") {
            CodeLang::Go
        } else if lower.ends_with(".java") || lower.ends_with(".kt") || lower.ends_with(".scala") {
            CodeLang::Java
        } else if lower.ends_with(".c") || lower.ends_with(".h") {
            CodeLang::C
        } else if lower.ends_with(".cpp")
            || lower.ends_with(".cc")
            || lower.ends_with(".cxx")
            || lower.ends_with(".hpp")
            || lower.ends_with(".hh")
        {
            CodeLang::Cpp
        } else if lower.ends_with(".sh")
            || lower.ends_with(".bash")
            || lower.ends_with(".zsh")
            || lower.ends_with(".fish")
            || lower.ends_with(".ps1")
            || lower.ends_with(".bat")
        {
            CodeLang::Shell
        } else if lower.ends_with(".json") {
            CodeLang::Json
        } else if lower.ends_with(".toml") {
            CodeLang::Toml
        } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
            CodeLang::Yaml
        } else if lower.ends_with(".html")
            || lower.ends_with(".htm")
            || lower.ends_with(".xml")
            || lower.ends_with(".svg")
        {
            CodeLang::Html
        } else if lower.ends_with(".css") || lower.ends_with(".scss") || lower.ends_with(".less") {
            CodeLang::Css
        } else if lower.ends_with(".sql") || lower.ends_with(".psql") {
            CodeLang::Sql
        } else if lower.ends_with(".md") || lower.ends_with(".mdx") {
            CodeLang::Markdown
        } else {
            CodeLang::Generic
        }
    }

    /// Detect language from a tool name
    pub fn from_tool(tool_name: &str) -> Self {
        match tool_name {
            "run_python_code" => CodeLang::Python,
            "execute_shell_command" => CodeLang::Shell,
            "read_local_file" => CodeLang::Generic, // determined later from path
            "github_get_file" => CodeLang::Generic,
            _ => CodeLang::Generic,
        }
    }
}

/// Parse state carried across chunks
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Normal,
    /// Inside inline code `` ` ``
    InlineCode,
    /// Inside a fenced code block (language tag stored)
    FencedBlock {
        lang: String,
    },
}
