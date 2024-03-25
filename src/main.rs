use anyhow::anyhow;
use lorri::cli::{Arguments, Command, Internal_, Verbosity};
use lorri::logging;
use lorri::ops::error::ExitError;
use lorri::project::{Project, ProjectFile};
use lorri::{constants, AbsPathBuf};
use lorri::{ops, AbsDirPathBuf};
use slog::{debug, o};
use std::convert::TryInto;
use std::env;
use std::path::Path;
use structopt::StructOpt;

const TRIVIAL_SHELL_SRC: &str = include_str!("./trivial-shell.nix");
const DEFAULT_ENVRC: &str = include_str!("./default-envrc");

fn main() -> Result<(), ExitError> {
    install_panic_handler();

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

    run_command(&logger, opts)
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

/// Search for `name` in the current directory.
/// If `name` is an absolute path and a file, it returns the file.
/// If it doesn’t exist, returns `None`.
pub fn is_file_in_current_directory(name: &Path) -> anyhow::Result<Option<AbsPathBuf>> {
    let path = AbsDirPathBuf::current_dir()
        .unwrap_or_else(|orig| {
            panic!(
                "Expected `env::current_dir` to return an absolute path, but was {}",
                orig
            )
        })
        .relative_to(name.to_path_buf())
        .map_err(|p| anyhow!("Current dir is not dir: {:?}", p))?;
    Ok(if path.as_path().is_file() {
        Some(path)
    } else {
        None
    })
}

fn create_project(paths: &constants::Paths, shell_nix: ProjectFile) -> Result<Project, ExitError> {
    Project::new(shell_nix, paths.gc_root_dir(), paths.cas_store().clone()).map_err(|err| {
        ExitError::temporary(anyhow::anyhow!(err).context("Could not set up project paths"))
    })
}

/// Run the main function of the relevant command.
fn run_command(logger: &slog::Logger, opts: Arguments) -> Result<(), ExitError> {
    let paths = lorri::ops::get_paths()?;

    match opts.command {
        Command::Info(opts) => {
            let (project, _logger) = with_project(logger, &opts.source.try_into()?)?;
            ops::info(project)
        }
        Command::Gc(opts) => ops::gc(logger, opts),
        Command::Direnv(opts) => {
            let (project, logger) = with_project(logger, &opts.source.try_into()?)?;
            ops::direnv(project, /* shell_output */ std::io::stdout(), &logger)
        }
        Command::Shell(opts) => {
            let (project, logger) = with_project(logger, &opts.source.clone().try_into()?)?;
            ops::shell(project, opts, &logger)
        }

        Command::Watch(opts) => {
            let (project, logger) = with_project(logger, &opts.source.clone().try_into()?)?;
            ops::watch(project, opts, &logger)
        }
        Command::Daemon(opts) => {
            install_signal_handler();
            ops::daemon(opts, logger)
        }
        Command::Upgrade(opts) => ops::upgrade(opts, paths.cas_store(), logger),
        Command::Init => ops::init(TRIVIAL_SHELL_SRC, DEFAULT_ENVRC, logger),

        Command::Internal { command } => match command {
            Internal_::Ping_(opts) => ops::ping(opts.source.try_into()?, logger),
            Internal_::StartUserShell_(opts) => {
                let (project, _logger) = with_project(logger, &opts.source.clone().try_into()?)?;
                ops::start_user_shell(project, opts)
            }
            Internal_::StreamEvents_(se) => ops::stream_events(se.kind, logger),
        },
    }
}

fn with_project(
    logger: &slog::Logger,
    project_file: &ProjectFile,
) -> std::result::Result<(Project, slog::Logger), ExitError> {
    let project = create_project(&lorri::ops::get_paths()?, project_file.clone())?;
    let logger = logger.new(o!("nix_file" => project.file.clone()));
    Ok((project, logger))
}

#[cfg(test)]
mod tests {
    use lorri::AbsPathBuf;

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
            .args(&["-I", &format!("nixpkgs={}", nixpkgs)])
            .args(&["--expr", TRIVIAL_SHELL_SRC])
            .output()?;
        assert!(
            out.status.success(),
            "stdout:\n{}\nstderr:{}\n",
            std::str::from_utf8(&out.stdout).unwrap(),
            std::str::from_utf8(&out.stderr).unwrap()
        );
        Ok(())

        // TODO: provide .instantiate(), which does a plain nix-instantiate
        // and returns the .drv file.
        // let res = nix::CallOpts::expression(TRIVIAL_SHELL_SRC)
        //     .instantiate();

        // match res {
        //     Ok(_drv) => Ok(()),
        //     Err(nix::InstantiateError::ExecutionFailed(output)) =>
        //         panic!(
        //             "stdout:\n{}\nstderr:{}\n",
        //             std::str::from_utf8(&output.stdout).unwrap(),
        //             std::str::from_utf8(&output.stderr).unwrap()
        //         ),
        //     Err(nix::InstantiateError::Io(io)) => Err(io)
        // }
    }
    #[test]
    fn test_locate_config_file() {
        let mut path = PathBuf::from("shell.nix");
        let result = is_file_in_current_directory(&path);
        assert_eq!(
            result
                .unwrap()
                .expect("Should find the shell.nix in this projects' root"),
            AbsPathBuf::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                .unwrap()
                .join("shell.nix")
        );
        path.pop();
        path.push("this-lorri-specific-file-probably-does-not-exist");
        assert_eq!(None, is_file_in_current_directory(&path).unwrap());
    }
}
