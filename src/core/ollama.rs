use crate::{
    AppResult,
    streaming::{OutputStreamer, ProgressInfo, StreamEvent},
};
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

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
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: Option<ModelFileOptions>,
    raw: bool,
    template: Option<String>,
    system: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamResult {
    // model: String,
    // created_at: String,
    response: String,
    done: bool,
}

// #[derive(Debug, Deserialize)]
// struct OllamaResult {
//     model: String,
//     created_at: String,
//     response: String,
//     done: bool,
//     context: Vec<String>, // Need to check this type
//     total_duration: i64,
//     load_duration: i64,
//     prompt_eval_count: i32,
//     prompt_eval_duration: i64,
//     eval_count: i32,
//     eval_duration: i64,
// }

impl OllamaGenerate {
    pub async fn send_prompt_streaming(
        &self,
        prompt: &str,
        streamer: &mut impl OutputStreamer,
    ) -> AppResult<()> {
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
