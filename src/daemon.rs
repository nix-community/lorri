//! The lorri daemon, watches multiple projects in the background.

pub mod client;
pub mod server;

use crate::build_loop::{BuildLoop, Event};
use crate::nix::options::NixOptions;
use crate::ops::error::ExitError;
use crate::socket::communicate;
use crate::socket::path::SocketPath;
use crate::{project, AbsPathBuf, NixFile};
use crossbeam_channel as chan;
use slog::debug;
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// Events created by the event loop.
///
/// Union of build_loop::Event and NewListener for internal use.
pub enum LoopHandlerEvent {
    /// A new listener has joined for event streaming
    NewListener(chan::Sender<Event>),
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
    pub nix_file: NixFile,
    /// Determines when this activity will cause a rebuild.
    pub rebuild: communicate::Rebuild,
}

/// Keeps all state of the running `lorri daemon` service, watches nix files and runs builds.
pub struct Daemon {
    /// Sending end that we pass to every `BuildLoop` the daemon controls.
    // TODO: this needs to transmit information to identify the builder with
    tx_build_events: chan::Sender<LoopHandlerEvent>,
    rx_build_events: chan::Receiver<LoopHandlerEvent>,
    mon_tx: chan::Sender<LoopHandlerEvent>,
    /// Extra options to pass to each nix invocation
    extra_nix_options: NixOptions,
}

impl Daemon {
    /// Create a new daemon. Also return an `chan::Receiver` that
    /// receives `LoopHandlerEvent`s for all builders this daemon
    /// supervises.
    pub fn new(extra_nix_options: NixOptions) -> (Daemon, chan::Receiver<LoopHandlerEvent>) {
        let (tx_build_events, rx_build_events) = chan::unbounded();
        let (mon_tx, mon_rx) = chan::unbounded();
        (
            Daemon {
                tx_build_events,
                rx_build_events,
                mon_tx,
                extra_nix_options,
            },
            mon_rx,
        )
    }

    /// Serve the daemon's RPC endpoint.
    pub fn serve(
        &mut self,
        socket_path: &SocketPath,
        gc_root_dir: &AbsPathBuf,
        cas: crate::cas::ContentAddressable,
        nix_gc_root_user_dir: project::NixGcRootUserDir,
        logger: &slog::Logger,
    ) -> Result<(), ExitError> {
        let (tx_activity, rx_activity): (
            chan::Sender<IndicateActivity>,
            chan::Receiver<IndicateActivity>,
        ) = chan::unbounded();

        let mut pool = crate::thread::Pool::new(logger.clone());
        let tx_build_events = self.tx_build_events.clone();

        let server = server::Server::new(tx_activity, tx_build_events);

        let socket_path = socket_path.clone();
        let logger = logger.clone();
        let logger2 = logger.clone();
        let logger3 = logger.clone();

        pool.spawn("accept-loop", move || {
            server.listen(&socket_path, &logger).map(|n| n.never())
        })?;

        let rx_build_events = self.rx_build_events.clone();
        let mon_tx = self.mon_tx.clone();
        pool.spawn("build-loop", move || {
            Self::build_loop(rx_build_events, mon_tx, &logger2);
            Ok(())
        })?;

        let tx_build_events = self.tx_build_events.clone();
        let extra_nix_options = self.extra_nix_options.clone();
        let gc_root_dir = gc_root_dir.clone();
        pool.spawn("build-instruction-handler", move || {
            Self::build_instruction_handler(
                tx_build_events,
                extra_nix_options,
                rx_activity,
                &gc_root_dir,
                cas,
                nix_gc_root_user_dir,
                &logger3,
            );
            Ok(())
        })?;

        pool.join_all_or_panic()?;

        Ok(())
    }

    fn build_loop(
        rx_build_events: chan::Receiver<LoopHandlerEvent>,
        mon_tx: chan::Sender<LoopHandlerEvent>,
        logger: &slog::Logger,
    ) {
        let mut project_states: HashMap<NixFile, Event> = HashMap::new();
        let mut event_listeners: Vec<chan::Sender<Event>> = Vec::new();

        for msg in rx_build_events {
            mon_tx
                .send(msg.clone())
                .expect("listener still to be there");
            match &msg {
                LoopHandlerEvent::BuildEvent(ev) => match ev {
                    Event::SectionEnd => (),
                    Event::Started { nix_file, .. }
                    | Event::Completed { nix_file, .. }
                    | Event::Failure { nix_file, .. } => {
                        project_states.insert(nix_file.clone(), ev.clone());
                        event_listeners.retain(|tx| {
                            let keep = tx.send(ev.clone()).is_ok();
                            debug!(logger,"Sent"; "event" => ?ev, "keep" => keep);
                            keep
                        })
                    }
                },
                LoopHandlerEvent::NewListener(tx) => {
                    debug!(logger, "adding listener");
                    let keep = project_states.values().all(|event| {
                        let keeping = tx.send(event.clone()).is_ok();
                        debug!(logger, "Sent snapshot"; "event" => ?&event, "keep" => keeping);
                        keeping
                    });
                    debug!(logger,"Finished snapshot"; "keep" => keep);
                    if keep {
                        event_listeners.push(tx.clone());
                    }
                    event_listeners.retain(|tx| {
                        let keep = tx.send(Event::SectionEnd).is_ok();
                        debug!(logger, "Sent new listener sectionend"; "keep" => keep);
                        keep
                    })
                }
            }
        }
    }

