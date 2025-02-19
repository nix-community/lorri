//! `bind()`ing & `connect()`ing to sockets.

use crate::ops::error::{ExitAs, ExitErrorType};
use crate::AbsPathBuf;
use nix::fcntl;
use nix::fcntl::Flock;
use std::fmt;
use std::fs::File;
use std::path::Path;
use thiserror::Error;
use tokio::net::{UnixListener, UnixStream};

/// Small wrapper that makes sure lorri sockets are handled correctly.
#[derive(Clone, Debug)]
pub struct SocketPath(AbsPathBuf);

/// Binding to the socket failed.
#[derive(Error, Debug)]
pub enum BindError {
    /// Another process is listening on the socket
    #[error("Another process is listening on the socket ({0}), do you have another lorri daemon running?")]
    OtherProcessListening(String),
    /// I/O error
    #[error("IO error binding to socket")]
    Io(#[source] std::io::Error),
    /// nix library I/O error (like Io)
    #[error("Unix error binding to socket")]
    Unix(#[source] nix::Error),
}

impl ExitAs for BindError {
    fn exit_as(&self) -> ExitErrorType {
        use BindError::*;
        use ExitErrorType::*;
        match self {
            OtherProcessListening(_) => UserError,
            Io(_) => Temporary,
            Unix(_) => Temporary,
        }
    }
}

impl From<std::io::Error> for BindError {
    fn from(e: std::io::Error) -> BindError {
        BindError::Io(e)
    }
}

impl SocketPath {
    /// Create from the path of the socket.
    /// Must be passed a valid socket file path (ending in a file name).
    pub fn from(socket_path: AbsPathBuf) -> SocketPath {
        SocketPath(socket_path)
    }

    /// Try to lock the lock file to find out whether another process is listening.
    pub fn lock(&self) -> Result<Flock<File>, BindError> {
        let h = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.lockfile())?;
        // we try to get an exclusive lock, nonblocking
        match fcntl::Flock::lock(h, nix::fcntl::FlockArg::LockExclusiveNonblock) {
            // if the lock would block, another process is listening
            Err((_, nix::Error::EWOULDBLOCK)) => Err(BindError::OtherProcessListening(
                self.lockfile().display().to_string(),
            )),
            Err((_, other)) => Err(BindError::Unix(other)),
            Ok(a) => Ok(a),
        }
    }

    /// `bind(2)` to this socket path.
    ///
    /// Uses a lock file to guarantee no other process is listening to the same socket.
    /// The lock file is the socket file with a `.lock` file ending appended.
    ///
    /// The lock file is released automatically when the returned `BindLock` is dropped.
    ///
    /// Uses the `flock(2)` trick decribed in
    /// <https://gavv.github.io/articles/unix-socket-reuse/>
    pub async fn bind(&self) -> Result<(UnixListener, Flock<File>), BindError> {
        // - try to lock lockfile (open and flock exclusive nonblocking)
        let lock = self.lock()?;
        // - remove socket file if it exists
        std::fs::remove_file(self.as_absolute_path()).or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        })?;
        // - bind to socket
        let l = UnixListener::bind(self.as_absolute_path())?;
        Ok((l, lock))
    }

    /// `connect(2)` to this socket path.
    pub async fn connect(&self) -> std::io::Result<UnixStream> {
        UnixStream::connect(self.as_absolute_path()).await
    }

    /// The absolute path of the socket.
    pub fn as_absolute_path(&self) -> &Path {
        self.0.as_ref()
    }

    /// `display` the path.
    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }

    fn lockfile(&self) -> AbsPathBuf {
        self.0.with_file_name({
            let mut s = self
                .0
                .as_path()
                .file_name()
                .unwrap_or_else(|| panic!("Socket file ({:?}) must end in a file name", self.0))
                .to_owned();
            s.push(".lock");
            s
        })
    }
}

impl fmt::Display for SocketPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lock_is_exclusive() {
        let tempdir = tempfile::tempdir().unwrap();
        let p = AbsPathBuf::new(tempdir.path().join("socket")).unwrap();
        let _lock = SocketPath(p.clone())
            .lock()
            .expect("first locking attempt should succeed");
        assert!(
            SocketPath(p).lock().is_err(),
            "second locking attempt should fail because we still hold the lock"
        );
    }
}
