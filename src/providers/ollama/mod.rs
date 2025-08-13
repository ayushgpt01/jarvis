mod config;
mod ollama_api;
mod provider;

use std::sync::Arc;

use crate::{AppResult, modules::ModuleRegistry};

pub use config::OllamaConfig;
pub use provider::OllamaProvider;

pub type OllamaClient = crate::model::AIClient<provider::OllamaProvider>;

pub fn create_ollama_client(
    config: OllamaConfig,
    modules: Arc<ModuleRegistry>,
) -> AppResult<OllamaClient> {
    let client = OllamaClient::new()
        .config(config)
        .provider(OllamaProvider::new())
        .modules(modules)
        .build()?;

    Ok(client)
}
