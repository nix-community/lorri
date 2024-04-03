//! Uses `builder` and filesystem watch code to repeatedly
//! evaluate and build a given Nix file.

use crate::builder::{self, BuildError};
use crate::daemon::LoopHandlerEvent;
use crate::nix::options::NixOptions;
use crate::pathreduction::reduce_paths;
use crate::project::{self, Project};
use crate::run_async::Async;
use crate::watch::{Watch, WatchPathBuf};
use crate::NixFile;
use anyhow::{anyhow, Context};
use crossbeam_channel as chan;
use slog::debug;
use std::path::PathBuf;

/// Build events that can happen.
/// Abstracting over its internal to make different serialize instances possible.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventI<NixFile, Reason, OutputPath, BuildError> {
    /// Demarks a stream of events from recent history becoming live
    SectionEnd,
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
        rooted_output_paths: OutputPath,
    },
    /// A build command returned a failing exit status
    Failure {
        /// The shell.nix file for the building project
        nix_file: NixFile,
        /// The error that exited the build
        failure: BuildError,
    },
}

/// Builder events sent back over `BuildLoop.tx`.
pub type Event = EventI<NixFile, Reason, builder::OutputPath<project::RootPath>, BuildError>;

impl<NixFile, Reason, OutputPath, BuildError> EventI<NixFile, Reason, OutputPath, BuildError> {
    /// Map over the inner types.
    pub fn map<F, G, H, I, NixFile2, Reason2, OutputPaths2, BuildError2>(
        self,
        nix_file_f: F,
        reason_f: G,
        output_paths_f: H,
        build_error_f: I,
    ) -> EventI<NixFile2, Reason2, OutputPaths2, BuildError2>
    where
        F: Fn(NixFile) -> NixFile2,
        G: Fn(Reason) -> Reason2,
        H: Fn(OutputPath) -> OutputPaths2,
        I: Fn(BuildError) -> BuildError2,
    {
        use EventI::*;
        match self {
            SectionEnd => SectionEnd,
            Started { nix_file, reason } => Started {
                nix_file: nix_file_f(nix_file),
                reason: reason_f(reason),
            },
            Completed {
                nix_file,
                rooted_output_paths,
            } => Completed {
                nix_file: nix_file_f(nix_file),
                rooted_output_paths: output_paths_f(rooted_output_paths),
            },
            Failure { nix_file, failure } => Failure {
                nix_file: nix_file_f(nix_file),
                failure: build_error_f(failure),
            },
        }
    }
}

/// Description of the project change that triggered a build.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReasonI<NixFile> {
    /// When a project is presented to Lorri to track, it's built for this reason.
    ProjectAdded(NixFile),
    /// When a ping is received.
    PingReceived,
    /// When there is a filesystem change, the first changed file is recorded,
    /// along with a count of other filesystem events.
    FilesChanged(Vec<PathBuf>),
}

impl<NixFile> ReasonI<NixFile> {
    /// Map over the inner types.
    pub fn map<F, NixFile2>(self, nix_file_f: F) -> ReasonI<NixFile2>
    where
        F: Fn(NixFile) -> NixFile2,
    {
        use ReasonI::*;
        match self {
            ProjectAdded(nix_file) => ProjectAdded(nix_file_f(nix_file)),
            PingReceived => PingReceived,
            FilesChanged(vec) => FilesChanged(vec),
        }
    }
}

type Reason = ReasonI<NixFile>;

/// The BuildLoop repeatedly builds the Nix expression in
/// `project` each time a source file influencing
/// a previous build changes.
/// If a build is ongoing, it will finish the build first.
/// If there was intermediate requests for new builds, it will schedule a build to be run right after.
/// Additionally, we create GC roots for the build results.
pub struct BuildLoop<'a> {
    /// Project to be built.
    project: &'a Project,
    /// Extra options to pass to each nix invocation
    extra_nix_options: NixOptions,
    /// Watches all input files for changes.
    /// As new input files are discovered, they are added to the watchlist.
    watch: Watch,
    user: project::Username,
    logger: slog::Logger,
}

