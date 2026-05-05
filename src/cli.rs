use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Session ID to resume
    #[arg(short, long)]
    pub session: Option<String>,

    /// AI Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    /// Debug mode
    #[arg(short, long)]
    pub debug: bool,

    /// Auto-approve all tools
    #[arg(short, long)]
    pub auto_approve: bool,

    /// Accept invalid TLS certificates (for corporate proxies / MITM appliances)
    #[arg(long)]
    pub danger_accept_invalid_certs: bool,

    /// Generate shell completions
    #[arg(long, value_enum)]
    pub generate_completion: Option<ShellType>,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    #[clap(name = "powershell")]
    PowerShell,
    #[clap(name = "elvish")]
    Elvish,
}
