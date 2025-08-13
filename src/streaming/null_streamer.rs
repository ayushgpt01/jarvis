use super::{OutputStreamer, StreamEvent};
use crate::AppResult;

pub struct NullStreamer;

#[allow(dead_code)]
impl NullStreamer {
    pub fn new() -> Self {
        NullStreamer {}
    }
}

#[async_trait::async_trait]
impl OutputStreamer for NullStreamer {
    async fn finish(&mut self) -> AppResult<()> {
        Ok(())
    }

    async fn handle_event(&mut self, event: StreamEvent) -> AppResult<()> {
        log::debug!("{:#?}", event);
        Ok(())
    }
}
