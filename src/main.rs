use backtrace::Backtrace;
use clap::Parser;
use lorri::cli::{Arguments, Command, Internal_, Verbosity};
use lorri::logging;
use lorri::ops::error::ExitError;
use lorri::project::{Project, ProjectFile};
use lorri::sqlite::Sqlite;
use lorri::{lorri_runtime_block_on, ops};
use nix::sys::signal::SigHandler::SigIgn;
use nix::sys::signal::{signal, Signal};
use slog::{debug, error, o};
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::{env, mem, panic};

const TRIVIAL_SHELL_SRC: &str = include_str!("./trivial-shell.nix");
const DEFAULT_ENVRC: &str = include_str!("./default-envrc");

fn main() {
    setup_panic();

    // Ignore SIGPIPE, otherwise we get SIGPIPE panics if a client disconnects from out daemon.
    // When ignoring SIGPIPE, an `Err` is returned instead of the signal panic.
    unsafe {
        if let Err(e) = signal(Signal::SIGPIPE, SigIgn) {
            writeln!(std::io::stderr(), "could not setup SIGPIPE ignore: {e}").unwrap();
        }
    }

    let exit_code = {
        lorri_runtime_block_on(async {
            let opts = Arguments::parse();

            let verbosity = match opts.verbosity {
                // -v flag was given 0 times
                0 => Verbosity::DefaultInfo,
                // -v flag was specified one or more times, we log everything
                _n => Verbosity::Debug,
            };

            // This logger is asynchronous. It is guaranteed to be flushed upon destruction. By tying
            // its lifetime to this smaller scope, we ensure that it is destroyed before
            // 'std::process::exit' gets called.
            let logger = logging::root(verbosity);
            debug!(logger, "input options"; "options" => ?opts);

            match run_command(&logger, opts).await {
                Err(err) => {
                    error!(logger, "{}", err.message());
                    err.exitcode()
                }
                Ok(()) => 0,
            }
        })
    };

    std::process::exit(exit_code);
}

/// Run the main function of the relevant command.
async fn run_command(orig_logger: &slog::Logger, opts: Arguments) -> Result<(), ExitError> {
    let paths = ops::get_paths()?;
    let conn = Sqlite::new_connection(&paths.sqlite_db).await;

    // TODO: TMP
    conn.migrate_gc_roots(&orig_logger, &paths, conn.clone())
        .await
        .unwrap();

    match opts.command {
        Command::Info(opts) => {
            let (project, logger) =
                with_project(conn, orig_logger, &opts.source.try_into()?).await?;
            ops::op_info(&paths, project, &logger).await
        }
        Command::Gc(opts) => ops::op_gc(orig_logger, opts, &paths).await,
        Command::Direnv(opts) => {
            let (project, logger) =
                with_project(conn, orig_logger, &opts.source.try_into()?).await?;
            ops::op_direnv(
                project,
                &paths,
                /* shell_output */ std::io::stdout(),
                &logger,
            )
            .await
        }
        Command::Shell(opts) => {
            let (project, logger) =
                with_project(conn, orig_logger, &opts.source.clone().try_into()?).await?;
            ops::op_shell(project, &paths.cas_store(), opts, &logger).await
        }

        Command::Watch(opts) => {
            let (project, logger) =
                with_project(conn, orig_logger, &opts.source.clone().try_into()?).await?;
            ops::op_watch(project, &paths.cas_store(), opts, &logger).await
        }
        Command::Daemon(opts) => {
            ctrlc::set_handler(move || {
                std::process::exit(0);
            })
            .expect("Error setting SIGINT and SIGTERM handler");
            ops::op_daemon(opts, orig_logger).await
        }
        Command::Init => ops::op_init(TRIVIAL_SHELL_SRC, DEFAULT_ENVRC, orig_logger),

        Command::Internal { command } => match command {
            Internal_::Ping_(opts) => {
                ops::op_ping(&paths, opts.source.try_into()?, orig_logger).await
            }
            Internal_::StartUserShell_(opts) => {
                ops::op_start_user_shell(orig_logger, paths.cas_store(), opts)
            }
            Internal_::StreamEvents_(se) => {
                ops::op_stream_events(&paths, se.kind, orig_logger).await
            }
        },
    }
}

async fn with_project(
    conn: Sqlite,
    logger: &slog::Logger,
    project_file: &ProjectFile,
) -> Result<(Project, slog::Logger), ExitError> {
    let project =
        Project::new_and_gc_nix_files(conn, project_file.clone(), ops::get_paths()?.gc_root_dir())
            .await
            .map_err(|err| {
                ExitError::temporary(anyhow::anyhow!(err).context("Could not set up project paths"))
            })?;
    let logger = logger.new(o!("nix_file" => project_file.clone()));
    Ok((project, logger))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Try instantiating the trivial shell file we provide the user.
    #[test]
    fn trivial_shell_nix() -> std::io::Result<()> {
        let nixpkgs = "./nix/bogus-nixpkgs/";

        // Sanity check the test environment
        assert!(Path::new(nixpkgs).is_dir(), "nixpkgs must be a directory");
        assert!(
            Path::new(nixpkgs).join("default.nix").is_file(),
            "nixpkgs/default.nix must be a file"
        );

        let out = std::process::Command::new("nix-instantiate")
            // we can’t assume to have a <nixpkgs>, so use bogus-nixpkgs
            .args(["-I", &format!("nixpkgs={}", nixpkgs)])
            .args(["--expr", TRIVIAL_SHELL_SRC])
            .output()?;
        assert!(
            out.status.success(),
            "stdout:\n{}\nstderr:{}\n",
            std::str::from_utf8(&out.stdout).unwrap(),
            std::str::from_utf8(&out.stderr).unwrap()
        );
        Ok(())
    }
    #[test]
    fn test_locate_config_file() {
        let mut shell_nix = env::current_dir().unwrap().join("shell.nix");
        assert!(shell_nix.exists());

        let manifest_shell = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shell.nix");
        assert!(manifest_shell.exists());
        assert_eq!(shell_nix, manifest_shell);
        shell_nix.pop();
        shell_nix.push("this-lorri-specific-file-probably-does-not-exist");

        assert!(!shell_nix.exists());
    }
}

