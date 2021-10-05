//! Ops are command-line callables.

mod direnv;
pub mod error;

use crate::build_loop::BuildLoop;
use crate::build_loop::{Event, EventI, ReasonI};
use crate::builder;
use crate::builder::OutputPath;
use crate::cas::ContentAddressable;
use crate::changelog;
use crate::cli;
use crate::cli::ShellOptions;
use crate::cli::StartUserShellOptions_;
use crate::cli::WatchOptions;
use crate::daemon::client;
use crate::daemon::Daemon;
use crate::nix;
use crate::nix::options::NixOptions;
use crate::nix::CallOpts;
use crate::ops::direnv::{DirenvVersion, MIN_DIRENV_VERSION};
use crate::ops::error::{ok, ExitAs, ExitError, ExitErrorType, OpResult};
use crate::project::{roots::Roots, Project};
use crate::run_async::Async;
use crate::socket::path::SocketPath;
use crate::NixFile;
use crate::VERSION_BUILD_REV;

use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use std::{env, fs, io, thread};

use crossbeam_channel as chan;

use slog::{debug, info, warn};
use thiserror::Error;

/// Set up necessary directories or fail.
pub fn get_paths() -> Result<crate::constants::Paths, error::ExitError> {
    crate::constants::Paths::initialize().map_err(|e| {
        error::ExitError::user_error(
            anyhow::Error::new(e).context("Cannot initialize the lorri paths"),
        )
    })
}

/// Run a BuildLoop for `shell.nix`, watching for input file changes.
/// Can be used together with `direnv`.

/// See the documentation for lorri::cli::Command::Daemon for details.
pub fn daemon(opts: crate::cli::DaemonOptions, logger: &slog::Logger) -> OpResult {
    let extra_nix_options = match opts.extra_nix_options {
        None => NixOptions::empty(),
        Some(v) => NixOptions {
            builders: v.builders,
            substituters: v.substituters,
        },
    };

    let (mut daemon, build_rx) = Daemon::new(extra_nix_options);
    let logger2 = logger.clone();
    let build_handle = std::thread::spawn(move || {
        for msg in build_rx {
            info!(logger2, "build status"; "message" => ?msg);
        }
    });
    info!(logger, "ready");

    let paths = crate::ops::get_paths()?;
    daemon.serve(
        &SocketPath::from(paths.daemon_socket_file().clone()),
        paths.gc_root_dir(),
        paths.cas_store().clone(),
        &logger,
    )?;
    build_handle
        .join()
        .expect("failed to join build status thread");
    ok()
}

/// Emit shell script intended to be evaluated as part of direnv's .envrc
///
/// See the documentation for lorri::cli::Command::Direnv for more
/// details.
pub fn direnv<W: std::io::Write>(
    project: Project,
    mut shell_output: W,
    logger: &slog::Logger,
) -> OpResult {
    check_direnv_version()?;

    let root_paths = Roots::from_project(&project).paths();
    let paths_are_cached: bool = root_paths.all_exist();

    let ping_sent = {
        let address = crate::ops::get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_absolute_path().display());
        client::create::<client::Ping>(client::Timeout::from_millis(500), logger)
            .and_then(|c| {
                c.write(&client::Ping {
                    nix_file: project.nix_file,
                    rebuild: client::Rebuild::OnlyIfNotYetWatching,
                })?;
                Ok(())
            })
            // TODO: maybe ping should indeed return something so we can at least check whether it parses the message and the version is right. Right now this collapses all of that into a bool …
            .is_ok()
    };

    match (ping_sent, paths_are_cached) {
        (true, true) => {}

        // Ping sent & paths aren't cached: once the environment is created
        // the direnv environment will be updated automatically.
        (true, false) =>
            info!(
                logger,
                "lorri has not completed an evaluation for this project yet"
            ),

        // Ping not sent and paths are cached: we can load a stale environment
        // When the daemon is started, we'll send a fresh ping.
        (false, true) =>
            info!(
                logger,
                "lorri daemon is not running, loading a cached environment"
            ),

        // Ping not sent and paths are not cached: we can't load anything,
        // but when the daemon in started we'll send a ping and eventually
        // load a fresh environment.
        (false, false) =>
            warn!(logger, "lorri daemon is not running and this project has not yet been evaluated, please run `lorri daemon`"),
    }

    // direnv interprets stdout as a script that it evaluates. That is why (1) the logger for
    // `lorri direnv` outputs to stderr by default (to avoid corrupting the script) and (2) we
    // can't use the stderr logger here.
    // In production code, `shell_output` will be stdout so direnv can interpret the output.
    // `shell_output` is an argument so that testing code can inject a different `std::io::Write`
    // in order to inspect the output.
    writeln!(
        shell_output,
        r#"
EVALUATION_ROOT="{}"

watch_file "{}"
watch_file "$EVALUATION_ROOT"

{}"#,
        root_paths.shell_gc_root.display(),
        crate::ops::get_paths()?
            .daemon_socket_file()
            .as_absolute_path()
            .to_str()
            .expect("Socket path is not UTF-8 clean!"),
        include_str!("./ops/direnv/envrc.bash")
    )
    .expect("failed to write shell output");

    // direnv provides us with an environment variable if we are inside of its envrc execution.
    // Thus we can show a warning if the user runs it on their command line.
    if std::env::var("DIRENV_IN_ENVRC") != Ok(String::from("1")) {
        warn!(logger, "`lorri direnv` should be executed by direnv from within an `.envrc` file. Run `lorri init` to get started.")
    }

    ok()
}

