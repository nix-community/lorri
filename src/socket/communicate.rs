//! Communication with the lorri daemon.
//!
//! We provide a fixed number of `CommunicationType`s that are sent
//! between the lorri daemon and lorri clients wishing to connect to it.
//!
//! `listener` implements the daemon side, which provides an `accept`
//! method which is similar to how `accept()` works on a plain Unix socket.
//!
//! `client` implements a set of clients specialized to the communications
//! we support.

use thiserror::Error;

use crate::build_loop;
use crate::ops::error::{ExitAs, ExitErrorType};
use crate::project::ProjectFile;
use crate::socket::path::{BindError, SocketPath};
use crate::socket::read_writer::{ReadWriteError, ReadWriter, Timeout};

/// We declare 1s as the time readers should wait
/// for the other side to send something.
pub const DEFAULT_READ_TIMEOUT: Timeout = Timeout::from_millis(1000);

/// Binds a client request type to a server response.
///
/// For example, the handler for `Ping` has the response type `NoMessage`,
/// because the server does not reply to pings.
pub trait Handler {
    /// The request type used by this handler.
    type Request;
    /// The response returned to the client for the given request type.
    type Response;
    /// The `CommunicationType` that corresponds to these types.
    fn communication_type() -> CommunicationType;
}

/// Enum of all communication modes the lorri daemon supports.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CommunicationType {
    /// Return some information about the running daemon
    DaemonInfo,
    /// Ping the daemon from a project to tell it to watch & evaluate
    Ping,
    /// Stream events that happen in the daemon to the client, as they happen.
    StreamEvents,
    /// Return a snapshot of the stream events that already happened.
    StreamSnapshot,
}

/// No message can be sent through this socket end (empty type).
#[derive(Serialize, Deserialize)]
pub enum NoMessage {}

/// Message sent by the client to ask the server about some status info
#[derive(Serialize, Deserialize, Debug)]
pub struct DaemonInfo {}

impl Handler for DaemonInfo {
    type Request = DaemonInfo;
    type Response = ();

    fn communication_type() -> CommunicationType {
        CommunicationType::DaemonInfo
    }
}

/// Message sent by the client to ask the server to start
/// watching `nix_file`. See `CommunicationType::Ping`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Ping {
    /// The nix file to watch and build on changes.
    pub project_file: ProjectFile,
    /// When/whether to start the build.
    pub rebuild: Rebuild,
}

/// In which cases a ping will trigger a rebuild
#[derive(Serialize, Deserialize, Debug)]
pub enum Rebuild {
    /// Only if the nix_file is not yet watched
    OnlyIfNotYetWatching,
    /// Always
    Always,
}

impl Handler for Ping {
    type Request = Ping;
    type Response = NoMessage;

    fn communication_type() -> CommunicationType {
        CommunicationType::Ping
    }
}

/// Stream events to the client, as they happen.
#[derive(Serialize, Deserialize, Debug)]
pub struct StreamEvents {}

impl Handler for StreamEvents {
    type Request = StreamEvents;
    type Response = build_loop::Event;

    fn communication_type() -> CommunicationType {
        CommunicationType::StreamEvents
    }
}

/// A snapshot of build events that already happened.
#[derive(Serialize, Deserialize, Debug)]
pub struct StreamSnapshot {}

impl Handler for StreamSnapshot {
    type Request = NoMessage;
    type Response = build_loop::EventSnapshot;

    fn communication_type() -> CommunicationType {
        CommunicationType::StreamSnapshot
    }
}

/// `Listener` and possible errors.
pub mod listener {
    use super::*;
    use crate::socket::read_writer::ReadWriterState;
    use nix::fcntl::Flock;
    use std::fs::File;
    use tokio::net::UnixListener;

    /// If a connection on the socket is attempted and the first
    /// message is of a `ConnectionType`, the `Listener` returns
    /// this message as an ack.
    /// In all other cases the `Listener` returns no answer (the
    /// bad client should time out after some time).
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ConnectionAccepted {
        /// A field that is only put into the serialized message to make debugging
        /// the wire protocol easier.
        /// We don’t branch on it when deserializing, thus the use of `serde(default)`.
        #[serde(default)]
        accepted: String,
    }

