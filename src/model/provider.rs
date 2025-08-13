use super::Message;
use crate::{AppResult, modules::ToolCall, streaming::OutputStreamer};

#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub response: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

pub trait ModelConfig: Send + Sync + Clone {
    #[allow(dead_code)]
    fn model_name(&self) -> &str;

    #[allow(dead_code)]
    fn validate(&self) -> AppResult<()>;
}

#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    type Config: ModelConfig;

    #[allow(dead_code)]
    async fn generate_streaming(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult>;

    #[allow(dead_code)]
    async fn generate(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult>;

    #[allow(dead_code)]
    fn provider_name(&self) -> &'static str;

    #[allow(dead_code)]
    fn supports_streaming(&self) -> bool {
        true
    }

    #[allow(dead_code)]
    fn supports_system_messages(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    #[allow(dead_code)]
    fn max_context_length(&self) -> Option<usize> {
        None
    }
}