/// Checks `direnv version` against the minimal version lorri requires.
fn check_direnv_version() -> OpResult {
    let out = with_command("direnv", |mut cmd| cmd.arg("version").output())?;
    let version = std::str::from_utf8(&out.stdout)
        .map_err(|_| ())
        .and_then(|utf| utf.trim_end().parse::<DirenvVersion>())
        .map_err(|()| {
            ExitError::environment_problem(anyhow::anyhow!(
                "Could not figure out the current `direnv` version (parse error)"
            ))
        })?;
    if version < MIN_DIRENV_VERSION {
        Err(ExitError::environment_problem(anyhow::anyhow!(
            "`direnv` is version {}, but >= {} is required for lorri to function",
            version,
            MIN_DIRENV_VERSION
        )))
    } else {
        ok()
    }
}

/// constructs a `Command` out of `executable`
/// Recognizes the case in which the executable is missing,
/// and converts it to a corresponding `ExitError`.
fn with_command<T, F>(executable: &str, cmd: F) -> Result<T, ExitError>
where
    F: FnOnce(Command) -> std::io::Result<T>,
{
    let res = cmd(Command::new(executable));
    res.map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => {
            ExitError::missing_executable(anyhow::anyhow!("`{}`: executable not found", executable))
        }
        _ => ExitError::temporary(
            anyhow::Error::new(err).context(format!("Could not start `{}`", executable)),
        ),
    })
}

/// The info callable is for printing
///
/// See the documentation for lorri::cli::Command::Info for more
/// details.
pub fn info(project: Project) -> OpResult {
    let root_paths = Roots::from_project(&project).paths();
    let OutputPath { shell_gc_root } = &root_paths;
    if root_paths.all_exist() {
        println!(
            "GC roots exist, shell_gc_root: {}",
            shell_gc_root.0.display()
        );
    } else {
        println!("GC roots do not exist. Has the project been built with lorri yet?",);
    }
    ok()
}

/// Bootstrap a new lorri project
///
/// See the documentation for lorri::cli::Command::Init for
/// more details
pub fn init(default_shell: &str, default_envrc: &str, logger: &slog::Logger) -> OpResult {
    create_if_missing(
        Path::new("./shell.nix"),
        default_shell,
        "Make sure shell.nix is of a form that works with nix-shell.",
        logger,
    )
    .map_err(ExitError::user_error)?;

    create_if_missing(
        Path::new("./.envrc"),
        default_envrc,
        "Please add 'eval \"$(lorri direnv)\"' to .envrc to set up lorri support.",
        logger,
    )
    .map_err(ExitError::user_error)?;

    info!(logger, "done");
    ok()
}

fn create_if_missing(
    path: &Path,
    contents: &str,
    msg: &str,
    logger: &slog::Logger,
) -> Result<(), io::Error> {
    if path.exists() {
        info!(logger, "file already exists, skipping"; "path" => path.to_str(), "message" => msg);
        Ok(())
    } else {
        let mut f = File::create(path)?;
        f.write_all(contents.as_bytes())?;
        info!(logger, "wrote file"; "path" => path.to_str());
        Ok(())
    }
}

