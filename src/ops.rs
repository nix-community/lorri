//! Ops are command-line callables.

mod direnv;
pub mod error;

use crate::build_loop::BuildLoop;
use crate::build_loop::{Event, EventI, ReasonI};
use crate::builder::OutputPath;
use crate::cas::ContentAddressable;
use crate::changelog;
use crate::cli;
use crate::cli::ShellOptions;
use crate::cli::StartUserShellOptions_;
use crate::cli::WatchOptions;
use crate::constants::Paths;
use crate::daemon::client;
use crate::daemon::Daemon;
use crate::nix;
use crate::nix::options::NixOptions;
use crate::nix::CallOpts;
use crate::ops::direnv::{DirenvVersion, MIN_DIRENV_VERSION};
use crate::ops::error::{ExitAs, ExitError, ExitErrorType};
use crate::project::{NixGcRootUserDir, Project};
use crate::run_async::Async;
use crate::socket::path::SocketPath;
use crate::NixFile;
use crate::VERSION_BUILD_REV;
use crate::{builder, project};

use std::ffi::OsStr;
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
use std::{collections::HashSet, fs::File, time::SystemTime};
use std::{env, fs, io, thread};
use std::{fmt::Debug, fs::remove_dir_all};

use anyhow::Context;
use crossbeam_channel as chan;

use slog::{debug, info, warn};
use thiserror::Error;

/// `./trivial-shell.nix`
pub const TRIVIAL_SHELL_SRC: &str = include_str!("./trivial-shell.nix");
/// `./default-envrc`
pub const DEFAULT_ENVRC: &str = include_str!("./default-envrc");

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
pub fn op_daemon(opts: crate::cli::DaemonOptions, logger: &slog::Logger) -> Result<(), ExitError> {
    let extra_nix_options = match opts.extra_nix_options {
        None => NixOptions::empty(),
        Some(v) => NixOptions {
            builders: v.builders,
            substituters: v.substituters,
        },
    };

    let username = project::Username::from_env_var().map_err(ExitError::environment_problem)?;
    let nix_gc_root_user_dir = project::NixGcRootUserDir::get_or_create(&username)?;

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
        nix_gc_root_user_dir,
        logger,
    )?;
    build_handle
        .join()
        .expect("failed to join build status thread");
    Ok(())
}

/// Emit shell script intended to be evaluated as part of direnv's .envrc
///
/// See the documentation for lorri::cli::Command::Direnv for more
/// details.
pub fn op_direnv<W: std::io::Write>(
    project: Project,
    mut shell_output: W,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    check_direnv_version()?;

    let root_paths = project.root_paths();
    let paths_are_cached: bool = root_paths.all_exist();

    let ping_sent = {
        let address = crate::ops::get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_path().display());
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
            .as_path()
            .to_str()
            .expect("Socket path is not UTF-8 clean!"),
        include_str!("./ops/direnv/envrc.bash")
    )
    .expect("failed to write shell output");

    // direnv provides us with an environment variable if we are inside of its envrc execution.
    // Thus we can show a warning if the user runs it on their command line.
    if std::env::var("DIRENV_IN_ENVRC") != Ok("1".to_string()) {
        warn!(logger, "`lorri direnv` should be executed by direnv from within an `.envrc` file. Run `lorri init` to get started.")
    }

    Ok(())
}

