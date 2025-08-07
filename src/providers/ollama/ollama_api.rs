use crate::model::{Message, MessageRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelOptions {
    pub num_ctx: Option<i32>,
    pub repeat_last_n: Option<i32>,
    pub repeat_penalty: Option<f32>,
    pub temperature: Option<f32>,
    pub seed: Option<i32>,
    pub stop: Option<String>,
    pub num_predict: Option<i32>,
    pub top_k: Option<i32>,
    pub top_p: Option<f32>,
    pub min_p: Option<f32>,
}

impl Default for OllamaModelOptions {
    fn default() -> Self {
        Self {
            num_ctx: Some(4096),
            repeat_last_n: Some(64),
            repeat_penalty: Some(1.1),
            temperature: Some(0.8),
            seed: Some(-1),
            stop: None,
            num_predict: Some(-1),
            top_k: Some(40),
            top_p: Some(0.9),
            min_p: Some(0.0),
        }
    }
}

// Generate API types
#[derive(Debug, Serialize)]
pub struct OllamaGenerateRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: Option<OllamaModelOptions>,
    pub raw: bool,
    pub template: Option<String>,
    pub system: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResponse {
    pub response: String,
    pub done: bool,
}

// Completions API types
#[derive(Debug, Serialize)]
pub struct OllamaCompletionRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    pub stream: bool,
    pub options: Option<OllamaModelOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

impl From<&Message> for OllamaMessage {
    fn from(msg: &Message) -> Self {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };

        Self {
            role: role.to_string(),
            content: msg.content.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OllamaCompletionChoice {
    #[serde(skip_deserializing)]
    /// Used for non-streamed content
    pub message: Option<OllamaCompletionMessage>,

    /// Used for streamed content
    pub delta: Option<OllamaCompletionDelta>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaCompletionMessage {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaCompletionDelta {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaCompletionUsage {
    #[serde(skip_deserializing)]
    pub prompt_tokens: i32,
    #[serde(skip_deserializing)]
    pub completion_tokens: i32,
    #[serde(skip_deserializing)]
    pub total_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct OllamaCompletionResponse {
    #[serde(skip_deserializing)]
    pub id: String,
    #[serde(skip_deserializing)]
    pub object: String,
    #[serde(skip_deserializing)]
    pub created: i64,
    #[serde(skip_deserializing)]
    pub model: String,
    #[serde(skip_deserializing)]
    pub system_fingerprint: Option<String>,

    /// Contains all the generated completions
    pub choices: Vec<OllamaCompletionChoice>,

    #[serde(skip_deserializing)]
    pub usage: Option<OllamaCompletionUsage>,
}
