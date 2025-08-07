use serde::{Deserialize, Serialize};

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