    /// Server-side part of a socket transmission,
    /// listening for incoming messages.
    pub struct Listener {
        /// Bound Unix socket.
        listener: UnixListener,
        /// Lock that keeps our socket exclusive.
        // `bind_lock` is never actually used anywhere,
        // it is released when `Listener`’s lifetime ends.
        // We can ignore the “dead code” warning.
        #[allow(dead_code)]
        bind_lock: Flock<File>,
        /// How long to wait for the client to send its
        /// first message after opening the connection.
        accept_timeout: Timeout,
    }

    /// Standing connection to the client.
    /// The `communication_type` determines which handler the user has to call in `Handlers`,
    /// but the missing dependent types makes it hard to express;
    /// Handlers should probably be a trait instead.
    pub struct Connection {
        /// The kind of communication the client requested us to talk to it.
        pub communication_type: CommunicationType,
        /// socket that is open for the given CommunicationType
        pub read_writer_state: ReadWriterState,
    }

    /// Errors in `accept()`ing a new connection.
    #[derive(Debug)]
    pub enum AcceptError {
        /// something went wrong in the `accept()` syscall.
        Accept(std::io::Error),
        /// The client’s message could not be decoded.
        Message(ReadWriteError),
    }

    impl Listener {
        /// Create a new `daemon` by binding to `socket_path`.
        pub async fn new(socket_path: &SocketPath) -> Result<Listener, BindError> {
            let (l, lock) = socket_path.bind().await?;
            Ok(Listener {
                listener: l,
                bind_lock: lock,
                accept_timeout: DEFAULT_READ_TIMEOUT,
            })
        }

        /// Accept a new connection on the socket, and read the communication type,
        /// then return the open socket.
        ///
        /// This method blocks until a client tries to connect.
        pub async fn accept(&self) -> Result<Connection, AcceptError> {
            // - socket accept
            let (unix_stream, _) = self.listener.accept().await.map_err(AcceptError::Accept)?;
            // - read first message as a `CommunicationType`
            let mut rw = ReadWriter::<CommunicationType, ConnectionAccepted>::new(unix_stream);
            let communication_type: CommunicationType = rw
                .react(self.accept_timeout, |c| ConnectionAccepted {
                    accepted: format!("{:?}", c),
                })
                .await
                .map_err(AcceptError::Message)?;
            // spawn a thread with the accept handler
            Ok(Connection {
                read_writer_state: rw.into_inner_state(),
                communication_type,
            })
        }
    }

    /// All handlers we have available to read messages and reply.
    pub mod handlers {
        use crate::socket::communicate::*;
        use crate::socket::read_writer::{ReadWriter, ReadWriterState};

        /// Create a `ReadWriter` from our state, make sure to check the `CommunicationType`
        /// first so it corresponds to the message types.
        pub fn from_state_unsafe<R: Handler>(
            read_writer_state: ReadWriterState,
        ) -> ReadWriter<<R as Handler>::Request, <R as Handler>::Response> {
            ReadWriter::from_state_unsafe(read_writer_state)
        }
    }
}

/// Clients that can talk to a `Listener`.
///
/// `R` is the type of messages this client reads.
/// `W` is the type of messages this client writes.
///
/// The construction of `Client` is only exported for
/// the pre-defined interactions with the `Listener` we support.
pub mod client {
    use super::*;
    use crate::socket::read_writer::ReadWriterState;
    use std::marker::PhantomData;

    /// A `Client` that can talk to a `Listener`.
    pub struct Client<R, W> {
        /// Type of interaction with the `Listener`.
        comm_type: CommunicationType,
        /// Connected socket.
        socket: Option<ReadWriterState>,
        /// Timeout for reads/writes.
        timeout: Timeout,
        read_type: PhantomData<R>,
        write_type: PhantomData<W>,
    }

    /// Error when talking to the `Listener`.
    #[derive(Error, Debug)]
    pub enum Error {
        /// Not connected to the `Listener` socket.
        #[error("Not connected to the daemon socket")]
        NotConnected,
        /// Read error or write error.
        #[error("Unable communicate with the daemon")]
        Message(#[source] ReadWriteError),
    }

    impl ExitAs for Error {
        fn exit_as(&self) -> ExitErrorType {
            use Error::*;
            match self {
                // This should really never happen.
                NotConnected => ExitErrorType::Panic,
                Message(_) => ExitErrorType::Temporary,
            }
        }
    }

    /// Error when initializing connection with the `Listener`.
    #[derive(Error, Debug)]
    pub enum InitError {
        /// `connect()` syscall failed.
        #[error("Unable to connect to socket at {0}, is the daemon running?")]
        SocketConnect(SocketPath, #[source] std::io::Error),
        /// Handshake failed (write `ConnectionType`, read `ConnectionAccepted`).
        #[error("Server Handshake failed: {0}")]
        ServerHandshake(ReadWriteError),
    }

