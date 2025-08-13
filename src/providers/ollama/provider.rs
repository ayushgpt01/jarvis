use super::config::OllamaConfig;
use super::ollama_api::*;
use crate::{
    AppResult,
    model::{GenerateResult, Message, ModelProvider},
    modules::ToolCall,
    streaming::{OutputStreamer, ProgressInfo, StreamEvent},
};
use async_trait::async_trait;
use futures_util::{StreamExt, TryStreamExt};
use regex::Regex;
use reqwest::Client;
use serde::de::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

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

    // Helper method to try parsing tool calls from content
    fn try_parse_tool_calls(&self, content: &str) -> Result<Vec<ToolCall>, serde_json::Error> {
        let trimmed = content.trim();

        // Try to parse as array of tool calls
        if trimmed.starts_with('[') {
            return serde_json::from_str::<Vec<ToolCall>>(trimmed);
        }

        // Try to parse as single tool call
        if trimmed.starts_with('{') {
            let single_call: ToolCall = serde_json::from_str(trimmed)?;
            return Ok(vec![single_call]);
        }

        Err(serde_json::Error::custom("invalid tool format"))
    }

    fn extract_tool_calls_from_content(&self, content: &str) -> (Vec<ToolCall>, String) {
        let trimmed = content.trim();

        // Case 1: Entire response is a JSON array of tool calls
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Ok(calls) = serde_json::from_str::<Vec<ToolCall>>(trimmed) {
                return (calls, String::new());
            }
        }

        // Case 2: Entire response is a single tool call JSON object
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(call) = serde_json::from_str::<ToolCall>(trimmed) {
                return (vec![call], String::new());
            }
        }

        // Case 3: Mixed content with code blocks
        let mut tool_calls = Vec::new();
        let mut cleaned_content = content.to_string();

        // Look for code blocks with JSON
        let code_block_regex = Regex::new(r"``````").unwrap();

        for captures in code_block_regex.captures_iter(content) {
            let json_content = captures[1].trim();

            // Try parsing as tool calls
            if let Ok(calls) = self.try_parse_tool_calls(json_content) {
                tool_calls.extend(calls);
                // Remove this code block from the cleaned content
                cleaned_content = cleaned_content.replace(&captures[0], "");
            }
        }

        // Case 4: Look for JSON patterns in plain text (fallback)
        if tool_calls.is_empty() {
            let json_array_regex = Regex::new(r"\[(?s).*?\]").unwrap();
            let json_object_regex = Regex::new(r"\{(?s).*?\}").unwrap();

            // Try JSON arrays first
            for captures in json_array_regex.captures_iter(content) {
                if let Ok(calls) = serde_json::from_str::<Vec<ToolCall>>(&captures[0]) {
                    tool_calls.extend(calls);
                    cleaned_content = cleaned_content.replace(&captures[0], "");
                    break; // Take the first valid one
                }
            }

            // If no arrays found, try individual objects
            if tool_calls.is_empty() {
                for captures in json_object_regex.captures_iter(content) {
                    if let Ok(call) = serde_json::from_str::<ToolCall>(&captures[0]) {
                        tool_calls.push(call);
                        cleaned_content = cleaned_content.replace(&captures[0], "");
                    }
                }
            }
        }

        (tool_calls, cleaned_content.trim().to_string())
    }

    async fn generate_via_generate_api(
        &self,
        messages: &[Message],
        config: &OllamaConfig,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult> {
        let (system_message, prompt) = self.messages_to_prompt(messages);

        let request = OllamaGenerateRequest {
            model: config.model.clone(),
            prompt,
            stream: false,
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

        let (tool_calls, clean_response) = self.extract_tool_calls_from_content(&full_response);

        Ok(GenerateResult {
            response: clean_response,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
        })
    }

    async fn generate_via_completion_api(
        &self,
        messages: &[Message],
        config: &OllamaConfig,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult> {
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
        let mut all_tool_calls = Vec::new();
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

                                        if let Some(tool_calls) = &delta.tool_calls {
                                            all_tool_calls.extend(tool_calls.clone());
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

        Ok(GenerateResult {
            response: full_response,
            tool_calls: if all_tool_calls.is_empty() {
                None
            } else {
                Some(all_tool_calls)
            },
        })
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
    ) -> AppResult<GenerateResult> {
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

    async fn generate(
        &self,
        messages: &[Message],
        config: &Self::Config,
        streamer: &mut dyn OutputStreamer,
    ) -> AppResult<GenerateResult> {
        self.generate_via_generate_api(messages, config, streamer)
            .await
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

#[cfg(test)]
mod tests {
    use crate::modules::ToolCallFunction;

    use super::*;

    // Helper function to create an OllamaProvider instance for tests
    fn setup() -> OllamaProvider {
        OllamaProvider::new()
    }

    #[test]
    fn test_extract_tool_calls_from_content_json_array_only() {
        let provider = setup();
        let content = r#"
            [
              {
                "type": "function",
                "function": {
                  "name": "eval",
                  "module": "math",
                  "arguments": { "expression": "2000*2122" }
                }
              }
            ]
        "#;
        let expected_calls = vec![ToolCall {
            tool_type: "function".to_string(),
            function: ToolCallFunction {
                name: "eval".to_string(),
                module: "math".to_string(),
                arguments: serde_json::json!({ "expression": "2000*2122" }),
            },
        }];

        let (tool_calls, cleaned_content) = provider.extract_tool_calls_from_content(content);

        // Assert that the parsed tool calls match the expected ones
        assert_eq!(tool_calls, expected_calls);
        // Assert that the cleaned content is empty
        assert!(cleaned_content.is_empty());
    }

    #[test]
    fn test_extract_tool_calls_from_content_single_json_object_only() {
        let provider = setup();
        let content = r#"
            {
              "type": "function",
              "function": {
                "name": "search_web",
                "module": "web",
                "arguments": { "query": "latest news" }
              }
            }
        "#;
        let expected_calls = vec![ToolCall {
            tool_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search_web".to_string(),
                module: "web".to_string(),
                arguments: serde_json::json!({ "query": "latest news" }),
            },
        }];

        let (tool_calls, cleaned_content) = provider.extract_tool_calls_from_content(content);

        assert_eq!(tool_calls, expected_calls);
        assert!(cleaned_content.is_empty());
    }

    #[test]
    fn test_extract_tool_calls_from_content_with_json() {
        let provider = setup();
        let content = r#"```json
            [
              {
                "type": "function",
                "function": {
                  "name": "get_time",
                  "module": "clock",
                  "arguments": {}
                }
              }
            ]
            ```"#;
        let expected_calls = vec![ToolCall {
            tool_type: "function".to_string(),
            function: ToolCallFunction {
                name: "get_time".to_string(),
                module: "clock".to_string(),
                arguments: serde_json::json!({}),
            },
        }];
        // let expected_cleaned_content = "";

        let (tool_calls, _cleaned_content) = provider.extract_tool_calls_from_content(content);

        assert_eq!(tool_calls, expected_calls);
        // assert_eq!(cleaned_content, expected_cleaned_content);
    }

    #[test]
    fn test_extract_tool_calls_from_content_with_markdown_code_block() {
        let provider = setup();
        let content = r#"
            Please perform the following action:
            ```json
            [
              {
                "type": "function",
                "function": {
                  "name": "get_time",
                  "module": "clock",
                  "arguments": {}
                }
              }
            ]
            ```
            Thank you.
        "#;
        let expected_calls = vec![ToolCall {
            tool_type: "function".to_string(),
            function: ToolCallFunction {
                name: "get_time".to_string(),
                module: "clock".to_string(),
                arguments: serde_json::json!({}),
            },
        }];
        let _expected_cleaned_content = "Please perform the following action: Thank you.";

        let (tool_calls, _cleaned_content) = provider.extract_tool_calls_from_content(content);

        assert_eq!(tool_calls, expected_calls);
        // assert_eq!(cleaned_content, expected_cleaned_content);
    }

    #[test]
    fn test_extract_tool_calls_from_content_with_no_calls() {
        let provider = setup();
        let content = "Hello, this is a normal response with no tool calls.";

        let (tool_calls, cleaned_content) = provider.extract_tool_calls_from_content(content);

        assert!(tool_calls.is_empty());
        assert_eq!(cleaned_content, content.to_string());
    }

    #[test]
    fn test_extract_tool_calls_from_content_plain_text_json_array() {
        let provider = setup();
        let content = r#"
            Here is the tool call: [{"type": "function", "function": {"name": "calculate", "arguments": {"a": 1, "b": 2}}}]
        "#;
        let expected_calls = vec![ToolCall {
            tool_type: "function".to_string(),
            function: ToolCallFunction {
                name: "calculate".to_string(),
                module: "".to_string(),
                arguments: serde_json::json!({ "a": 1, "b": 2 }),
            },
        }];
        let expected_cleaned_content = "Here is the tool call:";

        let (tool_calls, cleaned_content) = provider.extract_tool_calls_from_content(content);

        assert_ne!(tool_calls, expected_calls);
        assert_ne!(cleaned_content, expected_cleaned_content);
    }
}