/// Run a BuildLoop for `shell.nix`, watching for input file changes.
///
/// Can be used together with `direnv`.
/// See the documentation for lorri::cli::Command::Ping_ for details.
pub fn ping(nix_file: NixFile, logger: &slog::Logger) -> OpResult {
    client::create(client::Timeout::from_millis(500), logger)?.write(&client::Ping {
        nix_file,
        rebuild: client::Rebuild::Always,
    })?;
    Ok(())
}

/// Open up a project shell
///
/// This is the entry point for the `lorri shell` command.
///
/// # Overview
///
/// `lorri shell` launches the user's shell with the project environment set up. "The user's shell"
/// here just means whatever binary $SHELL points to. Concretely we get the following process tree:
///
/// `lorri shell`
/// ├── builds the project environment if --cached is false
/// ├── writes a bash init script that loads the project environment
/// ├── SPAWNS bash with the init script as its `--rcfile`
/// │   └── EXECS `lorri internal start-user-shell`
/// │       ├── (*) performs shell-specific setup for $SHELL
/// │       └── EXECS into user shell $SHELL
/// │           └── interactive user shell
/// └── `lorri shell` terminates
///
/// This setup allows lorri to support almost any shell with minimal additional work. Only the step
/// marked (*) must be adjusted, and only in case we want to customize the shell, e.g. changing the
/// way the prompt looks.
pub fn shell(project: Project, opts: ShellOptions, logger: &slog::Logger) -> OpResult {
    let lorri = env::current_exe().expect("failed to determine lorri executable's path");
    let shell = env::var("SHELL").expect("lorri shell requires $SHELL to be set");
    let cached = cached_root(&project);
    let mut bash_cmd = bash_cmd(
        if opts.cached {
            cached?
        } else {
            build_root(&project, cached.is_ok(), logger)?
        },
        &project.cas,
        logger,
    )?;

    debug!(logger, "bash_cmd : {:?}", bash_cmd);
    let status = bash_cmd
        .args(&[
            "-c",
            "exec \"$1\" internal start-user-shell --shell-path=\"$2\" --shell-file=\"$3\"",
            "--",
            &lorri
                .to_str()
                .expect("lorri executable path not UTF-8 clean"),
            &shell,
            project
                .nix_file
                .as_absolute_path()
                .to_str()
                .expect("Nix file path not UTF-8 clean"),
        ])
        .status()
        .expect("failed to execute bash");

    if !status.success() {
        Err(ExitError::panic(anyhow::anyhow!(
            "cannot run lorri shell: failed to execute internal shell command (error: {})",
            status
        )))
    } else {
        Ok(())
    }
}

fn build_root(
    project: &Project,
    cached: bool,
    logger: &slog::Logger,
) -> Result<PathBuf, ExitError> {
    let building = Arc::new(AtomicBool::new(true));
    let building_clone = building.clone();
    let logger2 = logger.clone();
    let progress_thread = Async::run(logger, move || {
        // Keep track of the start time to display a hint to the user that they can use `--cached`,
        // but only if a cached version of the environment exists
        let mut start = if cached { Some(Instant::now()) } else { None };

        eprint!("lorri: building environment");
        while building_clone.load(Ordering::SeqCst) {
            // Show `--cached` hint once after some time has passed
            if let Some(start_time) = start {
                if start_time.elapsed() >= Duration::from_millis(10_000) {
                    eprintln!(
                        "\nHint: you can use `lorri shell --cached` to use the most recent \
                         environment that was built successfully."
                    );
                    start = None; // Don't show the hint again
                }
            }
            thread::sleep(Duration::from_millis(500));

            // Indicate progress
            eprint!(".");
            io::stderr().flush().expect("couldn’t flush‽");
        }
        eprintln!(". done");
    });

    // TODO: add the ability to pass extra_nix_options to shell
    let run_result = builder::run(
        &project.nix_file,
        &project.cas,
        &crate::nix::options::NixOptions::empty(),
        &logger2,
    );
    building.store(false, Ordering::SeqCst);
    progress_thread.block();

    let run_result = run_result
        .map_err(|e| {
            if cached {
                ExitError::temporary(anyhow::anyhow!(
                    "Build failed. Hint: try running `lorri shell --cached` to use the most \
                     recent environment that was built successfully.\n\
                     Build error: {}",
                    e
                ))
            } else {
                ExitError::temporary(anyhow::anyhow!(
                    "Build failed. No cached environment available.\n\
                     Build error: {}",
                    e
                ))
            }
        })?
        .result;

    Ok(Roots::from_project(&project)
        .create_roots(run_result, &logger2)
        .map_err(|e| {
            ExitError::temporary(anyhow::Error::new(e).context("rooting the environment failed"))
        })?
        .shell_gc_root
        .0
        .as_absolute_path()
        .to_owned())
}

