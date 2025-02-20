//! Ops are command-line callables.

mod direnv;
pub mod error;

use crate::build_loop::BuildLoop;
use crate::build_loop::Event;
use crate::build_loop::Reason;
use crate::builder;
use crate::builder::OutputPath;
use crate::cas::ContentAddressable;
use crate::cli;
use crate::cli::StartUserShellOptions_;
use crate::cli::WatchOptions;
use crate::cli::{EventKind, ShellOptions};
use crate::constants::Paths;
use crate::daemon::client::{self, DaemonInfo};
use crate::daemon::Daemon;
use crate::nix::options::NixOptions;
use crate::nix::CallOpts;
use crate::ops::direnv::{DirenvVersion, MIN_DIRENV_VERSION};
use crate::ops::error::ExitError;
use crate::path_to_json_string;
use crate::socket::path::SocketPath;
use crate::AbsPathBuf;
use std::ffi::OsStr;
use std::fs::remove_dir_all;
use std::io::{Error, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;
use std::{collections::HashSet, env, fs::File, time::SystemTime};

use anyhow::Context;

use crate::project::{Project, ProjectFile};
use itertools::Itertools;
use serde_json::json;
use serde_json::Value;
use slog::{debug, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{channel, unbounded_channel};

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
pub async fn op_daemon(
    opts: crate::cli::DaemonOptions,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let extra_nix_options = match opts.extra_nix_options {
        None => NixOptions::empty(),
        Some(v) => NixOptions {
            builders: v.builders,
            substituters: v.substituters,
        },
    };

    let (daemon, mut build_rx) = Daemon::new(extra_nix_options);
    let logger2 = logger.clone();
    let build_handle = tokio::task::spawn(async move {
        loop {
            match build_rx.recv().await {
                None => break,
                Some(msg) => info!(logger2, "build status"; "message" => ?msg),
            }
        }
    });
    info!(logger, "ready");

    let paths = crate::ops::get_paths()?;
    daemon
        .serve(
            &SocketPath::from(paths.daemon_socket_file().clone()),
            paths.gc_root_dir(),
            paths.cas_store().clone(),
            logger,
        )
        .await?;
    build_handle.await.expect("build_handle join failed");
    Ok(())
}

/// Emit shell script intended to be evaluated as part of direnv's .envrc
///
/// See the documentation for lorri::cli::Command::Direnv for more
/// details.
pub async fn op_direnv<W: std::io::Write>(
    project: Project,
    paths: &Paths,
    mut shell_output: W,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    check_direnv_version()?;

    let root_paths = project.root_paths();
    let paths_are_cached: bool = root_paths.all_exist();

    let ping_sent = {
        let address = crate::ops::get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_path().display());
        // TODO: maybe ping should indeed return something so we can at least check whether it parses the message and the version is right. Right now this collapses all of that into a bool …
        match client::create::<client::Ping>(paths, client::Timeout::from_millis(500), logger)
            .await
            .map_err(ExitError::from)
        {
            Err(_) => false,
            Ok(mut client) => {
                let res = client
                    .write(&client::Ping {
                        project_file: project.file.clone(),
                        rebuild: client::Rebuild::OnlyIfNotYetWatching,
                    })
                    .await
                    .is_ok();
                client.shutdown().await;
                res
            }
        }
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
pub async fn op_info(
    paths: &Paths,
    project: Project,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let root_paths = project.root_paths();
    let OutputPath { shell_gc_root } = &root_paths;
    let daemon_status =
        match client::create::<client::DaemonInfo>(paths, client::Timeout::from_millis(50), logger)
            .await
        {
            Err(init_error) => format!("`lorri daemon` is not up: {}", init_error),
            Ok(mut client) => {
                let res = match client.communicate(&DaemonInfo {}).await {
                    Ok(_) => "`lorri daemon` is running".to_string(),
                    Err(err) => format!("Problem connecting to the `lorri daemon`: {}", err),
                };
                client.shutdown().await;
                res
            }
        };

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
Lorri Daemon Status: {}
",
        project.file.as_nix_file().display(),
        gc_root,
        paths.gc_root_dir().display(),
        paths.daemon_socket_file().display(),
        daemon_status
    );

    Ok(())
}

/// Bootstrap a new lorri project
///
/// See the documentation for lorri::cli::Command::Init for
/// more details
pub fn op_init(
    default_shell: &str,
    default_envrc: &str,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
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
    Ok(())
}

fn create_if_missing(
    path: &Path,
    contents: &str,
    msg: &str,
    logger: &slog::Logger,
) -> Result<(), Error> {
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
pub async fn op_ping(
    paths: &Paths,
    project_file: ProjectFile,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let mut client = client::create(paths, client::Timeout::from_millis(500), logger).await?;
    client
        .write(&client::Ping {
            project_file,
            rebuild: client::Rebuild::Always,
        })
        .await?;
    client.shutdown().await;
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
pub async fn op_shell(
    project: Project,
    cas: &ContentAddressable,
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
    let root_paths = project.root_paths();
    let cached = if !root_paths.all_exist() {
        Err(ExitError::temporary(anyhow::anyhow!(
            "project has not previously been built successfully",
        )))
    } else {
        Ok(root_paths.shell_gc_root.0.as_path().to_owned())
    };
    let mut bash_cmd = bash_cmd(
        if opts.cached {
            cached?
        } else {
            build_root(&project, cached.is_ok(), cas, logger).await?
        },
        cas,
        logger,
    )
    .await?;

    debug!(logger, "bash_cmd : {:?}", bash_cmd);
    let status = bash_cmd
        .args([
            OsStr::new("-c"),
            OsStr::new(
                "exec \"$1\" internal start-user-shell --shell-path=\"$2\" --shell-file=\"$3\"",
            ),
            OsStr::new("--"),
            lorri.as_os_str(),
            &shell,
            project.file.as_absolute_path().as_os_str(),
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

async fn build_root(
    project: &Project,
    cached: bool,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> Result<PathBuf, ExitError> {
    let logger2 = logger.clone();
    let progress_thread = tokio::spawn(async move {
        // Keep track of the start time to display a hint to the user that they can use `--cached`,
        // but only if a cached version of the environment exists
        let mut start = if cached { Some(Instant::now()) } else { None };

        eprint!("lorri: building environment");
        loop {
            // Show `--cached` hint once after some time has passed
            if let Some(start_time) = start {
                if start_time.elapsed() >= Duration::from_secs(3) {
                    eprintln!(
                        "\nHint: you can use `lorri shell --cached` to use the most recent \
                         environment that was built successfully."
                    );
                    start = None; // Don't show the hint again
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Indicate progress
            eprint!(".");
            tokio::io::stderr().flush().await.expect("couldn’t flush‽");
        }
    });

    // TODO: add the ability to pass extra_nix_options to shell
    let run_result = match &project.file {
        ProjectFile::ShellNix(nix_file) => {
            builder::instantiate_and_build(nix_file, &cas, &NixOptions::empty(), &logger2).await
        }
        ProjectFile::FlakeNix(installable) => builder::flake(installable, &logger2).await,
    };
    eprintln!(". done");
    progress_thread.abort();

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
        .create_roots(run_result)
        .map_err(|e| {
            ExitError::temporary(anyhow::Error::new(e).context("rooting the environment failed"))
        })?
        .shell_gc_root
        .0
        .as_path()
        .to_owned())
}

/// Instantiates a `Command` to start bash.
pub async fn bash_cmd(
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
        .await
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
    cas: &ContentAddressable,
    opts: StartUserShellOptions_,
) -> Result<(), ExitError> {
    // This temporary directory will not be cleaned up by lorri because we exec into the shell
    // process, which means that destructors will not be run. However, (1) the temporary files
    // lorri creates in this directory are only a few hundred bytes long; (2) the directory will be
    // cleaned up on reboot or whenever the OS decides to purge temporary directories.
    let tempdir = tempfile::tempdir().expect("failed to create temporary directory");
    let e = shell_cmd(opts.shell_path.as_ref(), cas, tempdir.path()).exec();

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
            std::fs::write(
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

/// Run to output a stream of build events in a machine-parseable form.
///
/// See the documentation for lorri::cli::Command::StreamEvents_ for more
/// details.
pub async fn op_stream_events(
    paths: &Paths,
    kind: EventKind,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    {
        let address = get_paths()?.daemon_socket_file().clone();
        debug!(logger, "connecting to socket"; "socket" => address.as_path().display());
        let logger2 = logger.clone();
        let paths2 = (*paths).clone();
        // This async will not block when it is dropped,
        // since it only reads messages and don’t want to block exit in the Snapshot case.
        let mut client = client::create::<client::StreamEvents>(
            &paths2,
            // infinite timeout because we are listening indefinitely
            client::Timeout::Infinite,
            &logger2,
        )
        .await?;

        client.write(&client::StreamEvents {}).await?;
        let mut snapshot_done = false;
        let res = loop {
            let res = client.read().await?;

            match res {
                Event::SectionEnd => {
                    debug!(logger, "SectionEnd");
                    match kind {
                        // If we only want the snapshot, quit the program
                        EventKind::Snapshot => break Ok(()),
                        // Else we now start sending the incremental data
                        _ => {
                            snapshot_done = true;
                        }
                    }
                }
                ev => match (snapshot_done, &kind) {
                    (_, EventKind::All)
                    | (false, EventKind::Snapshot)
                    | (true, EventKind::Live) => {
                        let json: serde_json::Value = match ev {
                            Event::SectionEnd => json!({"SectionEnd":{}}),
                            Event::Started { nix_file, reason } => json!({
                              "Started": {
                                  "nix_file": nix_file.to_json_value(),
                                  "reason": match reason {
                                      Reason::PingReceived => json!({"PingReceived": {}}),
                                      Reason::FilesChanged(files) => json!({"FilesChanged": files.iter().map(|p| path_to_json_string(p)).collect::<Vec<serde_json::Value>>()})
                                  }
                              }
                            }),
                            Event::Completed {
                                nix_file,
                                rooted_output_paths,
                            } => json!({
                              "Completed": {
                                "nix_file": nix_file.to_json_value(),
                                "rooted_output_paths": {
                                    "shell_gc_root": rooted_output_paths.shell_gc_root.0.to_json_value()
                                }
                              }
                            }),
                            Event::Failure { nix_file, failure } => json!({
                              "Failure": {
                                "nix_file": nix_file.to_json_value(),
                                "failure": { "message": format!("{}",  failure) }
                              }
                            }),
                        };

                        let mut vec = serde_json::to_vec(&json).expect("couldn't serialize event");
                        vec.extend_from_slice("\n".as_bytes());
                        tokio::io::stdout()
                            .write_all(&vec)
                            .await
                            .expect("couldn’t write serialized event");
                        tokio::io::stdout()
                            .flush()
                            .await
                            .expect("couldn’t flush serialized event");
                    }
                    _ => (),
                },
            }
        };

        client.shutdown().await;
        res
    }
}

/// Run a BuildLoop for `shell.nix`, watching for input file changes.
/// Can be used together with `direnv`.
///
/// See the documentation for lorri::cli::Command::Shell for more
/// details.
pub async fn op_watch(
    project: Project,
    cas: &ContentAddressable,
    opts: WatchOptions,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    if opts.once {
        main_run_once(project, cas, logger).await
    } else {
        main_run_forever(project, cas, logger).await
    }
}

async fn main_run_once(
    project: Project,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    // TODO: add the ability to pass extra_nix_options to watch
    let build_loop = BuildLoop::new(project, NixOptions::empty(), cas.clone(), logger.clone())
        .map_err(ExitError::temporary)?;
    match build_loop.once().await {
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
    gc_dir: AbsPathBuf,
    /// nix file from which the root originates. If None, then the root is considered dead.
    nix_file: Option<PathBuf>,
    /// timestamp of the last build
    timestamp: SystemTime,
    /// whether `nix_file` still exists
    alive: bool,
}

impl GcRootInfo {
    fn format_pretty_oneline(&self) -> String {
        let target = match &self.nix_file {
            Some(p) => p.display().to_string(),
            None => "(?)".to_owned(),
        };
        let age = match self.timestamp.elapsed() {
            Err(_) => "future".to_owned(),
            Ok(d) => {
                let days = d.as_secs() / (24 * 60 * 60);
                format!("{} days ago", days)
            }
        };
        let alive = if self.alive { "" } else { "[dead]" };
        format!(
            "{} -> {} {} ({})",
            self.gc_dir.display(),
            target,
            alive,
            age
        )
    }
}

/// Returns a list of existing gc roots along with some metadata
fn list_roots(logger: &slog::Logger) -> Result<Vec<GcRootInfo>, ExitError> {
    let paths = get_paths()?;
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
        let gc_dir = AbsPathBuf::new(entry.path()).expect("entry.path() should always be absolute");
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
        let nix_file = std::fs::read_link(nix_file_symlink);
        let alive = match &nix_file {
            Err(_) => false,
            Ok(path) => match std::fs::metadata(path) {
                Ok(m) => m.is_file(),
                Err(_) => false,
            },
        };
        let nix_file = nix_file.ok();
        res.push(GcRootInfo {
            gc_dir,
            nix_file,
            timestamp,
            alive,
        });
    }
    Ok(res)
}

/// Print or remove gc roots depending on cli options.
pub fn op_gc(logger: &slog::Logger, opts: cli::GcOptions) -> Result<(), ExitError> {
    let infos = list_roots(logger)?;
    match opts.action {
        cli::GcSubcommand::Info => {
            if opts.json {
                serde_json::to_writer(
                    std::io::stdout(),
                    &infos
                        .iter()
                        .map(|info| {
                            json!({
                                "gc_dir": info.gc_dir.to_json_value(),
                                "nix_file": info.nix_file.as_ref().map_or(Value::Null, |n| path_to_json_string(&n)),
                                "timestamp": info.timestamp,
                                "alive": info.alive
                            })
                        })
                        .collect::<Vec<_>>(),
                )
                .expect("could not serialize gc roots");
            } else {
                for info in infos {
                    println!("{}", info.format_pretty_oneline());
                }
            }
        }
        cli::GcSubcommand::Rm {
            shell_file,
            all,
            older_than,
            dry_run,
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
            if dry_run {
                if to_remove.len() > 0 {
                    println!("--dry-run: Would delete the following GC roots:");
                    for info in to_remove {
                        println!("{}", info.format_pretty_oneline());
                    }
                } else {
                    println!("--dry-run: Would not delete any GC roots");
                }
            } else {
                for info in to_remove {
                    match remove_dir_all(&info.gc_dir) {
                        Ok(_) => {
                            result.push(Ok(info));
                        }
                        Err(e) => {
                            result.push(Err((info, e.to_string())));
                        }
                    }
                }
                if opts.json {
                    let res = result
                        .into_iter()
                        .map(|r| match r {
                            Err((info, err)) => json!({
                                // Error, if any
                                "error": err,
                                // The root we tried to remove
                                "root": info
                            }),
                            Ok(info) => json!({
                                "error": null,
                                "root": info
                            }),
                        })
                        .collect::<Vec<_>>();
                    serde_json::to_writer(std::io::stdout(), &res)
                        .expect("failed to serialize result");
                } else {
                    let (ok, err): (Vec<_>, Vec<_>) = result.into_iter().partition_result();
                    println!("Removed {} gc roots.", ok.len());
                    if err.len() > 0 {
                        for (info, e) in err {
                            warn!(
                                logger,
                                "Failed to remove gc root: {}: {}",
                                info.gc_dir.display(),
                                e
                            )
                        }
                    }
                    if ok.len() > 0 {
                        println!("Remember to run nix-collect-garbage to actually free space.");
                    }
                }
            }
        }
    }
    Ok(())
}

async fn main_run_forever(
    project: Project,
    cas: &ContentAddressable,
    logger: &slog::Logger,
) -> Result<(), ExitError> {
    let (tx_build_results, mut rx_build_results) = unbounded_channel();
    let (tx_ping, rx_ping) = channel(10);
    let logger2 = logger.clone();
    let cas2 = cas.clone();
    // TODO: add the ability to pass extra_nix_options to watch
    let build_loop = tokio::task::spawn(async move {
        match BuildLoop::new(project, NixOptions::empty(), cas2, logger2) {
            Ok(bl) => bl.forever(tx_build_results, rx_ping).await,
            Err(e) => Err(ExitError::temporary(e)),
        }
    });

    // We ping the build loop once, to make it run the first build immediately
    tx_ping
        .send(())
        .await
        .expect("could not send ping to build_loop");

    let logger2 = logger.clone();
    let print_build_message = tokio::task::spawn(async move {
        loop {
            let Some(msg) = rx_build_results.recv().await else {
                break;
            };
            info!(logger2, "build message"; "message" => ?msg);
        }
    })
    .abort_handle();

    let res = build_loop.await.expect("unable to join");
    print_build_message.abort();
    res
}
