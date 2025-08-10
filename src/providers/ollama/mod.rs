mod config;
mod ollama_api;
mod provider;

pub use config::OllamaConfig;
pub use ollama_api::OllamaModelOptions;
pub use provider::OllamaProvider;

use crate::AppResult;
pub type OllamaClient = crate::model::AIClient<provider::OllamaProvider>;

pub fn create_ollama_client(config: OllamaConfig) -> AppResult<OllamaClient> {
    let client = OllamaClient::new()
        .config(config)
        .provider(OllamaProvider::new())
        .build()?;

    Ok(client)
}