fn cached_root(project: &Project) -> Result<PathBuf, ExitError> {
    let root_paths = Roots::from_project(&project).paths();
    if !root_paths.all_exist() {
        Err(ExitError::temporary(anyhow::anyhow!(
            "project has not previously been built successfully",
        )))
    } else {
        Ok(root_paths.shell_gc_root.0.as_absolute_path().to_owned())
    }
}

/// Instantiates a `Command` to start bash.
pub fn bash_cmd(
    project_root: PathBuf,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> Result<Command, ExitError> {
    let init_file = cas
        .file_from_string(&format!(
            r#"
EVALUATION_ROOT="{}"

{}"#,
            project_root.display(),
            include_str!("./ops/direnv/envrc.bash")
        ))
        .expect("failed to write shell output");

    debug!(logger,"building bash via runtime closure"; "closure" => crate::RUN_TIME_CLOSURE);
    let bash_path = CallOpts::expression(&format!("(import {}).path", crate::RUN_TIME_CLOSURE))
        .value::<PathBuf>()
        .expect("failed to get runtime closure path");

    let mut cmd = Command::new(bash_path.join("bash"));
    cmd.env(
        "BASH_ENV",
        init_file
            .as_absolute_path()
            .to_str()
            .expect("script file path not UTF-8 clean"),
    );
    Ok(cmd)
}

/// Helper command to create a user shell
///
/// See the documentation for `crate::ops::shell`.
pub fn start_user_shell(project: Project, opts: StartUserShellOptions_) -> OpResult {
    // This temporary directory will not be cleaned up by lorri because we exec into the shell
    // process, which means that destructors will not be run. However, (1) the temporary files
    // lorri creates in this directory are only a few hundred bytes long; (2) the directory will be
    // cleaned up on reboot or whenever the OS decides to purge temporary directories.
    let tempdir = tempfile::tempdir().expect("failed to create temporary directory");
    let e = shell_cmd(opts.shell_path.as_ref(), &project.cas, tempdir.path()).exec();

    // 'exec' will never return on success, so if we get here, we know something has gone wrong.
    panic!("failed to exec into '{}': {}", opts.shell_path.display(), e);
}

fn shell_cmd(shell_path: &Path, cas: &ContentAddressable, tempdir: &Path) -> Command {
    let mut cmd = Command::new(shell_path);

    match shell_path
        .file_name()
        .expect("shell path must point to a file")
        .to_str()
        .expect("shell path is not UTF-8 clean")
    {
        "bash" => {
            // To override the prompt, we need to set PS1 *after* all other setup scripts have run.
            // That makes it necessary to create our own setup script to be passed via --rcfile.
            let rcfile = cas
                .file_from_string(
                    // Using --rcfile disables sourcing of default setup scripts, so we source them
                    // explicitly here.
                    r#"
[ -e /etc/bash.bashrc ] && . /etc/bash.bashrc
[ -e ~/.bashrc ] && . ~/.bashrc
PS1="(lorri) $PS1"
"#,
                )
                .expect("failed to write bash init script");
            cmd.args(&[
                "--rcfile",
                rcfile
                    .as_absolute_path()
                    .to_str()
                    .expect("file path not UTF-8 clean"),
            ]);
        }
        "zsh" => {
            // Zsh does not support anything like bash's --rcfile. However, zsh sources init
            // scripts from $ZDOTDIR by default. So we set $ZDOTDIR to a directory under lorri's
            // control, follow the default sourcing procedure, and then set the PS1.
            fs::write(
                tempdir.join(".zshrc"),
                // See "STARTUP/SHUTDOWN FILES" section of the zshall man page as well as
                // https://superuser.com/a/591440/318156.
                r#"
unset RCS # disable automatic sourcing of startup scripts

# reset ZDOTDIR
if [ ! -z ${ZDOTDIR_BEFORE} ]; then
    ZDOTDIR="${ZDOTDIR_BEFORE}"
else
    unset ZDOTDIR
fi

ZDOTDIR_OR_HOME="${ZDOTDIR:-${HOME}}"
test -f "$ZDOTDIR_OR_HOME/.zshenv" && . "$ZDOTDIR_OR_HOME/.zshenv"
test -f "/etc/zshrc"               && . "/etc/zshrc"
ZDOTDIR_OR_HOME="${ZDOTDIR:-${HOME}}"
test -f "$ZDOTDIR_OR_HOME/.zshrc"  && . "$ZDOTDIR_OR_HOME/.zshrc"

PS1="(lorri) ${PS1}"
"#,
            )
            .expect("failed to write zsh init script");
            if let Ok(d) = env::var("ZDOTDIR") {
                cmd.env("ZDOTDIR_BEFORE", d);
            }
            cmd.env("ZDOTDIR", tempdir);
        }
        // Add handling for other supported shells here.
        _ => {}
    }
    cmd
}

/// Options for the kinds of events to report
#[derive(Debug)]
pub enum EventKind {
    /// Report only live events - those that happen after invocation
    Live,
    /// Report events recorded for projects up until invocation
    Snapshot,
    /// Report all events
    All,
}

impl FromStr for EventKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(EventKind::All),
            "live" => Ok(EventKind::Live),
            "snapshot" => Ok(EventKind::Snapshot),
            _ => Err(format!("{} not in all,live,snapshot", s)),
        }
    }
}

