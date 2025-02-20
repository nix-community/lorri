//! Uses `builder` and filesystem watch code to repeatedly
//! evaluate and build a given Nix file.

use crate::builder::{self, BuildError, RootedPath};
use crate::cas::ContentAddressable;
use crate::daemon::LoopHandlerEvent;
use crate::nix::options::NixOptions;
use crate::pathreduction::reduce_paths;
use crate::project::{self, Project};
use crate::watch::{Watch, WatchPathBuf};
use crate::NixFile;
use anyhow::anyhow;
use slog::debug;
use std::future::pending;
use std::path::PathBuf;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::task::JoinHandle;

/// Build events that can happen.
/// Abstracting over its internal to make different serialize instances possible.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// A build has started
    Started {
        /// The shell.nix file for the building project
        nix_file: NixFile,
        /// The reason the build started
        reason: Reason,
    },
    /// A build completed successfully
    Completed {
        /// The shell.nix file for the building project
        nix_file: NixFile,
        /// the output paths of the build
        rooted_output_paths: builder::OutputPath,
    },
    /// A build command returned a failing exit status
    Failure {
        /// The shell.nix file for the building project
        nix_file: NixFile,
        /// The error that exited the build
        failure: BuildError,
    },
}
/// Build events that can happen.
/// Abstracting over its internal to make different serialize instances possible.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSnapshot {
    /// the snapshot of events
    pub snapshot: Vec<Event>,
}

/// Description of the project change that triggered a build.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reason {
    /// We received a ping, requesting us to reevaluate and maybe build the project
    PingReceived,
    /// When there is a filesystem change, the first changed file is recorded,
    /// along with a count of other filesystem events.
    FilesChanged(Vec<PathBuf>),
}

type BuildResult = Result<builder::RunResult, BuildError>;

/// The BuildLoop repeatedly builds the Nix expression in
/// `project` each time a source file influencing
/// a previous build changes.
/// If a build is ongoing, it will finish the build first.
/// If there was intermediate requests for new builds, it will schedule a build to be run right after.
/// Additionally, we create GC roots for the build results.
pub struct BuildLoop {
    /// Watches all input files for changes.
    /// As new input files are discovered, they are added to the watchlist.
    watch: Watch,
    dat: BuildLoopDat,
}

/// Clone-able part of the build-loop struct
#[derive(Clone)]
pub struct BuildLoopDat {
    /// Project to be built.
    project: Project,
    /// Extra options to pass to each nix invocation
    extra_nix_options: NixOptions,
    /// Content addressable store to save static files in.
    cas: ContentAddressable,
    logger: slog::Logger,
}

impl BuildLoop {
    /// Instatiate a new BuildLoop. Uses an internal filesystem
    /// watching implementation.
    ///
    /// Will start by only watching the project’s nix file,
    /// and then add new files after each nix run.
    pub fn new(
        project: Project,
        extra_nix_options: NixOptions,
        cas: ContentAddressable,
        logger: slog::Logger,
    ) -> anyhow::Result<BuildLoop> {
        let mut watch = Watch::try_new(&logger).map_err(|err| anyhow!(err))?;
        watch.filter.add_to_watch(vec![WatchPathBuf::Normal(
            project.file.as_absolute_path().to_owned(),
        )]);

        Ok(BuildLoop {
            dat: BuildLoopDat {
                project,
                extra_nix_options,
                logger,
                cas,
            },
            watch,
        })
    }

