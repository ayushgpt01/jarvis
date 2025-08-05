use crate::{AppResult, streaming::OutputStreamer};
use serde::{Deserialize, Serialize};

pub mod client;
mod context;

/// Message Role enum for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

/// Context Message struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// Core Model Config Trait that all provider configs must implement
pub trait ModelConfig: Send + Sync + Clone {
    /// Returns the selected model name
    fn model_name(&self) -> &str;

    /// Validates the config
    fn validate(&self) -> AppResult<()>;
}

/// Core Model Provider Trait that all providers must implement
#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    /// Ensures that config also follows the ModelConfig trait
    type Config: ModelConfig;

    /// Method to generate a stream response
    async fn generate_streaming(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String>;

    /// Method to generate a normal response. Default implementation is of CLI streaming
    async fn generate(&self, messages: &[Message], config: &Self::Config) -> AppResult<String> {
        use crate::streaming::create_cli_streamer;

        let mut cli_streamer = create_cli_streamer(false);
        self.generate_streaming(messages, config, &mut cli_streamer)
            .await?;

        Ok("".to_string())
    }

    /// Returns the provider name
    fn provider_name(&self) -> &'static str;

    /// Returns true if the provider supports streaming
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Returns true if the provider supports system messages
    fn supports_system_messages(&self) -> bool {
        true
    }

    /// Returns the maximum context length
    fn max_context_length(&self) -> Option<usize> {
        None
    }
}
