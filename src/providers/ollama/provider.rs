use async_trait::async_trait;
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Client;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use super::config::OllamaConfig;
use super::ollama_api::*;

use crate::{
    AppResult,
    model::{Message, ModelProvider},
    streaming::{OutputStreamer, ProgressInfo, StreamEvent},
};

const GENERATE_API: &str = "/api/generate";
const COMPLETION_API: &str = "/v1/chat/completions";

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    // Internal method to convert context to generate API format
    fn messages_to_prompt(&self, messages: &[Message]) -> (Option<String>, String) {
        let mut system_message = None;
        let mut conversation_parts = Vec::new();

        for message in messages {
            match message.role {
                crate::model::MessageRole::System => {
                    system_message = Some(message.content.clone());
                }
                crate::model::MessageRole::User => {
                    conversation_parts.push(format!("User: {}", message.content));
                }
                crate::model::MessageRole::Assistant => {
                    conversation_parts.push(format!("Assistant: {}", message.content));
                }
            }
        }

        let prompt = conversation_parts.join("\n");
        (system_message, prompt)
    }

    async fn generate_via_generate_api(
        &self,
        messages: &[Message],
        config: &OllamaConfig,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        let (system_message, prompt) = self.messages_to_prompt(messages);

        streamer
            .handle_event(StreamEvent::Progress(ProgressInfo {
                current: 20,
                total: None,
                message: "Getting response...".to_string(),
            }))
            .await?;

        let request = OllamaGenerateRequest {
            model: config.model.clone(),
            prompt,
            stream: true,
            options: Some(config.options.clone()),
            raw: config.raw,
            template: config.template.clone(),
            system: system_message,
        };

        let response = self
            .client
            .post(format!("{}{}", config.endpoint_url(), GENERATE_API))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let mut full_response = String::new();
        let byte_stream = response.bytes_stream();
        let stream_reader = StreamReader::new(
            byte_stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = LinesStream::new(BufReader::new(stream_reader).lines());

        while let Some(line) = lines.next().await {
            match line {
                Ok(l) if !l.trim().is_empty() => {
                    match serde_json::from_str::<OllamaGenerateResponse>(&l) {
                        Ok(result) => {
                            if !result.response.is_empty() {
                                full_response.push_str(&result.response);
                                streamer
                                    .handle_event(StreamEvent::Token(result.response))
                                    .await?;
                            }

                            if result.done {
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse generate response: {}, line: {}", e, l);
                            streamer
                                .handle_event(StreamEvent::Error(format!("Parse error: {}", e)))
                                .await?;
                        }
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    log::error!("Stream error: {}", e);
                    streamer
                        .handle_event(StreamEvent::Error(format!("Stream error: {}", e)))
                        .await?;
                }
            }
        }

        Ok(full_response)
    }

    async fn generate_via_completion_api(
        &self,
        messages: &[Message],
        config: &OllamaConfig,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        streamer
            .handle_event(StreamEvent::Progress(ProgressInfo {
                current: 20,
                total: None,
                message: "Getting completion response...".to_string(),
            }))
            .await?;

        let ollama_messages: Vec<OllamaMessage> = messages.iter().map(|m| m.into()).collect();

        let request = OllamaCompletionRequest {
            model: config.model.clone(),
            messages: ollama_messages,
            stream: true,
            options: Some(config.options.clone()),
            tools: config.tools.clone(),
        };

        let response = self
            .client
            .post(format!("{}{}", config.endpoint_url(), COMPLETION_API))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let mut full_response = String::new();
        let byte_stream = response.bytes_stream();
        let stream_reader = StreamReader::new(
            byte_stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = LinesStream::new(BufReader::new(stream_reader).lines());

        while let Some(line) = lines.next().await {
            match line {
                Ok(l) if !l.trim().is_empty() => {
                    let line = l.trim();
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            break;
                        }

                        match serde_json::from_str::<OllamaCompletionResponse>(data) {
                            Ok(result) => {
                                if let Some(choice) = result.choices.first() {
                                    if let Some(delta) = &choice.delta {
                                        if let Some(content) = &delta.content {
                                            full_response.push_str(content);
                                            streamer
                                                .handle_event(StreamEvent::Token(content.clone()))
                                                .await?;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse completion: {}, data: {}", e, data);
                                streamer
                                    .handle_event(StreamEvent::Error(format!("Parse error: {}", e)))
                                    .await?;
                            }
                        }
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    log::error!("Stream error: {}", e);
                    streamer
                        .handle_event(StreamEvent::Error(format!("Stream error: {}", e)))
                        .await?;
                }
            }
        }

        Ok(full_response)
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    type Config = OllamaConfig;

    async fn generate_streaming(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<String> {
        // Try completion API first, fallback to generate API
        match self
            .generate_via_completion_api(messages, config, streamer)
            .await
        {
            Ok(response) => Ok(response),
            Err(_) => {
                log::warn!("Completion API failed, falling back to generate API");
                self.generate_via_generate_api(messages, config, streamer)
                    .await
            }
        }
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_system_messages(&self) -> bool {
        true
    }

    fn max_context_length(&self) -> Option<usize> {
        None // Depends on the specific model
    }
}
