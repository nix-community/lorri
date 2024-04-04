//! Serve the lorri daemon on a unix socket.
use crate::daemon::{IndicateActivity, LoopHandlerEvent};
use crate::run_async::Async;
use crate::socket::communicate;
use crate::socket::communicate::listener::{Connection, Listener};
use crate::socket::communicate::{CommunicationType, Ping, StreamEvents};
use crate::socket::path::{BindError, SocketPath};
use crate::Never;
use crossbeam_channel as chan;
use slog::{debug, info};
use std::collections::HashMap;
use std::thread;

/// Native backend Server
pub struct Server {
    tx_activity: chan::Sender<IndicateActivity>,
    tx_build: chan::Sender<LoopHandlerEvent>,
}

impl Server {
    /// Create a new server.
    pub fn new(
        tx_activity: chan::Sender<IndicateActivity>,
        tx_build: chan::Sender<LoopHandlerEvent>,
    ) -> Self {
        Server {
            tx_activity,
            tx_build,
        }
    }

    /// Listen for incoming clients. Goes into an accept() loop, thus blocks.
    pub fn listen(
        &self,
        socket_path: &SocketPath,
        logger: &slog::Logger,
    ) -> Result<Never, BindError> {
        let listener = Listener::new(socket_path)?;

        // We have to continuously be joining threads,
        // otherwise they turn into zombies and we eventually run out of processes on linux.
        let (tx_new_thread, rx_new_thread) = chan::unbounded();
        let (tx_done_thread, rx_done_thread) = chan::unbounded();
        let logger2 = logger.clone();
        let _joiner = Async::run(logger, move || {
            join_continuously(rx_new_thread, rx_done_thread, &logger2)
        });

        loop {
            let tx_done_thread = tx_done_thread.clone();
            match listener.accept() {
                Ok(connection) => {
                    self.handle_client(connection, tx_new_thread.clone(), tx_done_thread, logger)
                }
                Err(accept_err) => {
                    info!(logger, "Failed accepting a client connection"; "accept_error" => format!("{:?}", accept_err));
                    // If we hit an error like `too many open file descriptors`, avoid retrying
                    // immediately and hogging the CPU in a busy loop.
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    fn handle_client(
        &self,
        conn: Connection,
        tx_new_thread: chan::Sender<(String, CommunicationType, std::thread::JoinHandle<()>)>,
        tx_done_thread: chan::Sender<std::thread::ThreadId>,
        logger: &slog::Logger,
    ) {
        let Connection {
            handlers,
            communication_type,
        } = conn;

        // We can’t display thread ids, so let’s generate a short random string to identify a thread
        let display_id: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(4)
            .collect();
        let display_id_copy = display_id.clone();

        let tx_activity = self.tx_activity.clone();
        let tx_build = self.tx_build.clone();
        let logger = logger.clone();

        let new_thread = std::thread::spawn(move || {
            let id = thread::current().id();
            debug!(&logger, "New client connection accepted"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);

            let err = |ct, e| debug!(logger, "Unable to communicate with client"; "communication_type" => format!("{:?}", ct), "error" => format!("{:?}", e));

            // catch any panics that happen, to be able to send a done message in any case
            let res = std::panic::catch_unwind(|| {
                // handle all events
                // TODO: it would be good if we didn’t have to match on the communication type here, but I don’t see a way to do that.
                match communication_type {
                    CommunicationType::Ping => {
                        match handlers.ping().read(communicate::DEFAULT_READ_TIMEOUT) {
                            Ok(Ping { nix_file, rebuild }) => tx_activity
                                .send(IndicateActivity { nix_file, rebuild })
                                .expect("Unable to send a ping from listener"),
                            Err(e) => err(communication_type, e),
                        }
                    }
                    CommunicationType::StreamEvents => {
                        let mut rw = handlers.stream_events();
                        match rw.read(communicate::DEFAULT_READ_TIMEOUT) {
                            Ok(StreamEvents {}) => {
                                let (tx_event, rx_event) = chan::unbounded();
                                tx_build
                                    .send(LoopHandlerEvent::NewListener(tx_event))
                                    .expect("Unable to send a new listener to the build_loop");
                                for event in rx_event {
                                    match rw.write(communicate::DEFAULT_READ_TIMEOUT, &event) {
                                        Ok(()) => {}
                                        Err(err) => {
                                            debug!(logger, "client vanished, closing socket"; "communication_type" => format!("{:?}", communication_type), "error" => format!("{:?}", err));
                                            // break out of the loop or the handler is not stopped
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => err(communication_type, e),
                        }
                    }
                }
            });

            tx_done_thread
                .send(id)
                .expect("Server::listen: done channel closed");

            // if a panic happened, continue it after sending the message
            match res {
                Err(panic) => std::panic::resume_unwind(panic),
                Ok(()) => {}
            }

            debug!(logger, "Client connection handled"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);
        });

        tx_new_thread
            .send((display_id_copy, communication_type, new_thread))
            .expect("Serve::listen: new thread channel closed");
    }
}

/// Join threads continuously, so that we don’t generate too many zombies.
/// Every new thread it signalled by `rx_new`, while every thread that is finishe
/// sends its id to `rx_done`.
/// Ideally, we’d add a method like that to our thread::Pool, but it was too hard for now.
fn join_continuously(
    rx_new: chan::Receiver<(String, CommunicationType, std::thread::JoinHandle<()>)>,
    rx_done: chan::Receiver<thread::ThreadId>,
    logger: &slog::Logger,
) {
    let mut running = HashMap::new();
    loop {
        chan::select! {
            recv(rx_new) -> msg => match msg {
                Ok(new) => {
                    let _ = running.insert(new.2.thread().id(), new);
                },
                Err(chan::RecvError) => panic!("Serve::listen: new thread channel closed")
            },
            recv(rx_done) -> msg => match msg {
                Ok(done) => match running.remove(&done) {
                    // this join should be instant, provided the message was correct
                    // and sent right before the thread exited.
                    Some(thread) => {
                        let message_type = format!("{:?}", thread.2);
                        let display_id = thread.0.clone();
                        match thread.2.join() {
                            Ok(()) => {},
                            Err(_panic) => info!(logger, "Server::accept: a connect thread panicked"; "message_type" => message_type, "thread_id" => &display_id),
                        }
                        debug!(logger, "joined thread"; "thread_id" => &display_id);
                    },
                    None => panic!("Server::accept: join_continuously was sent a threadId it did not know about."),
                },
                Err(chan::RecvError) => panic!("Serve::listen: done channel closed"),
            }
        }
    }
}
