//! The lorri daemon, watches multiple projects in the background.

use crate::build_loop::{BuildLoop, Event};
use crate::internal_proto;
use crate::nix::options::NixOptions;
use crate::ops::error::ExitError;
use crate::socket::SocketPath;
use crate::{AbsPathBuf, NixFile};
use crossbeam_channel as chan;
use slog_scope::debug;
use std::collections::HashMap;

mod server;

#[derive(Debug, Clone)]
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
    pub rebuild: internal_proto::Rebuild,
}

struct Handler {
    tx_ping: chan::Sender<internal_proto::Rebuild>,
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
        socket_path: SocketPath,
        gc_root_dir: &AbsPathBuf,
        cas: crate::cas::ContentAddressable,
    ) -> Result<(), ExitError> {
        let (tx_activity, rx_activity): (
            chan::Sender<IndicateActivity>,
            chan::Receiver<IndicateActivity>,
        ) = chan::unbounded();

        let mut pool = crate::thread::Pool::new();
        let tx_build_events = self.tx_build_events.clone();

        let server = server::Server::new(socket_path.clone(), tx_activity, tx_build_events)
            .map_err(|e| {
                ExitError::temporary(format!(
                    "unable to bind to the server socket at {}: {:?}",
                    socket_path.as_absolute_path().display(),
                    e
                ))
            })?;

        pool.spawn("accept-loop", || {
            server.serve().expect("varlink error");
        })?;

        let rx_build_events = self.rx_build_events.clone();
        let mon_tx = self.mon_tx.clone();
        pool.spawn("build-loop", || Self::build_loop(rx_build_events, mon_tx))?;

        let tx_build_events = self.tx_build_events.clone();
        let extra_nix_options = self.extra_nix_options.clone();
        let gc_root_dir = gc_root_dir.clone();
        pool.spawn("foo", move || {
            Self::build_instruction_handler(
                tx_build_events,
                extra_nix_options,
                rx_activity,
                &gc_root_dir,
                cas,
            )
        })?;

        pool.join_all_or_panic();

        Ok(())
    }

    fn build_loop(
        rx_build_events: chan::Receiver<LoopHandlerEvent>,
        mon_tx: chan::Sender<LoopHandlerEvent>,
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
                            debug!("Sent"; "event" => ?ev, "keep" => keep);
                            keep
                        })
                    }
                },
                LoopHandlerEvent::NewListener(tx) => {
                    debug!("adding listener");
                    let keep = project_states.values().all(|event| {
                        let keeping = tx.send(event.clone()).is_ok();
                        debug!("Sent snapshot"; "event" => ?&event, "keep" => keeping);
                        keeping
                    });
                    debug!("Finished snapshot"; "keep" => keep);
                    if keep {
                        event_listeners.push(tx.clone());
                    }
                    event_listeners.retain(|tx| {
                        let keep = tx.send(Event::SectionEnd).is_ok();
                        debug!("Sent new listener sectionend"; "keep" => keep);
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
    ) {
        // A thread for each `BuildLoop`, keyed by the nix files listened on.
        let mut handler_threads: HashMap<NixFile, Handler> = HashMap::new();

        // For each build instruction, add the corresponding file
        // to the watch list.
        for IndicateActivity { nix_file, rebuild } in rx_activity {
            let project = crate::project::Project::new(nix_file, gc_root_dir, cas.clone())
                // TODO: the project needs to create its gc root dir
                .unwrap();

            // Add nix file to the set of files this daemon watches
            // & build if they change.
            let (tx_ping, rx_ping) = chan::unbounded();
            // cloning the tx means the daemon’s rx gets all
            // messages from all builders.
            let tx_build_events = tx_build_events.clone();
            let extra_nix_options = extra_nix_options.clone();

            handler_threads
                // only add if there is no no build_loop for this file yet.
                .entry(project.nix_file.clone())
                .or_insert_with(|| {
                    // TODO: how to use the pool here?
                    // We cannot just spawn new threads once messages come in,
                    // because then then pool objects is stuck in this loop
                    // and will never start to wait for joins, which means
                    // we don’t catch panics as they happen!
                    // If we can get the pool to “wait for join but also spawn new
                    // thread when you get a message” that could work!
                    // pool.spawn(format!("build_loop for {}", nix_file.display()),
                    let _ = std::thread::spawn(move || {
                        let mut build_loop = BuildLoop::new(&project, extra_nix_options);

                        build_loop.forever(tx_build_events, rx_ping);
                    });
                    Handler { tx_ping }
                })
                // Notify the handler, whether or not it was newly added
                .tx_ping
                .send(rebuild)
                .unwrap();
        }
    }
}
