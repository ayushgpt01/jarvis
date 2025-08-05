use super::{ModelConfig, ModelProvider, context::Context};
use crate::{AppError, streaming::OutputStreamer};

/// AI Client struct that can be used by the user
#[derive(Debug)]
pub struct AIClient<P: ModelProvider> {
    provider: P,
    config: P::Config,
    context: Context,
}

impl<P: ModelProvider> AIClient<P> {
    pub fn new(
        provider: P,
        config: P::Config,
        max_context_history: usize,
    ) -> Result<Self, AppError> {
        config.validate()?;

        Ok(Self {
            provider,
            config,
            context: Context::new(max_context_history),
        })
    }

    // Main interaction methods
    pub async fn chat_streaming(
        &mut self,
        prompt: &str,
        streamer: &mut dyn OutputStreamer,
    ) -> Result<String, AppError> {
        self.context.add_user_message(prompt.to_string());

        let messages = self.context.get_messages();
        let response = self
            .provider
            .generate_streaming(&messages, &self.config, streamer)
            .await?;

        self.context.add_assistant_message(response.clone());
        Ok(response)
    }

    pub async fn chat(&mut self, prompt: &str) -> Result<String, AppError> {
        self.context.add_user_message(prompt.to_string());

        let messages = self.context.get_messages();
        let response = self.provider.generate(&messages, &self.config).await?;

        self.context.add_assistant_message(response.clone());
        Ok(response)
    }

    // Context management
    pub fn set_system_message(&mut self, message: &str) {
        self.context.add_system_message(message.to_string());
    }

    pub fn clear_context(&mut self) {
        self.context.clear();
    }

    pub fn context_size(&self) -> usize {
        self.context.len()
    }

    pub fn get_context(&self) -> &Context {
        &self.context
    }

    // Configuration access
    pub fn config(&self) -> &P::Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut P::Config {
        &mut self.config
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }
}

/// Builder for AIClient struct that can be used by the user to create a new instance
pub struct AIClientBuilder<P: ModelProvider> {
    provider: Option<P>,
    config: Option<P::Config>,
    max_context_history: usize,
    system_message: Option<String>,
}

impl<P: ModelProvider> AIClientBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            config: None,
            max_context_history: 100,
            system_message: None,
        }
    }

    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn config(mut self, config: P::Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn max_context_history(mut self, max: usize) -> Self {
        self.max_context_history = max;
        self
    }

    pub fn system_message(mut self, message: String) -> Self {
        self.system_message = Some(message);
        self
    }

    pub fn build(self) -> Result<AIClient<P>, AppError> {
        let provider = self
            .provider
            .ok_or_else(|| AppError::from("Provider is required"))?;
        let config = self
            .config
            .ok_or_else(|| AppError::from("Config is required"))?;

        let mut client = AIClient::new(provider, config, self.max_context_history)?;

        if let Some(system_msg) = self.system_message {
            client.set_system_message(&system_msg);
        }

        Ok(client)
    }
}

impl<P: ModelProvider> Default for AIClientBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}
