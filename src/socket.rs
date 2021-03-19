//! Modules to set up communication between client and server over a unix socket.
pub mod communicate;
pub mod path;
pub mod read_writer;

use crate::daemon::{IndicateActivity, LoopHandlerEvent};
use crate::ops::error::ExitError;
use crate::run_async::Async;
use crate::socket::path::SocketPath;
use crate::Never;
use communicate::{CommunicationType, Ping, StreamEvents};
use crossbeam_channel as chan;
use slog_scope::{debug, info};
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
    pub fn listen<'a>(&'a self, socket_path: &SocketPath) -> Result<Never, ExitError> {
        let listener = communicate::listener::Listener::new(socket_path)
            // TODO: this is bad, but mirroring what the varlink code does for now. this code should not return exit errors
            .map_err(|e| ExitError::temporary(format!("{:?}", e)))?;

        // We have to continuously be joining threads,
        // otherwise they turn into zombies and we eventually run out of processes on linux.
        let (tx_new_thread, rx_new_thread) = chan::unbounded();
        let (tx_done_thread, rx_done_thread) = chan::unbounded();
        let _joiner = Async::run(slog_scope::logger(), || {
            join_continuously(rx_new_thread, rx_done_thread)
        });

        loop {
            let tx_done_thread = tx_done_thread.clone();
            let tx_activity = self.tx_activity.clone();
            let tx_build = self.tx_build.clone();
            // We can’t display thread ids, so let’s generate a short random string to identify a thread
            let display_id: String = std::iter::repeat_with(|| fastrand::alphanumeric())
                .take(4)
                .collect();
            let display_id_copy = display_id.clone();

            let accept = listener.accept(
                move |communication_type, handlers| {
                    let id = thread::current().id();
                    debug!("New client connection accepted"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);

                    fn err<E>(ct: CommunicationType, e: E)
                        where E: std::fmt::Debug
                    {
                        debug!("Unable to communicate with client"; "communication_type" => format!("{:?}", ct), "error" => format!("{:?}", e))
                    }

                    // catch any panics that happen, to be able to send a done message in any case
                    let res = std::panic::catch_unwind(|| {

                        // handle all events
                        // TODO: it would be good if we didn’t have to match on the communication type here, but I don’t see a way to do that.
                        match communication_type {
                            CommunicationType::Ping => {
                                match handlers.ping().read(communicate::DEFAULT_READ_TIMEOUT) {
                                    Ok(Ping { nix_file, rebuild }) => {
                                        tx_activity.send(IndicateActivity {
                                            nix_file,
                                            rebuild
                                        }).expect("Unable to send a ping from listener")
                                    },
                                    Err(e) => err(communication_type, e)
                                }
                            },
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
                                                Ok(()) => {},
                                                Err(err) => debug!("client vanished"; "communication_type" => format!("{:?}", communication_type), "error" => format!("{:?}", err))
                                            }
                                        }
                                    },
                                    Err(e) => err(communication_type, e)
                                }
                            }
                        }

                    });

                    tx_done_thread.send(id).expect("Server::listen: done channel closed");

                    // if a panic happened, continue it after sending the message
                    match res {
                        Err(panic) => std::panic::resume_unwind(panic),
                        Ok(()) => {}
                    }

                    debug!("Client connection handled"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);
                },
            );

            match accept {
                Ok(new_thread) => {
                    tx_new_thread
                        .send((display_id_copy, new_thread))
                        .expect("Serve::listen: new thread channel closed");
                }
                Err(accept_err) => {
                    info!("Failed accepting a client connection"; "accept_error" => format!("{:?}", accept_err))
                }
            }
        }
    }
}

/// Join threads continuously, so that we don’t generate too many zombies.
/// Every new thread it signalled by `rx_new`, while evrey
/// Ideally, we’d add a method like that to our thread::Pool, but it was too hard for now.
fn join_continuously(
    rx_new: chan::Receiver<(String, communicate::listener::Thread)>,
    rx_done: chan::Receiver<thread::ThreadId>,
) {
    let mut running = HashMap::new();
    loop {
        chan::select! {
            recv(rx_new) -> msg => match msg {
                Ok(new) => {
                    let _ = running.insert(new.1.handle.thread().id(), new);
                },
                Err(chan::RecvError) => panic!("Serve::listen: new thread channel closed")
            },
            recv(rx_done) -> msg => match msg {
                Ok(done) => match running.remove(&done) {
                    // this join should be instant, provided the message was correct
                    // and sent right before the thread exited.
                    Some(thread) => {
                        let message_type = format!("{:?}", thread.1.message_type);
                        let display_id = thread.0.clone();
                        match thread.1.handle.join() {
                            Ok(()) => {},
                            Err(_panic) => info!("Server::accept: a connect thread panicked"; "message_type" => message_type, "thread_id" => &display_id),
                        }
                        debug!("joined thread"; "thread_id" => &display_id);
                    },
                    None => panic!("Server::accept: join_continuously was sent a threadId it did not know about."),
                },
                Err(chan::RecvError) => panic!("Serve::listen: done channel closed"),
            }
        }
    }
}
