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

    fn validate(&self) -> AppResult<()>;
}

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    type Config: ModelConfig;

    async fn generate_streaming(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult>;

    async fn generate(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult>;

    fn provider_name(&self) -> &'static str;

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_system_messages(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_length(&self) -> Option<usize> {
        None
    }
}
