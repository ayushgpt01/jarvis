use crate::streaming::cli_streamer::CliStreamer;

pub mod cli_streamer;
pub mod streamer;

pub fn create_cli_streamer(show_progress: bool) -> CliStreamer {
    CliStreamer::new(show_progress)
}
