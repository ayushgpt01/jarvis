use crate::AppError;
use flexi_logger::{FileSpec, FlexiLoggerError, Logger, LoggerHandle, WriteMode};

pub fn logger_init() -> Result<LoggerHandle, AppError> {
    let logger = Logger::try_with_env_or_str("info, my::critical::module=trace")?
        .log_to_file(FileSpec::default())
        .write_mode(WriteMode::BufferAndFlush)
        .duplicate_to_stderr(flexi_logger::Duplicate::None)
        .start()?;

    Ok(logger)
}

impl From<FlexiLoggerError> for AppError {
    fn from(e: FlexiLoggerError) -> Self {
        AppError::Other(e.to_string())
    }
}
