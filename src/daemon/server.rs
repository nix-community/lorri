//! Serve the lorri daemon on a unix socket.
use crate::daemon::{IndicateActivity, LoopHandlerEvent};
use crate::socket::communicate::listener::{handlers, Connection, Listener};
use crate::socket::communicate::{self};
use crate::socket::communicate::{CommunicationType, Ping};
use communicate::DaemonInfo;
use slog::{debug, error, info};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::mpsc::{channel, Sender, UnboundedSender};
use tokio::sync::oneshot;

/// Native backend Server
#[derive(Clone)]
pub struct Server {
    tx_activity: Sender<IndicateActivity>,
    tx_build: UnboundedSender<LoopHandlerEvent>,
}

impl Server {
    /// Create a new server.
    pub fn new(
        tx_activity: Sender<IndicateActivity>,
        tx_build: UnboundedSender<LoopHandlerEvent>,
    ) -> Self {
        Server {
            tx_activity,
            tx_build,
        }
    }

    /// Listen for incoming clients. Goes into an accept() loop, thus blocks.
    pub async fn listen(self, listener: Listener, logger: slog::Logger) {
        loop {
            let logger2 = logger.clone();
            let self2 = self.clone();

            // make sure each thread is accepted but then spawned into a task
            // so that we can accept multiple clients simultaneously
            let conn = listener.accept().await;

            tokio::spawn(async move {
                match conn {
                    Ok(connection) => {
                        let mut sock = self2.clone().handle_client(connection, &logger2).await;
                        if let Err(_) = sock.shutdown().await {};
                        drop(sock)
                    }
                    Err(accept_err) => {
                        info!(logger2, "Failed accepting a client connection"; "accept_error" => format!("{:?}", accept_err));
                    }
                }
            });
        }
    }

    async fn handle_client(&self, conn: Connection, logger: &slog::Logger) -> UnixStream {
        let Connection {
            socket,
            communication_type,
        } = conn;

        // We can’t display thread ids, so let’s generate a short random string to identify a thread
        let display_id: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(4)
            .collect();

        let tx_activity = self.tx_activity.clone();
        let tx_build = self.tx_build.clone();
        let logger = logger.clone();

        {
            debug!(&logger, "New client connection accepted"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);

            let err = |ct, e| debug!(logger, "Unable to communicate with client"; "communication_type" => format!("{:?}", ct), "error" => format!("{:?}", e));

            let socket = {
                // handle all events
                // TODO: it would be good if we didn’t have to match on the communication type here, but I don’t see a way to do that.
                match communication_type {
                    CommunicationType::DaemonInfo => {
                        let mut di = handlers::daemon_info(socket);
                        match di.read(communicate::DEFAULT_READ_TIMEOUT).await {
                            Ok((_, DaemonInfo {})) => {
                                let mut rw = handlers::daemon_info(di.into_inner());
                                // TODO: the server could return its info here
                                match rw.write(communicate::DEFAULT_READ_TIMEOUT, &()).await {
                                    Ok(_timeout) => {}
                                    Err(err) => {
                                        debug!(logger, "client vanished, closing socket"; "communication_type" => format!("{:?}", communication_type), "error" => format!("{:?}", err));
                                    }
                                }
                                rw.into_inner()
                            }
                            Err(_) => todo!(),
                        }
                    }
                    CommunicationType::Ping => {
                        let mut rw = handlers::ping(socket);

                        match rw.read(communicate::DEFAULT_READ_TIMEOUT).await {
                            Ok((
                                _,
                                Ping {
                                    project_file,
                                    rebuild,
                                },
                            )) => tx_activity
                                .send(IndicateActivity {
                                    project_file,
                                    rebuild,
                                })
                                .await
                                .expect("Unable to send a ping from listener"),
                            Err(e) => err(communication_type, e),
                        };
                        rw.into_inner()
                    }
                    CommunicationType::StreamEvents => {
                        let mut rw = handlers::stream_events(socket);
                        let (tx_event, mut rx_event) = channel(10);
                        tx_build
                            .send(LoopHandlerEvent::EventStreamListener(tx_event))
                            .expect("Unable to send a new listener to the build_loop");
                        loop {
                            match rx_event.recv().await {
                                None => break,
                                Some(event) => {
                                    match rw.write(communicate::DEFAULT_READ_TIMEOUT, &event).await
                                    {
                                        Ok(_) => {}
                                        Err(err) => {
                                            debug!(logger, "client vanished, closing socket"; "communication_type" => format!("{:?}", communication_type), "error" => format!("{:?}", err));
                                            // break out of the loop or the handler is not stopped
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        rw.into_inner()
                    }
                    CommunicationType::StreamSnapshot => {
                        let mut rw = handlers::stream_snapshot(socket);
                        let (tx_snapshot, rx_snapshot) = oneshot::channel();
                        tx_build
                            .send(LoopHandlerEvent::SnapshotListener(tx_snapshot))
                            .expect("Unable to send a new listener to the build_loop");

                        match rx_snapshot.await {
                            Err(e) => {
                                error!(logger, "Snapshot oneshot was closed"; "error" => ?e)
                            }
                            Ok(snapshot) => {
                                if let Err(_) =
                                    rw.write(communicate::DEFAULT_READ_TIMEOUT, &snapshot).await
                                {
                                };
                            }
                        }

                        rw.into_inner()
                    }
                }
            };

            debug!(logger, "Client connection handled"; "message_type" => format!("{:?}", communication_type), "thread_id" => &display_id);

            socket
        }
    }
}
