//! Uses `builder` and filesystem watch code to repeatedly
//! evaluate and build a given Nix file.

use crate::builder;
use crate::daemon::LoopHandlerEvent;
use crate::error::BuildError;
use crate::nix::options::NixOptions;
use crate::pathreduction::reduce_paths;
use crate::project::roots;
use crate::project::roots::Roots;
use crate::project::Project;
use crate::watch::Watch;
use crate::NixFile;
use crossbeam_channel as chan;
use slog_scope::debug;
use std::path::PathBuf;

/// Builder events sent back over `BuildLoop.tx`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
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
        rooted_output_paths: builder::OutputPaths<roots::RootPath>,
    },
    /// A build command returned a failing exit status
    Failure {
        /// The shell.nix file for the building project
        nix_file: NixFile,
        /// The error that exited the build
        failure: BuildError,
    },
}

/// Description of the project change that triggered a build.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// When a project is presented to Lorri to track, it's built for this reason.
    ProjectAdded(NixFile),
    /// When a ping is received.
    PingReceived,
    /// When there is a filesystem change, the first changed file is recorded,
    /// along with a count of other filesystem events.
    FilesChanged(Vec<PathBuf>),
}

/// The BuildLoop repeatedly builds the Nix expression in
/// `project` each time a source file influencing
/// a previous build changes.
/// Additionally, we create GC roots for the build results.
pub struct BuildLoop<'a> {
    /// Project to be built.
    project: &'a Project,
    /// Watches all input files for changes.
    /// As new input files are discovered, they are added to the watchlist.
    watch: Watch,
    /// Extra options to pass to each nix invocation
    extra_nix_options: NixOptions,
}

impl<'a> BuildLoop<'a> {
    /// Instatiate a new BuildLoop. Uses an internal filesystem
    /// watching implementation.
    pub fn new(project: &'a Project, extra_nix_options: NixOptions) -> BuildLoop<'a> {
        BuildLoop {
            project,
            watch: Watch::try_new().expect("Failed to initialize watch"),
            extra_nix_options,
        }
    }

    /// Loop forever, watching the filesystem for changes. Blocks.
    /// Sends `Event`s over `Self.tx` once they happen.
    /// When new filesystem changes are detected while a build is
    /// still running, it is finished first before starting a new build.
    #[allow(clippy::drop_copy, clippy::zero_ptr)] // triggered by `select!`
    pub fn forever(&mut self, tx: chan::Sender<LoopHandlerEvent>, rx_ping: chan::Receiver<()>) {
        // The project has just been added, so run the builder in the first iteration
        let mut output_paths = self.once_with_send(
            &tx,
            Event::Started {
                nix_file: self.project.nix_file.clone(),
                reason: Reason::ProjectAdded(self.project.nix_file.clone()),
            },
        );

        // Drain pings initially: we're going to trigger a first build anyway
        rx_ping.try_iter().for_each(drop);

        let rx_notify = self.watch.rx.clone();

        loop {
            let reason = chan::select! {
                recv(rx_notify) -> msg => match msg {
                    Ok(msg) => {
                        match self.watch.process(msg) {
                            Some(changed) => {
                                Some(Event::Started{
                                    nix_file: self.project.nix_file.clone(),
                                    reason: Reason::FilesChanged(changed)
                                })
                            },
                            // No relevant file events
                            None => None
                        }
                    }
                    Err(chan::RecvError) => {
                        debug!("notify chan was disconnected"; "project" => &self.project.nix_file);
                        None
                    }
                },
                recv(rx_ping) -> msg => match (msg, &output_paths) {
                    (Ok(()), Some(output_paths)) => {
                        debug!("pinged"; "project" => &self.project.nix_file);
                        // TODO: why is this check done here?
                        if !output_paths.shell_gc_root_is_dir() {
                            Some(Event::Started{
                                nix_file: self.project.nix_file.clone(),
                                reason: Reason::PingReceived
                            })
                        }
                        else { None }
                    },
                    // TODO: can we just ignore this case?
                    (Ok(()), None) => None,
                    (Err(chan::RecvError), _) => {
                        debug!("ping chan was disconnected"; "project" => &self.project.nix_file);
                        None
                    }
                }
            };

            // If there is some reason to build, run the build!
            if let Some(rsn) = reason {
                output_paths = self.once_with_send(&tx, rsn)
            }
        }
    }

    fn once_with_send(
        &mut self,
        tx: &chan::Sender<LoopHandlerEvent>,
        reason: Event,
    ) -> Option<builder::OutputPaths<roots::RootPath>> {
        let send = |msg| tx.send(msg).expect("Failed to send an event");
        send(LoopHandlerEvent::BuildEvent(reason));
        match self.once() {
            Ok(rooted_output_paths) => {
                send(LoopHandlerEvent::BuildEvent(Event::Completed {
                    nix_file: self.project.nix_file.clone(),
                    rooted_output_paths: rooted_output_paths.clone(),
                }));
                Some(rooted_output_paths)
            }
            Err(e) => {
                if e.is_actionable() {
                    send(LoopHandlerEvent::BuildEvent(Event::Failure {
                        nix_file: self.project.nix_file.clone(),
                        failure: e,
                    }))
                } else {
                    panic!("Unrecoverable error:\n{:#?}", e);
                }
                None
            }
        }
    }

    /// Execute a single build of the environment.
    ///
    /// This will create GC roots and expand the file watch list for
    /// the evaluation.
    pub fn once(&mut self) -> Result<builder::OutputPaths<roots::RootPath>, BuildError> {
        let run_result = builder::run(
            &self.project.nix_file,
            &self.project.cas,
            &self.extra_nix_options,
        )?;
        self.register_paths(&run_result.referenced_paths)?;
        self.root_result(run_result.result)
    }

    fn register_paths(&mut self, paths: &[PathBuf]) -> Result<(), notify::Error> {
        let original_paths_len = paths.len();
        let paths = reduce_paths(&paths);
        debug!("paths reduced"; "from" => original_paths_len, "to" => paths.len());

        // add all new (reduced) nix sources to the input source watchlist
        self.watch.extend(paths.into_iter().collect::<Vec<_>>())?;

        Ok(())
    }

    fn root_result(
        &mut self,
        build: builder::RootedPath,
    ) -> Result<builder::OutputPaths<roots::RootPath>, BuildError> {
        let roots = Roots::from_project(&self.project);
        roots.create_roots(build).map_err(BuildError::io)
    }
}
