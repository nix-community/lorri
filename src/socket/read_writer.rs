//! Talking to the `lorri` daemon / unix sockets.

use std::convert::TryFrom;
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Wrapper around a socket that can send and receive structured messages.
///
/// `timeout` arguments set the socket timeout before reading/writing.
pub struct ReadWriter<'a, R, W> {
    // where R: serde::Deserialize {
    socket: &'a UnixStream,
    phantom_r: PhantomData<R>,
    phantom_w: PhantomData<W>,
}

/// Milliseconds accepted by a `Timeout`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Millis(u16);

impl From<Millis> for Duration {
    fn from(m: Millis) -> Duration {
        let Millis(u) = m;
        Duration::from_millis(u64::from(u))
    }
}

impl TryFrom<Duration> for Millis {
    type Error = ();
    fn try_from(d: Duration) -> Result<Millis, Self::Error> {
        Ok(Millis(u16::try_from(d.as_millis()).map_err(|_| ())?))
    }
}

/// A (possible) timeout.
#[derive(Clone, Copy, Debug)]
pub enum Timeout {
    /// Do not time out.
    Infinite,
    /// Time out after `Duration`.
    D(Millis),
}

impl Timeout {
    /// Construct from a millisecond u16.
    pub const fn from_millis(m: u16) -> Timeout {
        Timeout::D(Millis(m))
    }
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Timeout::Infinite => write!(f, "infinite timeout"),
            Timeout::D(Millis(m)) => write!(f, "{} ms", m),
        }
    }
}