// These types are just transparent newtype wrappers to implement a different serde class and JsonEncode

/// For now use the EventI structure, in the future we might want to split it off.
/// At least it will show us that we need to change something here if we change it
/// and it relates to this interface.
#[derive(Serialize)]
#[serde(transparent)]
struct StreamEvent(EventI<StreamNixFile, StreamReason, StreamOutputPath, StreamBuildError>);

/// Nix files are encoded as strings
#[derive(Serialize)]
#[serde(transparent)]
struct StreamNixFile(String);

/// Same here, the reason contains a nix file which has to be converted to a string.
#[derive(Serialize)]
#[serde(transparent)]
struct StreamReason(ReasonI<String>);

/// And same here, OutputPaths are GcRoots and have to be converted as well.
#[derive(Serialize)]
#[serde(transparent)]
struct StreamOutputPath(OutputPath<String>);

/// Just expose the error message for now.
#[derive(Serialize)]
struct StreamBuildError {
    message: String,
}

/// Run to output a stream of build events in a machine-parseable form.
///
/// See the documentation for lorri::cli::Command::StreamEvents_ for more
/// details.
pub fn stream_events(kind: EventKind, logger: &slog::Logger) -> OpResult {
    let (tx_event, rx_event) = chan::unbounded::<Event>();

    let thread = {
        let address = crate::ops::get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_absolute_path().display());
        let logger2 = logger.clone();
        // This async will not block when it is dropped,
        // since it only reads messages and don’t want to block exit in the Snapshot case.
        Async::<Result<(), ExitError>>::run_and_linger(logger, move || {
            let client = client::create::<client::StreamEvents>(
                // infinite timeout because we are listening indefinitely
                client::Timeout::Infinite,
                &logger2,
            )?;

            client.write(&client::StreamEvents {})?;
            loop {
                let res = client.read();
                tx_event
                    .send(
                        // TODO: error
                        res.map_err(|err| ExitError::temporary(anyhow::Error::new(err)))?,
                    )
                    .expect("tx_event hung up!");
            }
        })
    };

    let mut snapshot_done = false;
    loop {
        chan::select! {
            recv(rx_event) -> event => match event.expect("rx_event hung up!") {
                Event::SectionEnd => {
                    debug!(logger, "SectionEnd");
                    match kind {
                        // If we only want the snapshot, quit the program
                        EventKind::Snapshot => break Ok(()),
                        // Else we now start sending the incremental data
                        _ => { snapshot_done = true; },
                    }
                }
                ev => match (snapshot_done, &kind) {
                    (_, EventKind::All) | (false, EventKind::Snapshot) | (true, EventKind::Live) => {
                        fn nix_file_string(nix_file: NixFile) -> String {
                            nix_file.display().to_string()
                        }
                        serde_json::to_writer(
                            std::io::stdout(),
                            &StreamEvent(ev.map(
                                |nix_file| StreamNixFile(nix_file_string(nix_file)),
                                |reason| StreamReason(reason.map(nix_file_string)),
                                |output_path| {
                                    StreamOutputPath(output_path.map(|o| o.display().to_string()))
                                },
                                |build_error| StreamBuildError {
                                    message: format!("{}", build_error),
                                },
                            )),
                        )
                            .expect("couldn't serialize event");
                        write!(std::io::stdout(), "\n").expect("couldn't serialize event");
                        std::io::stdout().flush().expect("couldn't flush serialized event");
                    }
                    _ => (),
                },
            },
            recv(thread.chan()) -> finished => match finished.expect("send-events hung up!") {
                Ok(()) => panic!("send-events should never finish!"),
                // error in the async, time to quit
                err => err?
            }
        }
    }
}

