use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "A CLI tool that connects to the DeepSeek API as an AI agent."
)]
pub struct Args {
    #[arg(short, long)]
    pub model: Option<String>,

    /// Prompt for running a one-shot command (Sub-agent)
    #[arg(short, long)]
    pub prompt: Option<String>,
}
