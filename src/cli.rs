use clap::Parser;

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
}