enum BuildState {
    /// No build is currently running.
    NotRunning,
    /// A build is running.
    Running(Async<BuildResult>),
    /// A build is running and another build is scheduled to run immediately after it finishes.
    RunningAndScheduled(Async<BuildResult>),
}
type BuildResult = Result<builder::RunResult, BuildError>;

impl BuildState {
    fn result_chan(&self) -> chan::Receiver<BuildResult> {
        match self {
            Self::NotRunning => chan::never(),
            Self::Running(build) => build.chan(),
            Self::RunningAndScheduled(build) => build.chan(),
        }
    }

    fn display_status(&self) -> &str {
        match self {
            Self::NotRunning => "not running",
            Self::Running(_) => "running",
            Self::RunningAndScheduled(_) => "running and scheduled",
        }
    }
}

impl<'a> BuildLoop<'a> {
    /// Instatiate a new BuildLoop. Uses an internal filesystem
    /// watching implementation.
    ///
    /// Will start by only watching the project’s nix file,
    /// and then add new files after each nix run.
    pub fn new(
        project: &'a Project,
        extra_nix_options: NixOptions,
        user: project::Username,
        logger: slog::Logger,
    ) -> anyhow::Result<BuildLoop<'a>> {
        let mut watch = Watch::try_new(logger.clone()).map_err(|err| anyhow!(err))?;
        watch
            .extend(vec![WatchPathBuf::Normal(
                project.nix_file.as_absolute_path().to_owned(),
            )])
            .with_context(|| {
                format!(
                    "Failed to add nix path to watcher for nix file {}",
                    project.nix_file.display()
                )
            })?;

