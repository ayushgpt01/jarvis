use super::{Context, ModelConfig, ModelProvider};
use crate::{
    AppError, AppResult,
    modules::{ModuleRegistry, ToolCall},
    streaming::{NullStreamer, OutputStreamer, ProgressInfo, StreamEvent},
};
use std::sync::Arc;

#[derive(Debug)]
pub struct AIClient<P: ModelProvider> {
    provider: P,
    config: P::Config,
    context: Context,
    registry: Arc<ModuleRegistry>,
}

#[allow(dead_code)]
impl<P: ModelProvider> AIClient<P> {
    pub fn new() -> AIClientBuilder<P> {
        AIClientBuilder {
            provider: None,
            config: None,
            modules: None,
            max_context_history: 100,
            system_message: None,
        }
    }

    async fn execute_tool_calls(
        &mut self,
        tool_calls: &[ToolCall],
    ) -> AppResult<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        let mut tool_result_context = Vec::new();

        for tool_call in tool_calls {
            let result = self.registry.execute(&tool_call.function)?;

            results.push(result.clone());

            // Format tool results more naturally for context
            let context_message = format!(
                "{{ \"name\": \"{}\", \"module\": \"{}\", \"result\": {} }}",
                tool_call.function.name, tool_call.function.module, result
            );

            tool_result_context.push(context_message.clone());

            // streamer
            //     .handle_event(StreamEvent::Token(format!("\n[{}]\n", context_message)))
            //     .await?;
        }

        // Add a single, clear context message instead of multiple confusing ones
        if !tool_result_context.is_empty() {
            let combined_context = tool_result_context.join("; ");
            let result = format!("TOOL_RESULT: {}", combined_context);

            log::debug!("Tool message : {:#?}", result);
            self.context.add_assistant_message(result);
        }

        Ok(results)
    }

    pub async fn chat_streaming_with_tools(
        &mut self,
        prompt: &str,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        self.context.add_user_message(prompt.to_string());

        let max_iterations = 10;
        let mut iteration = 0;
        let mut final_response = String::new();

        streamer
            .handle_event(StreamEvent::Progress(ProgressInfo {
                current: 20,
                total: None,
                message: "Getting response...".to_string(),
            }))
            .await?;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                streamer
                    .handle_event(StreamEvent::Error(
                        "Maximum tool chain iterations reached".to_string(),
                    ))
                    .await?;

                break;
            }

            let messages = self.context.get_messages();
            log::debug!("Messages : {:#?}", messages);

            let result = self
                .provider
                .generate(&messages, &self.config, &mut NullStreamer::new())
                .await?;

            log::debug!("GenerateResult : {:#?}", result);

            // TODO - Check this later on if it is needed. This might need to be sanitized
            final_response.push_str(&result.response);

            if !result.response.is_empty() {
                self.context.add_assistant_message(result.response.clone());
            }

            if let Some(tool_calls) = &result.tool_calls {
                let tool_calls_json = serde_json::to_string(tool_calls)?;
                self.context.add_assistant_message(tool_calls_json);
                self.execute_tool_calls(&tool_calls).await?;
            } else {
                self.context.add_assistant_message(result.response.clone());

                for c in result.response.chars() {
                    streamer
                        .handle_event(StreamEvent::Token(c.to_string()))
                        .await?;
                }

                break;
            }
        }

        Ok(final_response)
    }

    pub async fn chat_streaming(
        &mut self,
        prompt: &str,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        if self.provider.supports_tools() {
            return self.chat_streaming_with_tools(prompt, streamer).await;
        }

        self.context.add_user_message(prompt.to_string());

        let messages = self.context.get_messages();
        let result = self
            .provider
            .generate_streaming(&messages, &self.config, streamer)
            .await?;

        self.context.add_assistant_message(result.response.clone());
        Ok(result.response)
    }

    pub async fn chat(
        &mut self,
        prompt: &str,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        self.context.add_user_message(prompt.to_string());

        let messages = self.context.get_messages();
        let result = self
            .provider
            .generate(&messages, &self.config, streamer)
            .await?;

        self.context.add_assistant_message(result.response.clone());
        Ok(result.response)
    }

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

    pub fn config(&self) -> &P::Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut P::Config {
        &mut self.config
    }

    pub fn registry(&self) -> &Arc<ModuleRegistry> {
        &self.registry
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }
}

pub struct AIClientBuilder<P: ModelProvider> {
    provider: Option<P>,
    config: Option<P::Config>,
    max_context_history: usize,
    modules: Option<Arc<ModuleRegistry>>,
    system_message: Option<String>,
}

#[allow(dead_code)]
impl<P: ModelProvider> AIClientBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            config: None,
            modules: None,
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

    pub fn modules(mut self, modules: Arc<ModuleRegistry>) -> Self {
        self.modules = Some(modules);
        self
    }

    pub fn build(self) -> AppResult<AIClient<P>> {
        let provider = self
            .provider
            .ok_or_else(|| AppError::from("Provider is required"))?;
        let config = self
            .config
            .ok_or_else(|| AppError::from("Config is required"))?;
        let modules = self
            .modules
            .unwrap_or(Arc::new(ModuleRegistry::empty_registry()));

        config.validate()?;

        let mut client = AIClient {
            provider,
            config,
            context: Context::new(self.max_context_history),
            registry: modules,
        };

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