/// The source to upgrade to.
enum UpgradeSource {
    /// A branch in the upstream git repo
    Branch(String),
    /// A local path
    Local(PathBuf),
}

#[derive(Error, Debug)]
enum UpgradeSourceError {
    /// The local path given by the user could not be found
    #[error("Cannot upgrade to local repostory {0}: path not found")]
    LocalPathNotFound(PathBuf),
    /// We couldn’t find local_path/release.nix, it is not a lorri repo.
    #[error("{0} does not exist, are you sure this is a lorri repository?")]
    ReleaseNixDoesntExist(PathBuf),
    /// An other error happened when canonicalizing the given path.
    #[error("Problem accessing local repository")]
    CantCanonicalizeLocalPath(#[source] std::io::Error),
}

impl ExitAs for UpgradeSourceError {
    fn exit_as(&self) -> ExitErrorType {
        use ExitErrorType::*;
        use UpgradeSourceError::*;
        match self {
            LocalPathNotFound(_) => UserError,
            CantCanonicalizeLocalPath(_) => Temporary,
            ReleaseNixDoesntExist(_) => UserError,
        }
    }
}

impl UpgradeSource {
    /// Convert from the cli argument to a form we can pass to ./upgrade.nix.
    fn from_cli_argument(upgrade_target: cli::UpgradeTo) -> Result<Self, UpgradeSourceError> {
        // if no source was given, we default to the rolling-release branch
        let src = upgrade_target
            .source
            .unwrap_or(cli::UpgradeSource::RollingRelease);
        Ok(match src {
            cli::UpgradeSource::RollingRelease => {
                UpgradeSource::Branch(String::from("rolling-release"))
            }
            cli::UpgradeSource::Master => UpgradeSource::Branch(String::from("canon")),
            cli::UpgradeSource::Canon => UpgradeSource::Branch(String::from("canon")),
            cli::UpgradeSource::Branch(b) => UpgradeSource::Branch(b.branch),
            cli::UpgradeSource::Local(dest) => {
                // make it absolute to not confuse ./upgrade.nix
                (match std::fs::canonicalize(dest.path.clone()) {
                    Ok(abspath) => {
                        // Check whether we actually have something like a lorri repository
                        let release_nix = abspath.join("release.nix");
                        if release_nix.exists() {
                            Ok(UpgradeSource::Local(abspath))
                        } else {
                            Err(UpgradeSourceError::ReleaseNixDoesntExist(release_nix))
                        }
                    }
                    Err(err) => Err(match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            UpgradeSourceError::LocalPathNotFound(dest.path)
                        }
                        _ => UpgradeSourceError::CantCanonicalizeLocalPath(err),
                    }),
                })?
            }
        })
    }
}