        Ok(BuildLoop {
            project,
            extra_nix_options,
            watch,
            user,
            logger,
        })
    }

    /// Loop forever, watching the filesystem for changes. Blocks.
    /// Sends `Event`s over `Self.tx` once they happen.
    /// When new filesystem changes are detected while a build is
    /// still running, it is finished first before starting a new build.
    pub fn forever(
        &mut self,
        tx_events: chan::Sender<LoopHandlerEvent>,
        rx_ping: chan::Receiver<()>,
    ) -> crate::Never {
        let mut current_build = BuildState::NotRunning;
        let rx_watcher = self.watch.rx.clone();

        loop {
            debug!(self.logger, "looping build_loop";
                   "current_build" => current_build.display_status(),
                   "project" => &self.project.nix_file);
            let rx_current_build = current_build.result_chan();

            let send_event = |msg| {
                tx_events
                    .send(LoopHandlerEvent::BuildEvent(msg))
                    .expect("Failed to send an event")
            };

            chan::select! {

                // build finished
                recv(rx_current_build) -> msg => match msg {
                    Ok(run_result) => {
                        self.start_another_build_or_stop(&mut current_build);

                        match self.handle_run_result(run_result) {
                            Ok(rooted_output_paths) => {
                                send_event(Event::Completed {
                                    nix_file: self.project.nix_file.clone(),
                                    rooted_output_paths,
                                });
                            }
                            Err(e) => {
                                if e.is_actionable() {
                                    send_event(Event::Failure {
                                        nix_file: self.project.nix_file.clone(),
                                        failure: e,
                                    })
                                } else {
                                    panic!("Unrecoverable error:\n{:#?}", e);
                                }
                            }
                        }
                    },
                    Err(chan::RecvError) =>
                        debug!(self.logger, "current build async chan was disconnected"; "project" => &self.project.nix_file)
                },

                // watcher found file change
                recv(rx_watcher) -> msg => match msg {
                    Ok(msg) => {
                        match self.watch.process(msg) {
                            Some(changed) => {
                                // TODO: this is not a started, this is just a scheduled!
                                send_event(Event::Started {
                                    nix_file: self.project.nix_file.clone(),
                                    reason: Reason::FilesChanged(changed)
                                });
                                self.start_or_schedule_build(&mut current_build)
                            },
                            // No relevant file events
                            None => {}
                        }
                    },
                    Err(chan::RecvError) =>
                        debug!(self.logger, "notify chan was disconnected"; "project" => &self.project.nix_file)
                },

                // we were pinged
                recv(rx_ping) -> msg => match msg {
                    Ok(()) => {
                        // TODO: this is not a started, this is just a scheduled!
                        send_event(Event::Started{
                            nix_file: self.project.nix_file.clone(),
                            reason: Reason::PingReceived
                        });
                        self.start_or_schedule_build(&mut current_build)
                    },
                    Err(chan::RecvError) =>
                        debug!(self.logger, "ping chan was disconnected"; "project" => &self.project.nix_file)
                }
            };
        }
    }

    /// Schedule a build to be run as soon as possible; immediately start the build if we are `NotRunning`.
    fn start_or_schedule_build(&self, current_build: &mut BuildState) {
        *current_build = match std::mem::replace(current_build, BuildState::NotRunning) {
            BuildState::NotRunning => BuildState::Running(self.start_build()),
            BuildState::Running(build) => BuildState::RunningAndScheduled(build),
            BuildState::RunningAndScheduled(build) => BuildState::RunningAndScheduled(build),
        }
    }

    /// If another build was scheduled, start it, else stop building.
    fn start_another_build_or_stop(&self, current_build: &mut BuildState) {
        *current_build = match std::mem::replace(current_build, BuildState::NotRunning) {
            BuildState::NotRunning => BuildState::NotRunning,
            BuildState::Running(_) => BuildState::NotRunning,
            BuildState::RunningAndScheduled(_) => BuildState::Running(self.start_build()),
        }
    }

    /// Start an actual build, asynchronously.
    fn start_build(&self) -> Async<Result<builder::RunResult, BuildError>> {
        let nix_file = self.project.nix_file.clone();
        let cas = self.project.cas.clone();
        let extra_nix_options = self.extra_nix_options.clone();
        let logger2 = self.logger.clone();
        crate::run_async::Async::run(&self.logger, move || {
            builder::run(&nix_file, &cas, &extra_nix_options, &logger2)
        })
    }

    /// Execute a single build of the environment.
    ///
    /// This will create GC roots and expand the file watch list for
    /// the evaluation.
    pub fn once(&mut self) -> Result<builder::OutputPath<project::RootPath>, BuildError> {
        let nix_file = self.project.nix_file.clone();
        let cas = self.project.cas.clone();
        let extra_nix_options = self.extra_nix_options.clone();
        let logger2 = self.logger.clone();
        self.handle_run_result(
            crate::run_async::Async::run(&self.logger, move || {
                builder::run(&nix_file, &cas, &extra_nix_options, &logger2)
            })
            .block(),
        )
    }

    fn handle_run_result(
        &mut self,
        run_result: Result<builder::RunResult, BuildError>,
    ) -> Result<builder::OutputPath<project::RootPath>, BuildError> {
        let run_result = run_result?;
        self.register_paths(&run_result.referenced_paths)?;
        self.root_result(run_result.result)
    }

    fn register_paths(&mut self, paths: &[WatchPathBuf]) -> Result<(), notify::Error> {
        let original_paths_len = paths.len();
        let paths = reduce_paths(paths);
        debug!(self.logger, "paths reduced"; "from" => original_paths_len, "to" => paths.len());

        // add all new (reduced) nix sources to the input source watchlist
        self.watch.extend(paths.into_iter().collect::<Vec<_>>())?;

        Ok(())
    }

    fn root_result(
        &mut self,
        build: builder::RootedPath,
    ) -> Result<builder::OutputPath<project::RootPath>, BuildError> {
        self.project
            .create_roots(build, self.user.clone(), &self.logger.clone())
            .map_err(BuildError::io)
    }
}
