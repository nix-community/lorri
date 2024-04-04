//! A thread pool for panicking immediately after any monitored threads
//! panic.
//!
//! The key implementation detail is each thread spawned gets a
//! DeathCertificate which sends a message on Drop. This allows us to
//! join a thread once we know it has completed execution, meaning
//! we don't block joining one thread while another thread has panicked
//! already.

use crossbeam_channel as chan;
use slog::debug;
use std::any::Any;
use std::collections::HashMap;
use std::thread;
use std::thread::ThreadId;

struct Thread {
    name: String,
    join_handle: thread::JoinHandle<()>,
}

/// This thread died, and why.
pub struct Dead<Err> {
    /// id of the thread
    pub thread_id: ThreadId,
    /// cause of death
    pub cause: Cause<Err>,
}

/// Cause of death
pub enum Cause<Err> {
    /// Natural causes
    Natural(Result<(), Err>),
    /// Thread paniced and then died
    Paniced(Box<dyn Any + Send>),
}

/// A thread pool for joining many threads at once, panicking
/// if any of the threads panicked.
pub struct Pool<Err> {
    threads: HashMap<ThreadId, Thread>,
    tx: chan::Sender<Dead<Err>>,
    rx: chan::Receiver<Dead<Err>>,
    logger: slog::Logger,
}

impl<Err> Pool<Err> {
    /// Construct a new thread pool.
    pub fn new(logger: slog::Logger) -> Pool<Err> {
        let (tx, rx) = chan::unbounded();
        Pool {
            threads: HashMap::new(),
            tx,
            rx,
            logger,
        }
    }

    /// Spawn a sub-thread which is joined at the same time as all the
    /// remaining threads.
    pub fn spawn<N, F>(&mut self, name: N, f: F) -> Result<(), std::io::Error>
    where
        N: Into<String>,
        F: FnOnce() -> Result<(), Err>,
        F: std::panic::UnwindSafe,
        F: Send + 'static,
        Err: Send + 'static,
    {
        let name = name.into();
        let name2 = name.clone();
        let builder = thread::Builder::new().name(name.clone());
        let logger = self.logger.clone();

        let tx = self.tx.clone();
        let handle = builder.spawn(move || {
            let thread_id = thread::current().id();
            let cause = match std::panic::catch_unwind(f) {
                Ok(res) => Cause::Natural(res),
                Err(panic) => Cause::Paniced(panic),
            };
            match tx.send(Dead { thread_id, cause }) {
                Ok(()) => {}
                Err(chan::SendError(_)) => {
                    debug!(logger, "thread died, but pool paniced"; "thread_name" => &name2)
                }
            }
        })?;

        self.threads.insert(
            handle.thread().id(),
            Thread {
                name,
                join_handle: handle,
            },
        );

        Ok(())
    }

    /// Attempt to join all threads, and if any of them panicked,
    /// also panic this thread.
    pub fn join_all_or_panic(&mut self) -> Result<(), Err> {
        loop {
            if self.threads.is_empty() {
                return Ok(());
            }

            let death = self
                .rx
                .recv()
                .expect("thread pool: Failed to receive a ThreadResult, even though there are more threads.");

            let thread = self
                .threads
                .remove(&death.thread_id)
                .expect("thread pool: Failed to find thread ID in handle table");

            let name = thread.name;
            thread
                .join_handle
                .join()
                // If the thread panics without an unwindable panic,
                // there’s nothing we can do here.
                // Otherwise the stack is unrolled via Cause::Paniced
                .unwrap_or_else(|_any| {
                    panic!(
                        "thread pool: thread {} paniced and we were unable to unwind it",
                        name
                    )
                });

            match death.cause {
                // The thread died successfully
                Cause::Natural(Ok(())) => {}
                // The thread didn’t panic, it returned an error, so we return early
                Cause::Natural(Err(err)) => return Err(err),
                Cause::Paniced(panic) => std::panic::resume_unwind(panic),
            }
        }
    }
}
