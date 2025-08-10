use crate::modules::ModuleError;
use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Invalid CLI input")]
    InvalidInput,

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Unknown error: {0}")]
    Other(String),

    #[error("Unknown function: {0}")]
    ModuleError(String),
}

impl AppError {
    pub fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

impl From<ModuleError> for AppError {
    fn from(e: ModuleError) -> Self {
        AppError::ModuleError(e.to_string())
    }
}
