//! The lorri daemon, watches multiple projects in the background.

pub mod client;
pub mod server;

use crate::build_loop::{BuildLoop, Event, EventSnapshot};
use crate::nix::options::NixOptions;
use crate::ops::error::ExitError;
use crate::project::ProjectFile;
use crate::socket::communicate;
use crate::socket::communicate::listener::Listener;
use crate::socket::path::SocketPath;
use crate::sqlite::Sqlite;
use crate::{AbsPathBuf, NixFile};
use slog::{debug, info};
use std::collections::HashMap;
use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};
use tokio::task::JoinSet;

#[derive(Debug)]
/// Union of build_loop::Event and NewListener for internal use.
pub enum LoopHandlerEvent {
    /// A new listener has joined for event streaming
    EventStreamListener(Sender<Event>),
    /// A new listener has requested the current snapshot
    SnapshotListener(tokio::sync::oneshot::Sender<EventSnapshot>),
    /// Events from a BuildLoop
    BuildEvent(Event),
}

/// Indicate that the user is interested in a specific nix file.
/// Usually a nix file describes the environment of a project,
/// so the user editor would send this message when a file
/// in the project is opened, through `lorri direnv` for example.
///
/// `lorri internal ping` is the internal command which triggers this signal
/// and forces a rebuild.
pub struct IndicateActivity {
    /// This nix file should be build/watched by the daemon.
    pub project_file: ProjectFile,
    /// Determines when this activity will cause a rebuild.
    pub rebuild: communicate::Rebuild,
}

/// Keeps all state of the running `lorri daemon` service, watches nix files and runs builds.
pub struct Daemon {
    /// Sending end that we pass to every `BuildLoop` the daemon controls.
    // TODO: this needs to transmit information to identify the builder with
    tx_build_events: UnboundedSender<LoopHandlerEvent>,
    rx_build_events: UnboundedReceiver<LoopHandlerEvent>,
    /// Extra options to pass to each nix invocation
    extra_nix_options: NixOptions,
}

impl Daemon {
    /// Create a new daemon. Also return an `chan::Receiver` that
    /// receives `LoopHandlerEvent`s for all builders this daemon
    /// supervises.
    pub fn new(extra_nix_options: NixOptions) -> Daemon {
        let (tx_build_events, rx_build_events) = unbounded_channel();
        Daemon {
            tx_build_events,
            rx_build_events,
            extra_nix_options,
        }
    }

    /// Serve the daemon's RPC endpoint.
    pub async fn serve(
        self,
        socket_path: &SocketPath,
        sqlite_path: &AbsPathBuf,
        gc_root_dir: &AbsPathBuf,
        cas: crate::cas::ContentAddressable,
        logger: &slog::Logger,
    ) -> Result<(), ExitError> {
        let (tx_activity, rx_activity): (Sender<IndicateActivity>, Receiver<IndicateActivity>) =
            channel(10);

        let socket_path = socket_path.clone();
        let logger = logger.clone();
        let logger2 = logger.clone();
        let logger3 = logger.clone();

        let server = server::Server::new(tx_activity, self.tx_build_events.clone());
        let listener = Listener::new(&socket_path).await?;
        tokio::task::spawn_local(server.listen(listener, logger));

        let build_loop_hdl =
            tokio::task::spawn_local(Self::build_loop(self.rx_build_events, logger2));

        let tx_build_events = self.tx_build_events.clone();
        let extra_nix_options = self.extra_nix_options.clone();
        let gc_root_dir = gc_root_dir.clone();
        let sqlite_path = sqlite_path.clone();
        let conn = Sqlite::new_connection(&sqlite_path).await;
        let join_set = Self::build_instruction_handler(
            tx_build_events,
            extra_nix_options,
            rx_activity,
            &gc_root_dir,
            conn,
            cas,
            &logger3,
        )
        .await;

        join_set.join_all().await;
        build_loop_hdl.await.expect("build loop error");
        Ok(())
    }