    /// Loop forever, watching the filesystem for changes. Blocks.
    /// Sends `Event`s over `Self.tx` once they happen.
    /// When new filesystem changes are detected while a build is
    /// still running, it is finished first before starting a new build.
    pub async fn forever(
        mut self,
        tx_events: UnboundedSender<LoopHandlerEvent>,
        mut rx_ping: Receiver<()>,
    ) -> ! {
        struct BuildFuture {
            is_building: bool,
            join_hdl: JoinHandle<BuildResult>,
        }
        let mk_not_building = || BuildFuture {
            is_building: false,
            join_hdl: tokio::spawn(pending()),
        };
        let mk_is_building = |dat| BuildFuture {
            is_building: true,
            join_hdl: BuildLoop::start_build(dat),
        };

        // currently running build, if any. This is set/read each recv loop.
        let mut current_build: BuildFuture = mk_not_building();
        // Whether we should start another build after finishing the current one.
        let mut scheduled: Option<()> = None;

        // Helper so we can pull things out of the channel select macro
        enum Msg {
            BuildResult(BuildResult),
            Changed(Vec<PathBuf>),
            Pinged,
        }

        loop {
            debug!(self.dat.logger, "looping build_loop";
               "current_build" => match (current_build.is_building, scheduled.is_some()) {
                 (true, true) => "running and scheduled",
                 (false, false) => "not running, nothing scheduled",
                 (false, true) => "not running, scheduled",
                 (true, false) => "running, nothing scheduled"
                },
               "project" => &self.dat.project.file);

            let send_event = |msg| {
                tx_events
                    .send(LoopHandlerEvent::BuildEvent(msg))
                    .expect("Failed to send an event")
            };

            let res = tokio::select! {
                // build finished
                msg = &mut current_build.join_hdl => Msg::BuildResult(msg.expect("build thread panicked")),
                // watcher found file change
                msg = self.watch.watch_events_rx.recv() => match msg {
                    None => continue,
                    Some(changed) => Msg::Changed(changed),
                },

                // we were pinged
                msg = rx_ping.recv() => match msg {
                    Some(()) => Msg::Pinged,
                    None => continue,
                }
            };

            match res {
                Msg::BuildResult(run_result) => {
                    // if there’s another build scheduled, start it.
                    if let Some(()) = scheduled.take() {
                        current_build = mk_is_building(self.dat.clone())
                    } else {
                        current_build = mk_not_building()
                    }

                    match self.handle_run_result(run_result) {
                        Ok(rooted_output_paths) => {
                            send_event(Event::Completed {
                                nix_file: self.dat.project.file.as_nix_file().clone(),
                                rooted_output_paths,
                            });
                        }
                        Err(e) => {
                            if e.is_actionable() {
                                send_event(Event::Failure {
                                    nix_file: self.dat.project.file.as_nix_file().clone(),
                                    failure: e,
                                })
                            } else {
                                panic!("Unrecoverable error:\n{:#?}", e);
                            }
                        }
                    }
                }
                Msg::Changed(changed) => {
                    // TODO: this is not a started, this is just a scheduled!
                    send_event(Event::Started {
                        nix_file: self.dat.project.file.as_nix_file().clone(),
                        reason: Reason::FilesChanged(changed),
                    });
                    // start a build, or if one is already running, schedule it.
                    if current_build.is_building {
                        scheduled = Some(())
                    } else {
                        current_build = mk_is_building(self.dat.clone())
                    }
                }
                Msg::Pinged => {
                    // TODO: this is not a started, this is just a scheduled!
                    send_event(Event::Started {
                        nix_file: self.dat.project.file.as_nix_file().clone(),
                        reason: Reason::PingReceived,
                    });
                    // start a build, or if one is already running, schedule it.
                    if current_build.is_building {
                        scheduled = Some(())
                    } else {
                        current_build = mk_is_building(self.dat.clone())
                    }
                }
            }
        }
    }

    /// Start an actual build, asynchronously.
    fn start_build(dat: BuildLoopDat) -> JoinHandle<Result<builder::RunResult, BuildError>> {
        match &dat.project.file {
            project::ProjectFile::ShellNix(nf) => {
                let nix_file = nf.clone();
                let cas = dat.cas.clone();
                let extra_nix_options = dat.extra_nix_options.clone();
                let logger2 = dat.logger.clone();
                tokio::task::spawn(async move {
                    builder::instantiate_and_build(&nix_file, &cas, &extra_nix_options, &logger2)
                        .await
                })
            }
            project::ProjectFile::FlakeNix(i) => {
                let logger = dat.logger.clone();
                let installable = i.clone();
                tokio::task::spawn(async move { builder::flake(&installable, &logger).await })
            }
        }
    }

    /// Execute a single build of the environment.
    ///
    /// This will create GC roots and expand the file watch list for
    /// the evaluation.
    pub async fn once(mut self) -> Result<builder::OutputPath, BuildError> {
        let run_result = BuildLoop::start_build(self.dat.clone())
            .await
            .expect("build panicked");
        let res = self.handle_run_result(run_result);
        self.watch.stop_nonblocking();
        res
    }

    fn handle_run_result(
        &mut self,
        run_result: Result<builder::RunResult, BuildError>,
    ) -> Result<builder::OutputPath, BuildError> {
        let run_result = run_result?;
        self.register_paths(&run_result.referenced_paths)?;
        self.root_result(run_result.result)
    }

    fn register_paths(&mut self, paths: &[WatchPathBuf]) -> Result<(), notify::Error> {
        let original_paths_len = paths.len();
        let paths = reduce_paths(paths);
        debug!(self.dat.logger, "paths reduced"; "from" => original_paths_len, "to" => paths.len());

        // add all new (reduced) nix sources to the input source watchlist
        self.watch
            .filter
            .add_to_watch(paths.into_iter().collect::<Vec<_>>());

        Ok(())
    }

    fn root_result(&mut self, build: RootedPath) -> Result<builder::OutputPath, BuildError> {
        self.dat.project.create_roots(build).map_err(BuildError::io)
    }
}
