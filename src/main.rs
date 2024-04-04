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

    // TODO: Once the 'Termination' trait has been stabilised, 'Result<(), ExitError>' should implement
    // 'Termination' and 'main' should return 'Result<(), ExitError>'.
    // https://doc.rust-lang.org/std/process/trait.Termination.html
    // https://github.com/rust-lang/rfcs/blob/master/text/1937-ques-in-main.md
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
    match is_file_in_current_directory(shellfile) {
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

    let with_project = |nix_file| -> std::result::Result<(Project, slog::Logger), ExitError> {
        let project = create_project(&lorri::ops::get_paths()?, find_nix_file(nix_file)?)?;
        let logger = logger.new(o!("nix_file" => project.nix_file.clone()));
        Ok((project, logger))
    };

    match opts.command {
        Command::Info(opts) => {
            let (project, _logger) = with_project(&opts.nix_file)?;
            ops::info(project)
        }
        Command::Gc(opts) => ops::gc(logger, opts),
        Command::Direnv(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::direnv(project, /* shell_output */ std::io::stdout(), &logger)
        }
        Command::Shell(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::shell(project, opts, &logger)
        }

        Command::Watch(opts) => {
            let (project, logger) = with_project(&opts.nix_file)?;
            ops::watch(project, opts, &logger)
        }
        Command::Daemon(opts) => {
            install_signal_handler();
            ops::daemon(opts, logger)
        }
        Command::Upgrade(opts) => ops::upgrade(opts, paths.cas_store(), logger),
        Command::Init => ops::init(logger),

        Command::Internal { command } => match command {
            Internal_::Ping_(opts) => {
                let nix_file = find_nix_file(&opts.nix_file)?;
                ops::ping(nix_file, logger)
            }
            Internal_::StartUserShell_(opts) => {
                let (project, _logger) = with_project(&opts.nix_file)?;
                ops::start_user_shell(project, opts)
            }
            Internal_::StreamEvents_(se) => ops::stream_events(se.kind, logger),
        },
    }
}

/// Search for `name` in the current directory.
/// If `name` is an absolute path and a file, it returns the file.
/// If it doesn’t exist, returns `None`.
pub fn is_file_in_current_directory(name: &Path) -> anyhow::Result<Option<AbsPathBuf>> {
    let path = AbsPathBuf::new(env::current_dir()?)
        .unwrap_or_else(|orig| {
            panic!(
                "Expected `env::current_dir` to return an absolute path, but was {}",
                orig.display()
            )
        })
        .join(name);
    Ok(if path.as_path().is_file() {
        Some(path)
    } else {
        None
    })
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