    impl ExitAs for InitError {
        fn exit_as(&self) -> ExitErrorType {
            use InitError::*;
            match self {
                SocketConnect(_, _) => ExitErrorType::Temporary,
                ServerHandshake(_) => ExitErrorType::Temporary,
            }
        }
    }

    /// Create a Client for a given `Handler` type.
    /// Every enum in `CommunicationType` will have an instance for the type,
    /// named after the request (e.g. `CommunicationType::Ping` has a handler instance for `Ping`).
    pub fn new<T>(timeout: Timeout) -> Client<T::Response, T>
    where
        T: Handler,
    {
        Client::bake(timeout, T::communication_type())
    }

    // builder pattern for timeouts?
    // TODO: turn around the types in Client?
    impl<R, W> Client<R, W> {
        /// “Bake” a Client, aka set its communication type (and message type arguments).
        /// Not exported.
        fn bake(timeout: Timeout, comm_type: CommunicationType) -> Client<R, W> {
            Client {
                comm_type,
                socket: None,
                timeout,
                read_type: PhantomData,
                write_type: PhantomData,
            }
        }

        /// Connect to the `Listener` listening on `socket_path`.
        /// TODO: remove the split between new() and connect(), and then remove `Error::NotConnected`
        ///
        /// Don’t forget to call `shutdown()` after finishing with communication.
        pub async fn connect(
            self,
            socket_path: &SocketPath,
            initial_connect: Timeout,
        ) -> Result<Client<R, W>, InitError> {
            // TODO: check if the file exists and is a socket

            // - connect to `socket_path`
            let socket = socket_path
                .connect()
                .await
                .map_err(|e| InitError::SocketConnect(socket_path.clone(), e))?;

            // - send initial message with the CommunicationType
            // - wait for server to acknowledge connect
            let mut rw = ReadWriter::new(socket);
            let (_, _): (Timeout, listener::ConnectionAccepted) = rw
                .communicate(initial_connect, &self.comm_type)
                .await
                .map_err(InitError::ServerHandshake)?;

            Ok(Client {
                comm_type: self.comm_type,
                socket: Some(rw.into_inner_state()),
                timeout: self.timeout,
                read_type: PhantomData,
                write_type: PhantomData,
            })
        }

        /// Shut down this client, disconnect from socket.
        /// Any disconnect errors are ignored.
        pub async fn shutdown(self) {
            let Self { socket, .. } = self;
            socket.unwrap().shutdown().await;
        }

        /// Write a message to the connected `Listener`, then wait for the reply. The configured timeout counts for the whole roundtrip.
        pub async fn communicate(&mut self, mes: &W) -> Result<(Timeout, R), Error>
        where
            W: serde::Serialize,
            R: serde::de::DeserializeOwned,
        {
            let sock = self.socket.take().ok_or(Error::NotConnected)?;
            let mut rw: ReadWriter<R, W> = ReadWriter::from_state_unsafe(sock);
            let res = rw
                .communicate(self.timeout, mes)
                .await
                .map_err(|e| Error::Message(e));
            self.socket = Some(rw.into_inner_state());
            res
        }

        /// Read a message returned by the connected `Listener`.
        pub async fn read(&mut self) -> Result<R, Error>
        where
            R: serde::de::DeserializeOwned,
        {
            let sock = self.socket.take().ok_or(Error::NotConnected)?;
            let mut rw: ReadWriter<R, W> = ReadWriter::from_state_unsafe(sock);
            let res = rw
                .read(self.timeout)
                .await
                .map_err(|e| Error::Message(ReadWriteError::R(e)))?;
            self.socket = Some(rw.into_inner_state());
            Ok(res.1)
        }

        /// Write a message to the connected `Listener`.
        pub async fn write(&mut self, mes: &W) -> Result<(), Error>
        where
            W: serde::Serialize,
        {
            let sock = self.socket.take().ok_or(Error::NotConnected)?;
            let mut rw: ReadWriter<R, W> = ReadWriter::from_state_unsafe(sock);
            let _timeout = rw
                .write(self.timeout, mes)
                .await
                .map_err(|e| Error::Message(ReadWriteError::W(e)))?;
            self.socket = Some(rw.into_inner_state());
            Ok(())
        }
    }
}
