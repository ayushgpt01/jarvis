use super::Message;
use crate::{AppResult, streaming::OutputStreamer};

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
    ) -> AppResult<String>;

    #[allow(dead_code)]
    async fn generate(&self, messages: &[Message], config: &Self::Config) -> AppResult<String> {
        use crate::streaming::create_cli_streamer;

        let mut cli_streamer = create_cli_streamer(false);
        self.generate_streaming(messages, config, &mut cli_streamer)
            .await?;

        Ok("".to_string())
    }

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

    #[allow(dead_code)]
    fn max_context_length(&self) -> Option<usize> {
        None
    }
}