    fn build_instruction_handler(
        // TODO: use the pool here
        // pool: &mut crate::thread::Pool,
        tx_build_events: chan::Sender<LoopHandlerEvent>,
        extra_nix_options: NixOptions,
        rx_activity: chan::Receiver<IndicateActivity>,
        gc_root_dir: &AbsPathBuf,
        cas: crate::cas::ContentAddressable,
        nix_gc_root_user_dir: project::NixGcRootUserDir,
        logger: &slog::Logger,
    ) {
        // A thread for each `BuildLoop`, keyed by the nix files listened on.
        let mut handler_threads: HashMap<NixFile, chan::Sender<()>> = HashMap::new();

        // For each build instruction, add the corresponding file
        // to the watch list.
        for IndicateActivity { nix_file, rebuild } in rx_activity {
            let project = crate::project::Project::new(nix_file, gc_root_dir, cas.clone())
                // TODO: the project needs to create its gc root dir
                .unwrap();

            let key = project.nix_file.clone();
            let project_is_watched = handler_threads.get(&key);

            let send_ping =
                |to: &chan::Sender<()>| to.send(()).expect("could not ping the build loop");

            match (project_is_watched, rebuild) {
                (Some(builder), communicate::Rebuild::Always) => {
                    debug!(logger, "triggering rebuild"; "project" => key, "cause" => "unconditional ping");
                    send_ping(builder)
                }
                (Some(_), communicate::Rebuild::OnlyIfNotYetWatching) => {
                    debug!(logger, "skipping rebuild"; "project" => key, "cause" => "already watching");
                }
                // only add if there is no no build_loop for this file yet.
                (None, _) => {
                    let (tx_ping, rx_ping) = chan::unbounded();
                    // cloning the tx means the daemon’s rx gets all
                    // messages from all builders.
                    let tx_build_events = tx_build_events.clone();
                    let extra_nix_options = extra_nix_options.clone();
                    let nix_gc_root_user_dir = nix_gc_root_user_dir.clone();
                    let logger = logger.clone();
                    let logger2 = logger.clone();
                    // TODO: how to use the pool here?
                    // We cannot just spawn new threads once messages come in,
                    // because then then pool objects is stuck in this loop
                    // and will never start to wait for joins, which means
                    // we don’t catch panics as they happen!
                    // If we can get the pool to “wait for join but also spawn new
                    // thread when you get a message” that could work!
                    // pool.spawn(format!("build_loop for {}", nix_file.display()),
                    let _ = std::thread::spawn(move || {
                        match BuildLoop::new(
                            &project,
                            extra_nix_options,
                            nix_gc_root_user_dir,
                            logger,
                        ) {
                            Ok(mut build_loop) => {
                                build_loop.forever(tx_build_events, rx_ping).never()
                            }
                            Err(err) =>
                            // TODO: omg this is so bad, too many layers of wrapping
                            {
                                tx_build_events
                                    .send(LoopHandlerEvent::BuildEvent(Event::Failure {
                                        nix_file: project.nix_file.clone(),
                                        failure: crate::builder::BuildError::Io {
                                            msg: err
                                                .context(format!(
                                                    "could not start the watcher for {}",
                                                    &project.nix_file.display()
                                                ))
                                                .to_string(),
                                        },
                                    }))
                                    .expect("rx_build_events hung up")
                            }
                        }
                    });

                    let e = handler_threads.insert(key.clone(), tx_ping.clone());
                    match e {
                        None => {}
                        Some(_) => {
                            panic!("handler_threads had the key, but we already checked before")
                        }
                    }
                    debug!(logger2, "triggering rebuild"; "project" => key, "cause" => "new project");
                    send_ping(&tx_ping);
                }
            }
        }
    }
}