/// Reading from a `ReadWriter<R, W>` failed.
#[derive(Error, Debug)]
pub enum ReadError {
    /// Deserializing `R` failed.
    #[error("Unable to deserialize message: {0}")]
    Deserialize(#[source] bincode::Error),
    /// No value available within given timeout.
    #[error("The read timed out ({0})")]
    Timeout(Timeout),
}

// TODO: combine with ReadError?
/// Writing to a `ReadWriter<R, W>` failed.
#[derive(Error, Debug)]
pub enum WriteError {
    /// Serializing `W` failed.
    #[error("Unable to serialize message: {0}")]
    Serialize(#[source] bincode::Error),
    /// No value available within given timeout.
    #[error("The read timed out ({0})")]
    Timeout(Timeout),
}

impl From<bincode::Error> for WriteError {
    fn from(e: bincode::Error) -> WriteError {
        WriteError::Serialize(e)
    }
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

// put into the Io part of a `bincode::ErrorKind`
fn into_bincode_io_error<T>(res: std::io::Result<T>) -> bincode::Result<T> {
    res.map_err(|e| Box::new(bincode::ErrorKind::Io(e)))
}

/// Run action with a timeout, return the remaining timeout
/// or `Err` if there’s no time remaining.
///
/// The interface is not very “rusty”, but it’s only used internally.
fn with_timeout<F, A>(timeout: Timeout, action: F) -> Result<(Timeout, A), ()>
where
    F: FnOnce(Timeout) -> A,
{
    // start a timer
    let i = Instant::now();
    // run the action with the full timeout
    let res = action(timeout);
    match timeout {
        Timeout::Infinite => Ok((Timeout::Infinite, res)),
        Timeout::D(Millis(u)) => match Millis::try_from(i.elapsed()) {
            // more time elapsed than we can hold in a timeout
            // so it must have blown the allowed timeout
            Err(_) => Err(()),
            Ok(Millis(elapsed)) => match u.checked_sub(elapsed) {
                // no time remaining
                None => Err(()),
                // return a new timeout with remaining time
                Some(u2) => Ok((Timeout::D(Millis(u2)), res)),
            },
        },
    }
}

impl<'a, R, W> ReadWriter<'a, R, W> {
    // TODO: &mut UnixStream
    /// Create from a unix socket.
    pub fn new(socket: &'a UnixStream) -> ReadWriter<'a, R, W> {
        ReadWriter {
            socket,
            phantom_r: PhantomData,
            phantom_w: PhantomData,
        }
    }

    /// Send a message to the other side and wait for a reply.
    ///
    /// The timeout counts for the whole roundtrip.
    pub fn communicate(&mut self, timeout: Timeout, mes: &W) -> Result<R, ReadWriteError>
    where
        R: serde::de::DeserializeOwned,
        W: serde::Serialize,
    {
        let orig_timeout = timeout;
        let (timeout, write_res) = with_timeout(timeout, |t| self.write(t, mes))
            .map_err(|()| WriteError::Timeout(orig_timeout))?;
        write_res?;
        let e = match self.read(timeout) {
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
    pub fn react<F>(&mut self, timeout: Timeout, reaction: F) -> Result<R, ReadWriteError>
    where
        R: serde::de::DeserializeOwned,
        W: serde::Serialize,
        F: FnOnce(&R) -> W,
    {
        let orig_timeout = timeout;
        let (timeout, read_res) = with_timeout(timeout, |t| self.read(t))
            .map_err(|()| ReadError::Timeout(orig_timeout))?;
        let read = read_res?;
        match self.write(timeout, &reaction(&read)) {
            // in this case we want to return the original timeout, not the remaining one
            Err(WriteError::Timeout(_t)) => Err(WriteError::Timeout(orig_timeout))?,
            o => o?,
        };
        Ok(read)
    }

    /// Check if the underlying socket timed out when serializing/deserializing.
    fn is_timed_out(e: &bincode::ErrorKind) -> bool {
        match e {
            bincode::ErrorKind::Io(io) => matches!(io.kind(), std::io::ErrorKind::TimedOut),
            _ => false,
        }
    }

    /// Wait for a message to arrive.
    pub fn read(&self, timeout: Timeout) -> Result<R, ReadError>
    where
        R: serde::de::DeserializeOwned,
    {
        let timeout_socket = timeout::TimeoutReadWriter::new(self.socket, timeout);

        // XXX: “If this returns an Error, `reader` may be in an invalid state”.
        // what the heck does that mean.
        bincode::deserialize_from(timeout_socket).map_err(|e| {
            if Self::is_timed_out(&e) {
                ReadError::Timeout(timeout)
            } else {
                ReadError::Deserialize(e)
            }
        })
    }

    /// Send a message to the other side.
    pub fn write(&mut self, timeout: Timeout, mes: &W) -> Result<(), WriteError>
    where
        W: serde::Serialize,
    {
        let timeout_socket = timeout::TimeoutReadWriter::new(self.socket, timeout);

        bincode::serialize_into(timeout_socket, mes).map_err(|e| {
            if Self::is_timed_out(&e) {
                WriteError::Timeout(timeout)
            } else {
                WriteError::Serialize(e)
            }
        })?;

        into_bincode_io_error(self.socket.flush())?;

        Ok(())
    }
}

/// Wrap a socket with a timeout. Inspired by <https://docs.rs/crate/timeout-readwrite/0.2.0/>.
mod timeout {
    extern crate nix;

    use self::nix::libc;
    use self::nix::poll;
    use super::{Millis, Timeout};
    use std::os::unix::io::AsRawFd;
    use std::os::unix::net::UnixStream;

    /// Wait until `to_fd` receives the poll event from `events`, up to `timeout` length
    /// of time.
    /// Copied from <https://docs.rs/crate/timeout-readwrite/0.2.0/source/src/utils.rs>
    /// written by Jonathan Creekmore and published under Apache-2.0.
    fn wait_until_ready<R: AsRawFd>(
        timeout: libc::c_int,
        to_fd: &R,
        events: poll::PollFlags,
    ) -> std::io::Result<()> {
        let mut pfd = poll::PollFd::new(to_fd.as_raw_fd(), events);
        let s = unsafe { std::slice::from_raw_parts_mut(&mut pfd, 1) };

        let retval = poll::poll(s, timeout)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if retval == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timed out waiting for fd to be ready",
            ));
        }
        Ok(())
    }

    pub struct TimeoutReadWriter<'a> {
        socket: &'a UnixStream,
        timeout: libc::c_int,
    }

    /// Convert timeout to the form that `poll(2)` expects.
    fn to_poll_2_timeout(t: Timeout) -> libc::c_int {
        match t {
            // negative number is infinite timeout
            Timeout::Infinite => -1,
            // otherwise a duration in milliseconds
            Timeout::D(Millis(u)) => libc::c_int::from(u),
        }
    }

    impl<'a> TimeoutReadWriter<'a> {
        pub fn new(socket: &'a UnixStream, timeout: Timeout) -> TimeoutReadWriter<'a> {
            TimeoutReadWriter {
                socket,
                timeout: to_poll_2_timeout(timeout),
            }
        }
    }

    impl<'a> std::io::Read for TimeoutReadWriter<'a> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            wait_until_ready(self.timeout, self.socket, poll::PollFlags::POLLIN)?;
            self.socket.read(buf)
        }
    }

    impl<'a> std::io::Write for TimeoutReadWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            wait_until_ready(self.timeout, self.socket, poll::PollFlags::POLLOUT)?;
            self.socket.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            wait_until_ready(self.timeout, self.socket, poll::PollFlags::POLLOUT)?;
            self.socket.flush()
        }
    }
}
