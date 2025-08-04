use clap::Parser;
use dotenv::dotenv;

use crate::{
    cli::{Cli, Commands},
    error::AppError,
};

mod cli;
mod core;
mod error;
mod model;
mod modules;
mod providers;
mod streaming;
mod utils;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    let _ = utils::logger::init();
    log::info!("Starting Program...");

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat) => {
            log::info!("Starting chat...");
        }
        None => {
            let mut streamer = streaming::create_cli_streamer(false);
            core::agent::process_prompt(&cli, &mut streamer).await?;
        }
    }

    Ok(())
}
