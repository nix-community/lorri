//! Create clients for the daemon.
use crate::constants::Paths;
use crate::ops::error::ExitError;
use crate::socket::communicate;
use crate::socket::communicate::client::InitError;
use crate::socket::communicate::{client::Client, Handler};
use crate::socket::path::SocketPath;
use slog::debug;

pub use crate::socket::communicate::{Ping, Rebuild, StreamEvents};
pub use crate::socket::read_writer::Timeout;

/// Create a connected client or exit.
pub fn create<H>(
    paths: &Paths,
    timeout: Timeout,
    logger: &slog::Logger,
) -> Result<Client<<H as Handler>::Resp, H>, InitError>
where
    H: Handler,
{
    let address = paths.daemon_socket_file().clone();
    debug!(logger, "connecting to socket"; "socket" => address.as_path().display());

    let client = communicate::client::new::<H>(timeout).connect(&SocketPath::from(address))?;

    Ok(client)
}