fn setup_panic() {
    enum PanicStyle {
        Debug,
        Human,
    }

    let mut style = PanicStyle::Human;

    // all these lead to “normal” rust panic output
    if cfg!(debug_assertions) {
        style = PanicStyle::Debug
    }
    if let Ok(_) = env::var("RUST_BACKTRACE") {
        style = PanicStyle::Debug
    };
    if let Ok(_) = env::var("LORRI_NO_INSTALL_PANIC_HANDLER") {
        style = PanicStyle::Debug
    }
    if let Ok(_) = env::var("RUST_BACKTRACE") {
        style = PanicStyle::Debug
    };

    match style {
        PanicStyle::Debug => {}
        PanicStyle::Human => {
            panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
                let message = match (
                    info.payload().downcast_ref::<&str>(),
                    info.payload().downcast_ref::<String>(),
                ) {
                    (Some(s), _) => Some((*s).to_owned()),
                    (_, Some(s)) => Some(s.to_owned()),
                    (None, None) => None,
                };

                let cause = message.unwrap_or_else(|| "Unknown".into());

                let cause_first = cause.lines().next();

                let expl = match info.location() {
                    Some(location) => format!(
                        "Panic occurred in file '{}' at line {}\n",
                        location.file(),
                        location.line()
                    ),
                    None => "Panic location unknown.\n".to_string(),
                };

                let pos = info
                    .location()
                    .map(|location| format!("'{}', line {}\n: ", location.file(), location.line()));

                let crate_version = env!("CARGO_PKG_VERSION");

                let operating_system = os_info::get().to_string();
                let backtrace = render_backtrace();

                let title = format!(
                    "Crash: {}{}",
                    pos.unwrap_or("".to_string()),
                    cause_first.unwrap_or("")
                );
                let body = format!(
                    r##"# Crash Report

<!-- Describe here what you did that lead to the crash -->
## Crash Info

* OS: {operating_system}
* Lorri Version: {crate_version}

{expl}
Cause: {cause}

Backtrace:
```
{backtrace}
```
"##
                );

                let issue_url = format!(
                    "https://github.com/nix-community/lorri/issues/new?title={}&body={}",
                    urlencoding::encode(&title),
                    urlencoding::encode(&body)
                );

                let common = r##"lorri had a problem and crashed. To help us diagnose the problem you can send us a crash report.

We take privacy seriously, and do not perform any automated error collection. In order to improve the software, we rely on people to submit reports.
"##;

                // if the URL would get longer than 2000 characters, we just add the title
                // and let the user copy the error themselves.
                let full_message = if issue_url.as_bytes().len() <= 2000 {
                    format!(
                        "{common}\nYou can submit a report to github by clicking on the following URL:\n\n{}\n",
                        issue_url
                    )
                } else {
                    format!(
                        r##"{body}
{common}
You can submit a report to github but going to the following URL and pasting the error message before this text:
https://github.com/nix-community/lorri/issues/new?title={}
"##,
                        urlencoding::encode(&title),
                    )
                };

                let stderr = std::io::stderr();
                let mut stderr = stderr.lock();
                writeln!(
                    &mut stderr,
                    r##"{full_message}
Thank you kindly!
"##
                )
                .expect("printing error message to console failed");
                drop(stderr);
            }));
        }
    }
}

fn render_backtrace() -> String {
    //We take padding for address and extra two letters
    //to pad after index.
    #[allow(unused_qualifications)] // needed for pre-1.80 MSRV
    const HEX_WIDTH: usize = mem::size_of::<usize>() * 2 + 2;
    //Padding for next lines after frame's address
    const NEXT_SYMBOL_PADDING: usize = HEX_WIDTH + 6;

    let mut backtrace = String::new();

    //Here we iterate over backtrace frames
    //(each corresponds to function's stack)
    //We need to print its address
    //and symbol(e.g. function name),
    //if it is available
    let bt = Backtrace::new();
    let symbols = bt
        .frames()
        .iter()
        .flat_map(|frame| {
            let symbols = frame.symbols();
            if symbols.is_empty() {
                vec![(frame, None, "<unresolved>".to_owned())]
            } else {
                symbols
                    .iter()
                    .map(|s| {
                        (
                            frame,
                            Some(s),
                            s.name()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "<unknown>".to_owned()),
                        )
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();
    let begin_unwind = "rust_begin_unwind";
    let begin_unwind_start = symbols
        .iter()
        .position(|(_, _, n)| n == begin_unwind)
        .unwrap_or(0);
    for (entry_idx, (frame, symbol, name)) in symbols.iter().skip(begin_unwind_start).enumerate() {
        let ip = frame.ip();
        let _ = writeln!(backtrace, "{entry_idx:4}: {ip:HEX_WIDTH$?} - {name}");
        if let Some(symbol) = symbol {
            //See if there is debug information with file name and line
            if let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno()) {
                let _ = writeln!(
                    backtrace,
                    "{:3$}at {}:{}",
                    "",
                    file.display(),
                    line,
                    NEXT_SYMBOL_PADDING
                );
            }
        }
    }

    backtrace
}