    async fn build_loop(
        mut rx_build_events: UnboundedReceiver<LoopHandlerEvent>,
        logger: slog::Logger,
    ) {
        let mut project_states: HashMap<NixFile, Event> = HashMap::new();
        let mut build_event_listeners: Vec<Option<Sender<Event>>> = Vec::new();

        loop {
            let Some(msg) = rx_build_events.recv().await else {
                break;
            };
            match msg {
                LoopHandlerEvent::BuildEvent(mut event) => {
                    let nix_file = match &mut event {
                        Event::Started { nix_file, .. }
                        | Event::Completed { nix_file, .. }
                        | Event::Failure { nix_file, .. } => nix_file.clone(),
                    };
                    info!(logger, "build status"; "event" => ?event);
                    project_states.insert(nix_file.clone(), event.clone());
                    // send to all listeners & remove any listeners that are closed automatically
                    for listener in build_event_listeners.iter_mut() {
                        if let Some(l) = listener {
                            if l.send(event.clone()).await.is_err() {
                                // we have to jump through this Some/None hoop
                                // because we can’t send().await inside vec.retain() directly
                                *listener = None
                            }
                        }
                    }
                    build_event_listeners.retain(|l| l.is_some());
                    debug!(logger,"Sent"; "event" => ?event);
                }
                LoopHandlerEvent::EventStreamListener(tx) => {
                    debug!(logger, "Adding EventStreamListener");
                    build_event_listeners.push(Some(tx.clone()));
                }
                LoopHandlerEvent::SnapshotListener(tx) => {
                    debug!(logger, "Answering SnapshotListener");
                    let states: Vec<_> = project_states.clone().into_values().collect();
                    let _ = tx.send(EventSnapshot { snapshot: states });
                    debug!(logger, "Sent snapshot"; "snapshot" => ?&project_states);
                }
            }
        }
    }

    async fn build_instruction_handler(
        tx_build_events: UnboundedSender<LoopHandlerEvent>,
        extra_nix_options: NixOptions,
        mut rx_activity: Receiver<IndicateActivity>,
        gc_root_dir: &AbsPathBuf,
        conn: Sqlite,
        cas: crate::cas::ContentAddressable,
        logger: &slog::Logger,
    ) -> JoinSet<()> {
        // A thread for each `BuildLoop`, keyed by the nix files listened on.
        let mut handler_threads: HashMap<NixFile, Sender<()>> = HashMap::new();

        let mut join_set = JoinSet::new();

        // For each build instruction, add the corresponding file
        // to the watch list.
        loop {
            let Some(IndicateActivity {
                project_file,
                rebuild,
            }) = rx_activity.recv().await
            else {
                break;
            };
            let project = crate::project::Project::new_and_gc_nix_files(
                conn.clone(),
                logger.clone(),
                project_file,
                gc_root_dir,
            )
            .await
            // TODO: the project needs to create its gc root dir
            .unwrap();

            let key = project.project_file.as_nix_file().clone();
            let project_is_watched = handler_threads.get(&key);

            match (project_is_watched, rebuild) {
                (Some(builder), communicate::Rebuild::Always) => {
                    debug!(logger, "triggering rebuild"; "project" => key, "cause" => "unconditional ping");
                    builder
                        .send(())
                        .await
                        .expect("could not ping the build loop");
                }
                (Some(_), communicate::Rebuild::OnlyIfNotYetWatching) => {
                    debug!(logger, "skipping rebuild"; "project" => key, "cause" => "already watching");
                }
                // only add if there is no no build_loop for this file yet.
                (None, _) => {
                    let (tx_ping, rx_ping) = channel(10);
                    // cloning the tx means the daemon’s rx gets all
                    // messages from all builders.
                    let tx_build_events = tx_build_events.clone();
                    let extra_nix_options = extra_nix_options.clone();
                    let logger = logger.clone();
                    let logger2 = logger.clone();
                    let cas2 = cas.clone();
                    let project_file = project.project_file.as_nix_file();

                    match BuildLoop::new(project, extra_nix_options, cas2, logger) {
                        Ok(build_loop) => {
                            let _ = join_set.spawn_local(async move {
                                build_loop.forever(tx_build_events, rx_ping).await
                            });
                        }
                        Err(err) =>
                        // TODO: omg this is so bad, too many layers of wrapping
                        {
                            tx_build_events
                                .send(LoopHandlerEvent::BuildEvent(Event::Failure {
                                    nix_file: project_file.clone(),
                                    failure: crate::builder::BuildError::Io {
                                        msg: err
                                            .context(format!(
                                                "could not start the watcher for {}",
                                                &project_file.display()
                                            ))
                                            .to_string(),
                                    },
                                }))
                                .expect("rx_build_events hung up")
                        }
                    }

                    let e = handler_threads.insert(key.clone(), tx_ping.clone());
                    match e {
                        None => {}
                        Some(_) => {
                            panic!("handler_threads had the key, but we already checked before")
                        }
                    }
                    debug!(logger2, "triggering rebuild"; "project" => key, "cause" => "new project");
                    tx_ping
                        .send(())
                        .await
                        .expect("could not ping the build loop");
                }
            }
        }

        join_set
    }
}
