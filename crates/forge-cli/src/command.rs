use clap::Parser;

use crate::log::LogLevel;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// API Key to be used
    #[arg(short, long)]
    pub key: String,

    /// Model to be used
    #[arg(short, long)]
    pub model: Option<String>,

    /// Base URL to be used
    #[arg(short, long)]
    pub base_url: Option<String>,

    /// Log level to use
    #[arg(long)]
    pub log_level: Option<LogLevel>,
}
