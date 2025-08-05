use clap::{Parser, Subcommand};

use crate::{AppError, AppResult};

#[derive(Debug, Parser)]
#[command(name = "Jarvis")]
#[command(version, about = "Your personal AI agent", long_about = None)]
pub struct Cli {
    /// Enable commands execution
    #[arg(short = 'x', long)]
    pub execute: bool,

    /// Select a module to run
    #[arg(short, long)]
    pub module: Option<String>,

    /// Input file to be used with current prompt
    #[arg(short, long)]
    pub input: Option<String>,

    /// Subcommands (e.g., chat)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Text prompt to be executed
    #[arg(required=false, num_args=1..)]
    pub prompt: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start a personal chat session
    Chat,
}

impl Cli {
    pub fn text(&self) -> AppResult<Option<String>> {
        // if prompt is empty, return error
        if self.prompt.is_empty() {
            if self.command.is_some() {
                return Ok(None);
            }

            Err(AppError::InvalidInput)
        } else {
            Ok(Some(self.prompt.join(" ")))
        }
    }
}
