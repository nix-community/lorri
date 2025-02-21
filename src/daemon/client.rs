//! Create clients for the daemon.
use crate::constants::Paths;
use crate::socket::communicate;
use crate::socket::communicate::client::InitError;
use crate::socket::communicate::{client::Client, Handler};
use crate::socket::path::SocketPath;
use slog::debug;

pub use crate::socket::read_writer::Timeout;

/// Create a connected client or exit.
///
/// Don’t forget to call `shutdown()` on the client after using it.
pub async fn create<H>(
    paths: &Paths,
    timeout: Timeout,
    initial_connect: Option<Timeout>,
    logger: &slog::Logger,
) -> Result<Client<<H as Handler>::Response, H>, InitError>
where
    H: Handler,
{
    let address = paths.daemon_socket_file().clone();
    debug!(logger, "connecting to socket"; "socket" => address.as_path().display());

    let client = communicate::client::new::<H>(timeout)
        .connect(
            &SocketPath::from(address),
            // The first connection does not use the read/write timeout
            // because we never want to block indefinitely on the daemon on first connect
            initial_connect.unwrap_or(Timeout::from_millis(1000)),
        )
        .await?;

    Ok(client)
}
