use crate::{AppError, AppResult};
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, FlexiLoggerError, Logger, LoggerHandle, Naming,
    WriteMode,
};

pub fn logger_init() -> AppResult<LoggerHandle> {
    let logger = Logger::try_with_env_or_str("info, my::critical::module=trace")?
        .log_to_file(FileSpec::default().directory("logs"))
        .rotate(Criterion::Age(Age::Day), Naming::Timestamps, Cleanup::Never)
        .append()
        .write_mode(WriteMode::BufferAndFlush)
        .duplicate_to_stderr(Duplicate::None)
        .start()?;

    Ok(logger)
}

impl From<FlexiLoggerError> for AppError {
    fn from(e: FlexiLoggerError) -> Self {
        AppError::Other(e.to_string())
    }
}
