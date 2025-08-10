use clap::Parser;
use dotenv::dotenv;

mod cli;
mod core;
mod error;
mod model;
mod modules;
mod providers;
mod streaming;
mod utils;

pub use crate::cli::{Cli, Commands};
pub use crate::error::AppError;
pub type AppResult<T, E = crate::error::AppError> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> AppResult<()> {
    dotenv().ok();
    let _ = utils::logger_init();
    log::info!("Starting Program...");

    let registry = modules::ModuleRegistry::new();
    let modules = registry.list_modules();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat) => {
            log::info!("Starting chat...");
        }
        None => {
            core::process_prompt(&cli, &registry).await?;
        }
    }

    Ok(())
}
