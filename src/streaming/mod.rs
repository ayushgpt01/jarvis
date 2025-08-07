mod cli_streamer;
mod streamer;

pub use cli_streamer::CliStreamer;
pub use streamer::*;

pub fn create_cli_streamer(show_progress: bool) -> CliStreamer {
    CliStreamer::new(show_progress)
}