/// Checks `direnv version` against the minimal version lorri requires.
fn check_direnv_version() -> Result<(), ExitError> {
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
        Ok(())
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
pub fn op_info(paths: &Paths, project: Project) -> Result<(), ExitError> {
    let root_paths = project.root_paths();
    let OutputPath { shell_gc_root } = &root_paths;

    let gc_root = if root_paths.all_exist() {
        format!("{}", shell_gc_root.0.display())
    } else {
        "GC roots do not exist. Has the project been built with lorri yet?".to_string()
    };

    print!(
        "\
Project Shell File: {}
Project Garbage Collector Root: {}

General:
Lorri User GC Root Dir: {}
Lorri Daemon Socket: {}
",
        project.nix_file.display(),
        gc_root,
        paths.gc_root_dir().display(),
        paths.daemon_socket_file().display()
    );

    Ok(())
}

/// Bootstrap a new lorri project
///
/// See the documentation for lorri::cli::Command::Init for
/// more details
pub fn op_init(logger: &slog::Logger) -> Result<(), ExitError> {
    create_if_missing(
        Path::new("./shell.nix"),
        TRIVIAL_SHELL_SRC,
        "Make sure shell.nix is of a form that works with nix-shell.",
        logger,
    )
    .map_err(ExitError::user_error)?;

    create_if_missing(
        Path::new("./.envrc"),
        DEFAULT_ENVRC,
        &format!("Please add the following code to the top of your .envrc to set up lorri support (with fallback for plain nix):\n```bash\n{}```\n.", DEFAULT_ENVRC),
        logger,
    )
    .map_err(ExitError::user_error)?;

    info!(logger, "done");
    Ok(())
}

fn create_if_missing(
    path: &Path,
    contents: &str,
    msg: &str,
    logger: &slog::Logger,
) -> Result<(), io::Error> {
    if path.exists() {
        info!(logger, "file {} already exists, skipping", path.display(); "path" => path.to_str(), "message" => msg);
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
pub fn op_ping(nix_file: NixFile, logger: &slog::Logger) -> Result<(), ExitError> {
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
pub fn op_shell(
    project: Project,
    opts: ShellOptions,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let lorri = env::current_exe()
        .with_context(|| "failed to determine lorri executable's path")
        .map_err(ExitError::environment_problem)?;
    let shell = env::var_os("SHELL").ok_or_else(|| {
        ExitError::environment_problem(anyhow::anyhow!(
            "`lorri shell` requires the `SHELL` environment variable to be set"
        ))
    })?;
    let username = project::Username::from_env_var().map_err(ExitError::environment_problem)?;
    let nix_gc_root_user_dir = project::NixGcRootUserDir::get_or_create(&username)?;
    let cached = cached_root(&project);
    let mut bash_cmd = bash_cmd(
        if opts.cached {
            cached?
        } else {
            build_root(&project, cached.is_ok(), nix_gc_root_user_dir, logger)?
        },
        &project.cas,
        logger,
    )?;

    debug!(logger, "bash_cmd : {:?}", bash_cmd);
    let status = bash_cmd
        .args([
            OsStr::new("-c"),
            OsStr::new(
                "exec \"$1\" internal start-user-shell --shell-path=\"$2\" --shell-file=\"$3\"",
            ),
            OsStr::new("--"),
            &lorri.as_os_str(),
            &shell,
            project.nix_file.as_absolute_path().as_os_str(),
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
    nix_gc_root_user_dir: NixGcRootUserDir,
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

    Ok(project
        .create_roots(run_result, nix_gc_root_user_dir, &logger2)
        .map_err(|e| {
            ExitError::temporary(anyhow::Error::new(e).context("rooting the environment failed"))
        })?
        .shell_gc_root
        .0
        .as_path()
        .to_owned())
}

fn cached_root(project: &Project) -> Result<PathBuf, ExitError> {
    let root_paths = project.root_paths();
    if !root_paths.all_exist() {
        Err(ExitError::temporary(anyhow::anyhow!(
            "project has not previously been built successfully",
        )))
    } else {
        Ok(root_paths.shell_gc_root.0.as_path().to_owned())
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
            .as_path()
            .to_str()
            .expect("script file path not UTF-8 clean"),
    );
    Ok(cmd)
}

/// Helper command to create a user shell
///
/// See the documentation for `crate::ops::shell`.
pub fn op_start_user_shell(
    project: Project,
    opts: StartUserShellOptions_,
) -> Result<(), ExitError> {
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
            cmd.args([
                "--rcfile",
                rcfile
                    .as_path()
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
pub fn op_stream_events(kind: EventKind, logger: &slog::Logger) -> Result<(), ExitError> {
    let (tx_event, rx_event) = chan::unbounded::<Event>();

    let thread = {
        let address = crate::ops::get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_path().display());
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
                        writeln!(std::io::stdout()).expect("couldn't serialize event");
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
                UpgradeSource::Branch("rolling-release".to_string())
            }
            cli::UpgradeSource::Master => UpgradeSource::Branch("canon".to_string()),
            cli::UpgradeSource::Canon => UpgradeSource::Branch("canon".to_string()),
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
pub fn op_upgrade(
    upgrade_target: cli::UpgradeTo,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
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

        let mut expr = nix::CallOpts::file(upgrade_expr.as_path());

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
                Ok(())
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
pub fn op_watch(
    project: Project,
    opts: WatchOptions,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let username = project::Username::from_env_var().map_err(ExitError::temporary)?;
    let nix_gc_root_user_dir = project::NixGcRootUserDir::get_or_create(&username)?;
    if opts.once {
        main_run_once(project, nix_gc_root_user_dir, logger)
    } else {
        main_run_forever(project, nix_gc_root_user_dir, logger)
    }
}

fn main_run_once(
    project: Project,
    nix_gc_root_user_dir: NixGcRootUserDir,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    // TODO: add the ability to pass extra_nix_options to watch
    let mut build_loop = BuildLoop::new(
        &project,
        NixOptions::empty(),
        nix_gc_root_user_dir,
        logger.clone(),
    )
    .map_err(ExitError::temporary)?;
    match build_loop.once() {
        Ok(msg) => {
            info!(logger, "build message"; "message" => ?msg);
            Ok(())
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

#[derive(Serialize)]
/// Represents a gc root along with some metadata, used for json output of lorri gc info
struct GcRootInfo {
    /// directory where root is stored
    gc_dir: PathBuf,
    /// nix file from which the root originates. If None, then the root is considered dead.
    nix_file: Option<PathBuf>,
    /// timestamp of the last build
    timestamp: SystemTime,
    /// whether `nix_file` still exists
    alive: bool,
}

/// Returns a list of existing gc roots along with some metadata
fn list_roots(logger: &slog::Logger) -> Result<Vec<GcRootInfo>, ExitError> {
    let paths = crate::ops::get_paths()?;
    let mut res = Vec::new();
    let gc_root_dir = paths.gc_root_dir();
    for entry in std::fs::read_dir(gc_root_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            debug!(
                logger,
                "Skipping {} which should be a directory",
                entry.path().display()
            );
            continue;
        }
        let gc_dir = entry.path();
        let gc_root_dir = gc_dir.join("gc_root");
        if !std::fs::metadata(&gc_root_dir).map_or(false, |m| m.is_dir()) {
            debug!(
                logger,
                "Skipping {} which should be a directory",
                gc_root_dir.display()
            );
            continue;
        };
        let timestamp = match std::fs::symlink_metadata(gc_root_dir.join("shell_gc_root")) {
            Err(_) => {
                // no gc root, so nothing to report
                continue;
            }
            Ok(m) => m.modified().unwrap_or(std::time::UNIX_EPOCH),
        };
        let nix_file_symlink = gc_root_dir.join("nix_file");
        let nix_file = std::fs::read_link(&nix_file_symlink);
        let alive = match &nix_file {
            Err(_) => false,
            Ok(path) => match std::fs::metadata(path) {
                Ok(m) => m.is_file(),
                Err(_) => false,
            },
        };
        let nix_file = match nix_file {
            Err(_) => None,
            Ok(p) => Some(p),
        };
        res.push(GcRootInfo {
            gc_dir,
            nix_file,
            timestamp,
            alive,
        });
    }
    Ok(res)
}

#[derive(Serialize)]
/// How removing a gc root went, for json ouput
struct RemovalStatus {
    /// What error was encountered, or success
    error: Option<String>,
    /// The root we tried to remove
    root: GcRootInfo,
}

/// Print or remove gc roots depending on cli options.
pub fn gc(logger: &slog::Logger, opts: crate::cli::GcOptions) -> Result<(), ExitError> {
    let infos = list_roots(logger)?;
    match opts.action {
        cli::GcSubcommand::Info => {
            if opts.json {
                serde_json::to_writer(std::io::stdout(), &infos)
                    .expect("could not serialize gc roots");
            } else {
                for info in infos {
                    let target = match info.nix_file {
                        Some(p) => p.display().to_string(),
                        None => "(?)".to_owned(),
                    };
                    let age = match info.timestamp.elapsed() {
                        Err(_) => "future".to_owned(),
                        Ok(d) => {
                            let days = d.as_secs() / (24 * 60 * 60);
                            format!("{} days ago", days)
                        }
                    };
                    let alive = if info.alive { "" } else { "[dead]" };
                    println!(
                        "{} -> {} {} ({})",
                        info.gc_dir.display(),
                        target,
                        alive,
                        age
                    );
                }
            }
        }
        cli::GcSubcommand::Rm {
            shell_file,
            all,
            older_than,
        } => {
            let files_to_remove: HashSet<PathBuf> = shell_file.into_iter().collect();
            let to_remove: Vec<GcRootInfo> = infos
                .into_iter()
                .filter(|root| {
                    all || !root.alive
                        || root
                            .nix_file
                            .as_ref()
                            .map_or(false, |p| files_to_remove.contains(p))
                        || older_than.map_or(false, |limit| {
                            root.timestamp
                                .elapsed()
                                .map_or(false, |actual| actual > limit)
                        })
                })
                .collect();
            let mut result = Vec::new();
            let n = to_remove.len();
            for info in to_remove {
                match remove_dir_all(&info.gc_dir) {
                    Ok(_) => {
                        if opts.json {
                            result.push(RemovalStatus {
                                error: None,
                                root: info,
                            });
                        }
                    }
                    Err(e) => {
                        if opts.json {
                            result.push(RemovalStatus {
                                error: Some(e.to_string()),
                                root: info,
                            });
                        } else {
                            eprintln!("Error: could not remove {}: {}", info.gc_dir.display(), e);
                        }
                    }
                }
            }
            if opts.json {
                serde_json::to_writer(std::io::stdout(), &result)
                    .expect("failed to serialize result");
            } else {
                println!("Removed {} gc roots.", n);
                if n > 0 {
                    println!("Remember to run nix-collect-garbage to actually free space.");
                }
            }
        }
    }
    Ok(())
}

fn main_run_forever(
    project: Project,
    nix_gc_root_user_dir: NixGcRootUserDir,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let (tx_build_results, rx_build_results) = chan::unbounded();
    let (tx_ping, rx_ping) = chan::unbounded();
    let logger2 = logger.clone();
    // TODO: add the ability to pass extra_nix_options to watch
    let build_thread = {
        Async::run(logger, move || {
            match BuildLoop::new(&project, NixOptions::empty(), nix_gc_root_user_dir, logger2) {
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
