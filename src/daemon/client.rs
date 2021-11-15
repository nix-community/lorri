//! Create clients for the daemon.
use crate::ops::error::ExitError;
use crate::socket::communicate;
use crate::socket::communicate::{client::Client, Handler};
use crate::socket::path::SocketPath;
use slog::debug;

pub use crate::socket::communicate::{Ping, Rebuild, StreamEvents};
pub use crate::socket::read_writer::Timeout;

/// Create a connected client or exit.
pub fn create<H>(
    timeout: Timeout,
    logger: &slog::Logger,
) -> Result<Client<<H as Handler>::Resp, H>, ExitError>
where
    H: Handler,
{
    let address = crate::ops::get_paths()?.daemon_socket_file().clone();
    debug!(logger, "connecting to socket"; "socket" => address.as_path().display());

    let client = communicate::client::new::<H>(timeout).connect(&SocketPath::from(address))?;

    Ok(client)
}
