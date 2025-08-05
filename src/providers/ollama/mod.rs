use crate::providers::ollama::client::OllamaProvider;

pub mod client;
pub mod config;
pub mod types;

pub type OllamaClient = crate::model::client::AIClient<OllamaProvider>;
