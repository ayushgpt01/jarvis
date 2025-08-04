use std::io::{self, Write};

use log::{debug, error, info};

use super::streamer::{OutputStreamer, StreamEvent};
use crate::error::AppError;
// use tokio::time::{Instant, Duration};

pub struct CliStreamer {
    pub show_progress: bool,
    //  last_progress_update: Instant,
}

impl CliStreamer {
    pub fn new(new_show_progress: bool) -> Self {
        CliStreamer {
            show_progress: new_show_progress,
            //  last_progress_update: Instant::now() - Duration::from_millis(100),
        }
    }

    pub fn write(&mut self, data: &str) -> Result<(), AppError> {
        print!("{}", data);
        io::stdout().flush()?;
        Ok(())
    }

    pub fn write_message(&mut self, message: &str) -> Result<(), AppError> {
        println!("{}", message);
        io::stdout().flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), AppError> {
        println!();
        io::stdout().flush()?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> Result<(), AppError> {
        print!("\r\x1b[K");
        io::stdout().flush()?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl OutputStreamer for CliStreamer {
    async fn finish(&mut self) -> Result<(), AppError> {
        self.handle_event(StreamEvent::Finished).await
    }

    async fn handle_event(&mut self, event: StreamEvent) -> Result<(), AppError> {
        debug!("{:?}", event);
        match event {
            StreamEvent::Token(token) => {
                if self.show_progress {
                    self.clear_line()?;
                }
                self.write(&token)?;
                debug!("Streamed token: {}", token);
            }
            StreamEvent::Progress(progress) => {
                // if self.show_progress && self.last_progress_update.elapsed() >= Duration::from_millis(50)
                if self.show_progress {
                    self.clear_line()?;
                    let progress_text = if let Some(total) = progress.total {
                        format!("\r⏳ {} ({}/{})", progress.message, progress.current, total)
                    } else {
                        format!("\r⏳ {} ({})", progress.message, progress.current)
                    };
                    self.write(&progress_text)?;
                    //  self.last_progress_update = Instant::now();
                }
            }
            StreamEvent::Status(status) => {
                if self.show_progress {
                    self.clear_line()?;
                    self.write(&format!("\r💬 {}\n", status))?;
                }
                info!("Status: {}", status);
            }
            StreamEvent::Error(error) => {
                if self.show_progress {
                    self.clear_line()?;
                }
                self.write(&format!("\rError: {}\n", error))?;
                error!("Stream error: {}", error);
            }
            StreamEvent::Finished => {
                if self.show_progress {
                    self.clear_line()?;
                }
                // Just ensure we're on a new line
                self.flush()?;
                info!("Streaming finished");
            }
        }

        Ok(())
    }
}
