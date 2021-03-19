//! The daemon's RPC server.

use super::IndicateActivity;
use super::LoopHandlerEvent;
use crate::ops::error::ExitError;
use crate::socket;
use crate::socket::path::SocketPath;

use crossbeam_channel as chan;

/// The daemon server.
pub struct Server {
    activity_tx: chan::Sender<IndicateActivity>,
    build_tx: chan::Sender<LoopHandlerEvent>,
    socket_path: SocketPath,
}

impl Server {
    /// Create a new Server.
    pub fn new(
        socket_path: SocketPath,
        activity_tx: chan::Sender<IndicateActivity>,
        build_tx: chan::Sender<LoopHandlerEvent>,
    ) -> Server {
        Server {
            socket_path,
            activity_tx,
            build_tx,
        }
    }

    // TODO: don’t return ExitError
    /// Serve the lorri daemon server
    pub fn serve(self) -> Result<(), ExitError> {
        socket::Server::new(self.activity_tx.clone(), self.build_tx.clone())
            .listen(&self.socket_path)
            .map(|n| n.never())
    }
}