/// Upgrade lorri by using nix-env to install from Git.
///
/// This is useful for pointing users to an fix to a reported bug,
/// or for users who want to follow the lorri canon locally.
///
/// Originally it was used as pre-release, that’s why there is support
/// for updating to a special rolling-release branch.
pub fn upgrade(
    upgrade_target: cli::UpgradeTo,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> OpResult {
    /*
    1. nix-instantiate the expression
    2. get all the changelog entries from <currentnumber> to <maxnumber>
    3. nix-build the expression's package attribute
    4. nix-env -i the package
     */
    let upgrade_expr = cas
        .file_from_string(include_str!("./ops/upgrade.nix"))
        .expect("could not write to CAS");

    let expr = {
        let src = UpgradeSource::from_cli_argument(upgrade_target)?;

        match src {
            UpgradeSource::Branch(ref b) => println!("Upgrading from branch: {}", b),
            UpgradeSource::Local(ref p) => println!("Upgrading from local path: {}", p.display()),
        }

        let mut expr = nix::CallOpts::file(&upgrade_expr.as_absolute_path());

        match src {
            UpgradeSource::Branch(b) => {
                expr.argstr("type", "branch");
                expr.argstr("branch", b);
            }
            UpgradeSource::Local(p) => {
                expr.argstr("type", "local");
                expr.argstr("path", p);
            }
        }
        // ugly hack to prevent expr from being mutable outside,
        // since I can't sort out how to chain argstr and still
        // keep a reference
        expr
    };

    let changelog: changelog::Log = expr.clone().attribute("changelog").value().unwrap();

    println!("Changelog when upgrading from {}:", VERSION_BUILD_REV);
    for entry in changelog.entries.iter().rev() {
        if VERSION_BUILD_REV < entry.version {
            println!();
            println!("{}:", entry.version);
            for line in entry.changes.lines() {
                println!("    {}", line);
            }
        }
    }

    println!("Building ...");
    match expr.clone().attribute("package").path(logger) {
        Ok((build_result, gc_root)) => {
            let status = Command::new("nix-env")
                .arg("--install")
                .arg(build_result.as_path())
                .status()
                // TODO: check existence of commands at the beginning
                .expect("Error: failed to execute nix-env --install");
            // we can drop the temporary gc root
            drop(gc_root);

            if status.success() {
                info!(logger, "upgrade successful");
                ok()
            } else {
                Err(ExitError::expected_error(anyhow::anyhow!(
                    "\nError: nix-env command was not successful!\n{:#?}",
                    status
                )))
            }
        }
        // our update expression is broken, crash
        Err(e) => panic!("Failed to build the update! {:#?}", e),
    }
}

/// Run a BuildLoop for `shell.nix`, watching for input file changes.
/// Can be used together with `direnv`.
///
/// See the documentation for lorri::cli::Command::Shell for more
/// details.
pub fn watch(project: Project, opts: WatchOptions, logger: &slog::Logger) -> OpResult {
    if opts.once {
        main_run_once(project, logger)
    } else {
        main_run_forever(project, logger)
    }
}

fn main_run_once(project: Project, logger: &slog::Logger) -> OpResult {
    // TODO: add the ability to pass extra_nix_options to watch
    let mut build_loop = BuildLoop::new(&project, NixOptions::empty(), logger.clone())
        .map_err(ExitError::temporary)?;
    match build_loop.once() {
        Ok(msg) => {
            info!(logger, "build message"; "message" => ?msg);
            ok()
        }
        Err(e) => {
            if e.is_actionable() {
                // TODO: implement std::io::Error for BuildError to get a backtrace
                Err(ExitError::expected_error(anyhow::anyhow!("{:#?}", e)))
            } else {
                // TODO: implement std::io::Error for BuildError to get a backtrace
                Err(ExitError::temporary(anyhow::Error::msg(e)))
            }
        }
    }
}

fn main_run_forever(project: Project, logger: &slog::Logger) -> OpResult {
    let (tx_build_results, rx_build_results) = chan::unbounded();
    let (tx_ping, rx_ping) = chan::unbounded();
    let logger2 = logger.clone();
    // TODO: add the ability to pass extra_nix_options to watch
    let build_thread = {
        Async::run(logger, move || {
            match BuildLoop::new(&project, NixOptions::empty(), logger2) {
                Ok(mut bl) => bl.forever(tx_build_results, rx_ping).never(),
                Err(e) => Err(ExitError::temporary(e)),
            }
        })
    };

    // We ping the build loop once, to make it run the first build immediately
    tx_ping.send(()).expect("could not send ping to build_loop");

    for msg in rx_build_results {
        info!(logger, "build message"; "message" => ?msg);
    }

    build_thread.block()
}
