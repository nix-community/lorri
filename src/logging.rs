//! Helps instantiate a root slog logger

use crate::cli::{Command, Verbosity};
use slog::Drain;

/// Instantiate a root logger appropriate for the subcommand
pub fn root(verbosity: Verbosity, command: &Command) -> slog::Logger {
    let level = match verbosity {
        // log only up to info
        Verbosity::DefaultInfo => slog::Level::Info,
        // log everything; be advised that trace-messages are removed at compile time by default,
        // see https://docs.rs/slog/2.7.0/slog/#notable-details
        Verbosity::Debug => slog::Level::Trace,
    };
    let log_to = match command {
        // direnv swallows stdout, so we must log to stderr
        Command::Direnv(_) => LogTo::Stderr,
        _ => LogTo::Stdout,
    };
    lorri_logger(level, log_to)
}

/// Logger that can be used in tests
#[cfg(test)]
pub fn test_logger() -> slog::Logger {
    lorri_logger(slog::Level::Trace, LogTo::Stderr)
}

/// output to log to
enum LogTo {
    Stdout,
    Stderr,
}

fn lorri_logger(level: slog::Level, log_to: LogTo) -> slog::Logger {
    let decorator = match log_to {
        LogTo::Stderr => slog_term::TermDecorator::new().stderr().build(),
        LogTo::Stdout => slog_term::TermDecorator::new().stdout().build(),
    };
    let drain = slog_term::FullFormat::new(decorator)
        .build()
        .filter_level(level)
        .fuse();
    // This makes all logging go through a mutex. Should logging ever become a bottleneck, consider
    // using slog_async instead.
    let drain = std::sync::Mutex::new(drain).fuse();
    slog::Logger::root(drain, slog::o!())
}
