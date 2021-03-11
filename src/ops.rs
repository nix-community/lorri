//! Ops are command-line callables.

pub mod error;

pub mod daemon;
pub mod direnv;
pub mod info;
pub mod init;
pub mod ping;
pub mod shell;
pub mod start_user_shell;
pub mod stream_events;
pub mod upgrade;
pub mod watch;

/// Set up necessary directories or fail.
pub fn get_paths() -> Result<crate::constants::Paths, error::ExitError> {
    crate::constants::Paths::initialize().map_err(|e| {
        error::ExitError::user_error(format!("Cannot initialize the lorri paths: {:#?}", e))
    })
}
