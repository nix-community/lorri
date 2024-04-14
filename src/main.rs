use lorri::cli::{Arguments, Command, Internal_, Verbosity};
use lorri::logging;
use lorri::ops;
use lorri::ops::error::ExitError;
use lorri::project::Project;
use lorri::NixFile;
use lorri::{constants, AbsPathBuf};
use slog::{debug, error, o};
use std::env;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

const FLAKE_COMPAT_SHELL_SRC: &str = include_str!("./flake-compat-shell.nix");

fn main() {
    install_panic_handler();

    let exit_code = {
        let opts = Arguments::from_args();

        let verbosity = match opts.verbosity {
            // -v flag was given 0 times
            0 => Verbosity::DefaultInfo,
            // -v flag was specified one or more times, we log everything
            _n => Verbosity::Debug,
        };

        // This logger is asynchronous. It is guaranteed to be flushed upon destruction. By tying
        // its lifetime to this smaller scope, we ensure that it is destroyed before
        // 'std::process::exit' gets called.
        let logger = logging::root(verbosity, &opts.command);
        debug!(logger, "input options"; "options" => ?opts);

        match run_command(&logger, opts) {
            Err(err) => {
                error!(logger, "{}", err.message());
                err.exitcode()
            }
            Ok(()) => 0,
        }
    };

    std::process::exit(exit_code);
}

fn install_panic_handler() {
    if let Err(env::VarError::NotPresent) = env::var("LORRI_NO_INSTALL_PANIC_HANDLER") {
        // This returns 101 on panics, see also `ExitError::panic`.
        human_panic::setup_panic!();
    }
}

// Exit with return code 0 on SIGINT and SIGTERM
fn install_signal_handler() {
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .expect("Error setting SIGINT and SIGTERM handler");
}

/// Reads a nix filename given by the user and either returns
/// the `NixFile` type or exists with a helpful error message
/// that instructs the user how to write a minimal `shell.nix`.
fn find_nix_file(shellfile: &Path) -> Result<NixFile, ExitError> {
    // use shell.nix from cwd
    match AbsPathBuf::new_from_current_directory(shellfile) {
        Err(err) => Err(ExitError::temporary(err)),
        Ok(None) => Err(ExitError::user_error(
            if PathBuf::from("flake.nix").exists() {
                anyhow::anyhow!(
                    "lorri does not currently natively support flakes.\nYou can use the following compatibility `shell.nix` to use lorri with flakes:\n\n\
                    {}",
                    FLAKE_COMPAT_SHELL_SRC
                )
            } else {
                anyhow::anyhow!(
                    "`{}` does not exist\n\
                    You can use the following minimal `shell.nix` to get started:\n\n\
                    {}",
                    shellfile.display(),
                    ops::TRIVIAL_SHELL_SRC
                )
            },
        )),
        Ok(Some(file)) => Ok(NixFile::from(file)),
    }
}

fn create_project(paths: &constants::Paths, shell_nix: NixFile) -> Result<Project, ExitError> {
    Project::new(shell_nix, paths.gc_root_dir(), paths.cas_store().clone()).map_err(|err| {
        ExitError::temporary(anyhow::anyhow!(err).context("Could not set up project paths"))
    })
}

/// Run the main function of the relevant command.
fn run_command(logger: &slog::Logger, opts: Arguments) -> Result<(), ExitError> {
    let paths = lorri::ops::get_paths()?;

    let with_project_resolved =
        |nix_file| -> std::result::Result<(Project, slog::Logger), ExitError> {
            let project = create_project(&lorri::ops::get_paths()?, nix_file)?;
            let logger = logger.new(o!("nix_file" => project.nix_file.clone()));
            Ok((project, logger))
        };
    let with_project = |nix_file| -> std::result::Result<(Project, slog::Logger), ExitError> {
        with_project_resolved(find_nix_file(nix_file)?)
    };

    match opts.command {
        Command::Info(opts) => {
            let nix_file = match opts.nix_file {
                Some(f) => f,
                None => {
                    slog::info!(logger, "Printing info for `./shell.nix`. If you want info on a different project, pass `--shell-file`");
                    PathBuf::from("./shell.nix")
                }
            };
            let (project, _logger) = with_project(&nix_file)?;
            ops::op_info(&paths, project)
        }
        Command::Gc(opts) => ops::gc(logger, opts),
        Command::Direnv(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::op_direnv(
                project,
                &paths,
                /* shell_output */ std::io::stdout(),
                &logger,
            )
        }
        Command::Shell(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::op_shell(project, opts, &logger)
        }

        Command::Watch(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::op_watch(project, opts, &logger)
        }
        Command::Daemon(opts) => {
            install_signal_handler();
            ops::op_daemon(opts, logger)
        }
        Command::Upgrade(opts) => ops::op_upgrade(opts, paths.cas_store(), logger),
        Command::Init => ops::op_init(logger),

        Command::Internal { command } => match command {
            Internal_::Ping_(opts) => {
                let nix_file = find_nix_file(&opts.nix_file)?;
                ops::op_ping(&paths, nix_file, logger)
            }
            Internal_::StartUserShell_(opts) => {
                let (project, _logger) = with_project(&opts.nix_file)?;
                ops::op_start_user_shell(project, opts)
            }
            Internal_::StreamEvents_(se) => ops::op_stream_events(&paths, se.kind, logger),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
            .args(["--expr", ops::TRIVIAL_SHELL_SRC])
            .output()?;
        assert!(
            out.status.success(),
            "stdout:\n{}\nstderr:{}\n",
            std::str::from_utf8(&out.stdout).unwrap(),
            std::str::from_utf8(&out.stderr).unwrap()
        );
        Ok(())
    }
}
