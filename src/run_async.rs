//! Run a function asynchronously.
use crate::thread::Pool;
use crossbeam_channel as chan;

/// Asynchronously execute an action, by executing it in a thread.
///
/// ATTN: dropping this will wait for the action to finish, and thus might block for a long time.
pub struct Async<Res> {
    // we use a pool with exactly one thread,
    // because our thread pool will do the resume_unwind dance for us
    // and thus not lose the stack trace like the naïve implementation would.
    thread: Pool<()>,
    result_chan: chan::Receiver<Res>,
    /// whether the thread should linger (aka detach) after this Async is dropped
    linger: bool,
}

impl<Res> Drop for Async<Res> {
    fn drop(&mut self) {
        // when the Async should not linger, we have to wait for it to finish here.
        if !self.linger {
            self.thread
                .join_all_or_panic()
                .expect("The async thread should never return an error");
        }
    }
}

impl<Res: Send + 'static> Async<Res> {
    /// Create a new Async that runs a function in a thread.
    ///
    /// You can read the result either by blocking
    /// or by using the `chan` method to get a channel that receives exactly
    /// one result as soon as the the function is done.
    pub fn run<F>(logger: &slog::Logger, f: F) -> Self
    where
        F: FnOnce() -> Res,
        F: std::panic::UnwindSafe,
        F: Send + 'static,
    {
        Self::run_inner(logger, f, false)
    }

    /// Create a new Async that runs a function in a thread.
    ///
    /// It behaves like `run`, until the Async is dropped.
    /// While an Async created by `run` will block until the inner task
    /// is finished, an Async created by this function will detach
    /// and continue lingering in the background.
    ///
    /// It will only be dropped when the program finishes.
    /// The inner thread will detach and not be joined when the task finishes.
    pub fn run_and_linger<F>(logger: &slog::Logger, f: F) -> Self
    where
        F: FnOnce() -> Res,
        F: std::panic::UnwindSafe,
        F: Send + 'static,
    {
        Self::run_inner(logger, f, true)
    }

    fn run_inner<F>(logger: &slog::Logger, f: F, linger: bool) -> Self
    where
        F: FnOnce() -> Res,
        F: std::panic::UnwindSafe,
        F: Send + 'static,
    {
        let (tx, rx) = chan::bounded(1);
        let mut thread = Pool::new(logger.clone());

        thread.spawn("async thread", move || {
            let res = f();
            match tx.try_send(res) {
                Ok(()) => Ok(()),
                Err(err) => panic!("unable to send the async result, because the channel was disconnected (should never happen): {:?}", err)
            }
        }).expect("unable to spawn Async thread, should not happen");

        Async {
            thread,
            result_chan: rx,
            linger,
        }
    }

    /// Block until the result is ready.
    /// Consumes the Async.
    pub fn block(mut self) -> Res {
        let res = match self.result_chan.recv() {
            Ok(res) => res,
            Err(chan::RecvError) =>
                panic!("unable to receive the async result, because the channel was disconnected and empty (should never happen)")
        };
        // since the function finished, the thread (created in `run()`) will join immediatly after
        self.thread
            .join_all_or_panic()
            .expect("The async thread should never return an error");
        res
    }

    /// Generate a channel receiving the result of the Async once it’s ready.
    ///
    /// Make sure you don’t drop the Async before the result is ready,
    /// since that will block until the function given to `run` is done.
    ///
    /// Also if you are waiting on the channel result, don’t block on it as well,
    /// since that introduces a race condition.
    pub fn chan(&self) -> chan::Receiver<Res> {
        self.result_chan.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chan_drop_order() {
        // we make the async just block on a channel which we can control from outside
        let (tx, rx) = chan::bounded(1);
        let a = Async::run(&crate::logging::test_logger(), move || rx.recv());
        let c = a.chan();
        // nothing has been sent to the thread yet, so timeout
        assert_eq!(
            c.recv_timeout(std::time::Duration::from_millis(1)),
            Err(chan::RecvTimeoutError::Timeout)
        );
        // now finish the thread, which will give us the return value on the chan
        tx.send(42).unwrap();
        assert_eq!(
            c.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(Ok(42))
        );
        // and we can drop the async, it should not block because the thread has finished and can be joined
        drop(a)
    }

    #[test]
    fn test_chan_block_still_works() {
        // check that even after getting a channel the blocking still works
        let a = Async::run(&crate::logging::test_logger(), move || 42);
        let c = a.chan();
        assert_eq!(a.block(), 42);
        // would be disconnected, because the result was already retrieved by the block
        // and the async went out of scope.
        // This API is a bit unsafe, but exposing the channel will lead to stuff like this.
        // We only ever return one result, so if you try to fetch it twice one will fail.
        assert_eq!(
            c.recv_timeout(std::time::Duration::from_millis(1)),
            Err(chan::RecvTimeoutError::Disconnected)
        );
        // At least you can’t block twice, because the .block() call consumes the Async
    }
}
