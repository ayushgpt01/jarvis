use crate::AppResult;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Token(String),
    Progress(ProgressInfo),
    Status(String),
    Error(String),
    Finished,
}

#[async_trait]
pub trait OutputStreamer: Send + Sync {
    async fn handle_event(&mut self, event: StreamEvent) -> AppResult<()>;
    async fn finish(&mut self) -> AppResult<()>;
}
