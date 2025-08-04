use futures_util::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use crate::{
    error::AppError,
    streaming::streamer::{OutputStreamer, ProgressInfo, StreamEvent},
};

const GENERATE_API: &str = "/api/generate";
// const COMPLETION_API: &str = "http://localhost:11434/v1/chat/completions";

#[derive(Debug, Serialize, Clone)]
pub struct ModelFileOptions {
    pub num_ctx: i32,
    pub repeat_last_n: i32,
    pub repeat_penalty: f32,
    pub temperature: f32,
    pub seed: i32,
    pub stop: String,
    pub num_predict: i32,
    pub top_k: i32,
    pub top_p: f32,
    pub min_p: f32,
}

#[derive(Debug, Serialize)]
pub struct OllamaGenerate {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub options: Option<ModelFileOptions>,
    pub raw: Option<bool>,
    pub stream: Option<bool>,
    pub template: Option<String>,
    pub system: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: Option<ModelFileOptions>,
    pub raw: bool,
    pub template: Option<String>,
    pub system: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaStreamResult {
    // pub model: String,
    // pub created_at: String,
    pub response: String,
    pub done: bool,
}

// #[derive(Debug, Deserialize)]
// pub struct OllamaResult {
//     pub model: String,
//     pub created_at: String,
//     pub response: String,
//     pub done: bool,
//     pub context: Vec<String>, // Need to check this type
//     pub total_duration: i64,
//     pub load_duration: i64,
//     pub prompt_eval_count: i32,
//     pub prompt_eval_duration: i64,
//     pub eval_count: i32,
//     pub eval_duration: i64,
// }

impl OllamaGenerate {
    pub async fn send_prompt_streaming(
        &self,
        prompt: &str,
        streamer: &mut impl OutputStreamer,
    ) -> Result<(), AppError> {
        let client = Client::new();

        streamer
            .handle_event(StreamEvent::Progress(ProgressInfo {
                current: 20,
                total: None,
                message: "Getting response...".to_string(),
            }))
            .await?;

        let response = client
            .post(format!("{}:{}{}", self.host, self.port, GENERATE_API))
            .json(&OllamaRequest {
                model: self.model.clone(),
                prompt: prompt.to_string(),
                stream: self.stream.unwrap_or(true),
                raw: self.raw.unwrap_or(false),
                options: self.options.clone(),
                template: self.template.clone(),
                system: self.system.clone(),
            })
            .send()
            .await?
            .error_for_status()?;

        let byte_stream = response.bytes_stream();
        let stream_reader = StreamReader::new(
            byte_stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let mut lines = LinesStream::new(BufReader::new(stream_reader).lines());

        while let Some(line) = lines.next().await {
            match line {
                Ok(l) if !l.trim().is_empty() => {
                    match serde_json::from_str::<OllamaStreamResult>(&l) {
                        Ok(result) => {
                            if !result.response.is_empty() {
                                streamer
                                    .handle_event(StreamEvent::Token(result.response))
                                    .await?;
                            }

                            if result.done {
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse: {}, line: {}", e, l);
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

        Ok(())
    }
}
