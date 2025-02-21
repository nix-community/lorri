//! Talking to the `lorri` daemon / unix sockets.

use std::fmt;
use std::future::IntoFuture;
use std::marker::PhantomData;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Wrapper around a socket that can send and receive structured messages.
///
/// `timeout` arguments set the socket timeout before reading/writing.
pub struct ReadWriter<R, W> {
    // where R: serde::Deserialize {
    socket: Option<UnixStream>,
    phantom_r: PhantomData<R>,
    phantom_w: PhantomData<W>,
}

/// A (possible) timeout.
#[derive(Clone, Copy, Debug)]
pub enum Timeout {
    /// Do not time out.
    Infinite,
    /// Time out after `Duration`.
    D(Duration),
}

impl Timeout {
    /// create timeout from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self::D(Duration::from_millis(millis))
    }

    /// Run the future with a timeout, and if the timeout finishes first return the timeout duration.
    ///
    /// Returns the remaining timeout for use in a looping operation.
    pub async fn with<F: IntoFuture>(self, f: F) -> Result<(F::Output, Timeout), Duration> {
        match self {
            Timeout::Infinite => Ok((f.await, Timeout::Infinite)),
            Timeout::D(d) => {
                let now = tokio::time::Instant::now();
                match tokio::time::timeout(d, f).await {
                    Err(_e) => Err(d),
                    Ok(res) => {
                        let remaining = d.saturating_sub(now.elapsed());
                        if remaining == Duration::ZERO {
                            Err(d)
                        } else {
                            Ok((res, Timeout::D(remaining)))
                        }
                    }
                }
            }
        }
    }
}
impl From<Duration> for Timeout {
    fn from(value: Duration) -> Self {
        Timeout::D(value)
    }
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Timeout::Infinite => write!(f, "infinite timeout"),
            Timeout::D(d) => write!(f, "{:?}", d),
        }
    }
}

/// Reading from a `ReadWriter<R, W>` failed.
#[derive(Error, Debug)]
pub enum ReadError {
    /// Deserializing `R` failed.
    #[error("Unable to deserialize message: {0}")]
    Deserialize(#[source] serde_json::Error),
    /// IO error reading line
    #[error("Cannot read from socket: {0}")]
    IO(std::io::Error),
    /// No value available within given timeout.
    #[error("The read timed out ({0})")]
    Timeout(Timeout),
    /// end of file read
    #[error("EOF on Reader")]
    EndOfFile,
}

// TODO: combine with ReadError?
/// Writing to a `ReadWriter<R, W>` failed.
#[derive(Error, Debug)]
pub enum WriteError {
    /// Serializing `W` failed.
    #[error("Unable to serialize message: {0}")]
    Serialize(#[source] serde_json::Error),
    /// IO error writing line
    #[error("Cannot write to socket: {0}")]
    IO(std::io::Error),
    /// No value available within given timeout.
    #[error("The read timed out ({0})")]
    Timeout(Timeout),
}

/// Reading from or writing to a `ReadWriter<R, W>` failed.
#[derive(Error, Debug)]
pub enum ReadWriteError {
    /// Reading failed.
    #[error("read error: {0}")]
    R(#[source] ReadError),
    /// Writing failed.
    #[error("write error: {0}")]
    W(#[source] WriteError),
}

impl From<ReadError> for ReadWriteError {
    fn from(r: ReadError) -> Self {
        ReadWriteError::R(r)
    }
}
impl From<WriteError> for ReadWriteError {
    fn from(r: WriteError) -> Self {
        ReadWriteError::W(r)
    }
}

impl<'a, R, W> ReadWriter<R, W> {
    // TODO: &mut UnixStream
    /// Create from a unix socket.
    pub fn new(socket: UnixStream) -> ReadWriter<R, W> {
        ReadWriter {
            socket: Some(socket),
            phantom_r: PhantomData,
            phantom_w: PhantomData,
        }
    }

    /// return original stream
    pub fn into_inner(self) -> UnixStream {
        self.socket.unwrap()
    }

    /// Send a message to the other side and wait for a reply.
    ///
    /// The timeout counts for the whole roundtrip.
    pub async fn communicate(
        &mut self,
        timeout: Timeout,
        mes: &W,
    ) -> Result<(Timeout, R), ReadWriteError>
    where
        R: serde::de::DeserializeOwned,
        W: serde::Serialize,
    {
        let orig_timeout = timeout;
        let timeout = self.write(timeout, mes).await?;
        let e = match self.read(timeout).await {
            // in this case we want to return the original timeout, not the remaining one
            Err(ReadError::Timeout(_t)) => Err(ReadError::Timeout(orig_timeout))?,
            o => o?,
        };
        Ok(e)
    }

    /// Listen for a message from the other side and immediately
    /// send a reply based on the message.
    ///
    /// The timeout counts for the whole roundtrip.
    pub async fn react<F>(&mut self, timeout: Timeout, reaction: F) -> Result<R, ReadWriteError>
    where
        R: serde::de::DeserializeOwned,
        W: serde::Serialize,
        F: FnOnce(&R) -> W,
    {
        let orig_timeout = timeout;
        let (timeout, read) = self.read(timeout).await?;
        match self.write(timeout, &reaction(&read)).await {
            // in this case we want to return the original timeout, not the remaining one
            Err(WriteError::Timeout(_t)) => Err(WriteError::Timeout(orig_timeout))?,
            o => o?,
        };
        Ok(read)
    }

    /// Wait for a message to arrive.
    pub async fn read(&mut self, timeout: Timeout) -> Result<(Timeout, R), ReadError>
    where
        R: serde::de::DeserializeOwned,
    {
        let sock = self.socket.take().unwrap();
        let mut take = BufReader::new(sock.take(1_000_000)).lines();
        let x = timeout.with(take.next_line()).await;
        self.socket = Some(take.into_inner().into_inner().into_inner());
        match x {
            Err(d) => Err(ReadError::Timeout(Timeout::D(d))),
            Ok((Ok(Some(line)), timeout)) => match serde_json::de::from_str(&line) {
                Ok(a) => Ok((timeout, a)),
                Err(e) => Err(ReadError::Deserialize(e)),
            },
            Ok((Ok(None), _)) => Err(ReadError::EndOfFile),
            Ok((Err(e), _)) => Err(ReadError::IO(e)),
        }
    }

    /// Send a message to the other side.
    pub async fn write(&mut self, timeout: Timeout, mes: &W) -> Result<Timeout, WriteError>
    where
        W: serde::Serialize,
    {
        let mut line = serde_json::to_vec(mes).map_err(WriteError::Serialize)?;
        let mut sock = self.socket.take().unwrap();
        line.extend_from_slice("\n".as_bytes());
        let res = match timeout.with(sock.write_all(&line)).await {
            Err(d) => Err(WriteError::Timeout(Timeout::D(d))),
            Ok((Err(e), _)) => Err(WriteError::IO(e)),
            Ok((Ok(()), remaining_time)) => Ok(remaining_time),
        };

        sock.flush().await.map_err(WriteError::IO)?;
        self.socket = Some(sock);
        res
    }
}
