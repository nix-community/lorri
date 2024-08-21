//! Defines the CLI interface using structopt.

// # Command line interface style guide
//
// Do not use short options unless they are extremely common and expected. A long option takes a
// bit more typing, but the long name makes the intent much more obvious. The only short option
// right now is `-v` for verbosity, and it should probably stay that way.
//
// See MAINTAINERS.md for details on internal and non-internal commands.

use std::{convert::TryFrom, path::PathBuf, str::FromStr, time::Duration};

use crate::{project::ProjectFile, AbsDirPathBuf, AbsPathBuf, Installable};
use clap::{
    error::{ContextKind, ContextValue},
    Parser, Subcommand,
};

#[derive(Parser, Debug)]
#[command(name = "lorri", version)]
/// Global arguments which set global program state. Most
/// arguments will be to sub-commands.
pub struct Arguments {
    /// Activate debug logging. Multiple occurrences are accepted for backwards compatibility, but
    /// have no effect. This will display all messages lorri logs.
    #[arg(short, long, default_value_t = 0, required = false)]
    pub verbosity: u8,

    /// Sub-command to execute
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Copy, Clone, Debug)]
/// Verbosity options lorri supports;
pub enum Verbosity {
    /// Default verbosity, print info and up
    DefaultInfo,
    /// Debug verbosity, print all messages
    Debug,
}

#[derive(Subcommand, Debug)]
/// Sub-commands which lorri can execute
pub enum Command {
    /// Emit shell script intended to be evaluated as part of direnv's .envrc, via: `eval "$(lorri
    /// direnv)"`
    Direnv(DirenvOptions),

    /// Remove lorri garbage collection roots that point to removed shell.nix files
    Gc(GcOptions),

    /// Show information about a lorri project
    Info(InfoOptions),

    /// Open a new project shell
    Shell(ShellOptions),

    /// Build project whenever an input file changes
    Watch(WatchOptions),

    /// Start the multi-project daemon. Replaces `lorri watch`
    Daemon(DaemonOptions),

    /// Write bootstrap files to current directory to create a new lorri project
    Init,

    /// Internal commands, only use to experiment with unstable features
    Internal {
        /// Sub-command to execute
        #[command(subcommand)]
        command: Internal_,
    },
}

/// Common options about the build source, defaults to `shell.nix`
#[derive(Parser, Debug, Clone)]
pub struct DefaultingSourceOptions {
    /// The .nix file in the current directory to use
    #[arg(long)]
    pub shell_file: Option<PathBuf>,

    /// The path to consider a flake source within
    #[arg(long = "context", default_value = ".", conflicts_with = "shell_file")]
    pub context_dir: PathBuf,

    /// The installable descriptor for a flake
    #[arg(long, conflicts_with = "shell_file")]
    pub flake: Option<String>,
}

fn from_current_dir(rel: &PathBuf) -> Result<AbsPathBuf, clap::Error> {
    AbsDirPathBuf::current_dir()?
        .relative_to(rel.clone())
        .map_err(|err| {
            let mut e = clap::error::Error::new(clap::error::ErrorKind::ValueValidation);
            e.insert(
                clap::error::ContextKind::InvalidValue,
                ContextValue::String(format!("could not make {:?} absolute: {:?}", rel, err)),
            );
            e
        })
}

impl TryFrom<DefaultingSourceOptions> for ProjectFile {
    type Error = clap::Error;

    fn try_from(opts: DefaultingSourceOptions) -> Result<Self, Self::Error> {
        match (opts.shell_file, opts.flake) {
            (Some(_), Some(_)) => {
                let mut e = clap::Error::new(clap::error::ErrorKind::ArgumentConflict);
                e.insert(
                    clap::error::ContextKind::Suggested,
                    ContextValue::String(
                        "cannot use nix-shell files and flakes together".to_string(),
                    ),
                );
                Err(e)
            }
            // XXX Consider more sophisticated default - e.g. first that exists: shell.nix, flake.nix, default.nix
            (None, None) => find_nix_file("shell.nix")
                .or_else(|| find_nix_file("flake.nix"))
                .or_else(|| find_nix_file("default.nix"))
                .ok_or_else(|| {
                    let mut e = clap::error::Error::new(clap::error::ErrorKind::ValueValidation);
                    e.insert(clap::error::ContextKind::Suggested, ContextValue::String(format!( "No default build sources found\n\
                    You can use a flake.nix file or the following minimal `shell.nix` to get started:\n\n\
                    {}", TRIVIAL_SHELL_SRC)));
                    e
                }),
            (Some(shell), None) => Ok(ProjectFile::ShellNix(from_current_dir(&shell)?.into())),
            (None, Some(flake)) => Ok(ProjectFile::FlakeNix(Installable {
                context: from_current_dir(&opts.context_dir)?,
                installable: flake,
            })),
        }
    }
}

const TRIVIAL_SHELL_SRC: &str = include_str!("./trivial-shell.nix");
/// Reads a nix filename given by the user and either returns
/// the `NixFile` type or exists with a helpful error message
/// that instructs the user how to write a minimal `shell.nix`.
fn find_nix_file(shellfile: &str) -> Option<ProjectFile> {
    let path = AbsDirPathBuf::current_dir()
        .ok()?
        .relative_to(shellfile.into())
        .ok()?;
    if !path.as_path().is_file() {
        return None;
    };

    // use shell.nix from cwd
    Some(
        match path
            .file_name()
            .expect("Should already have confirmed is_file")
            .to_str()
        {
            Some("flake.nix") => ProjectFile::flake(
                AbsPathBuf::new(
                    path.as_path()
                        .parent()
                        .expect("Should already be a file")
                        .to_path_buf(),
                )
                .expect("already absolute"),
                ".#".into(),
            ),
            _ => ProjectFile::ShellNix(path.into()),
        },
    )
}

/// Common options about the build source, where an explicit field is required
/// This version of the source options has no default value. That's on purpose: sometimes the user
/// will have projects with multiple shell files. This way, they are forced to think about which shell
/// file was causing problems when they submit a bug report.
#[derive(Parser, Debug, Clone)]
pub struct SourceOptions {
    /// The .nix file in the current directory to use
    #[arg(long)]
    pub shell_file: Option<PathBuf>,

    /// The path to consider a flake source within
    #[arg(long = "context", default_value = ".", conflicts_with = "shell_file")]
    pub context_dir: PathBuf,

    /// The installable descriptor for a flake
    #[arg(long = "flake", conflicts_with = "shell_file")]
    pub flake: Option<String>,
}

impl TryFrom<SourceOptions> for ProjectFile {
    type Error = clap::Error;

    fn try_from(opts: SourceOptions) -> Result<Self, Self::Error> {
        match (opts.shell_file, opts.flake) {
            (Some(_), Some(_)) => {
                let mut e = clap::error::Error::new(clap::error::ErrorKind::ArgumentConflict);
                e.insert(
                    ContextKind::InvalidValue,
                    ContextValue::String(
                        "cannot use nix-shell files and flakes together".to_string(),
                    ),
                );
                Err(e)
            }
            (None, None) => {
                let mut e =
                    clap::error::Error::new(clap::error::ErrorKind::MissingRequiredArgument);
                e.insert(
                    ContextKind::SuggestedArg,
                    ContextValue::String("either --shell-file or --flake is required".to_string()),
                );
                Err(e)
            }
            (Some(shell), None) => Ok(ProjectFile::ShellNix(from_current_dir(&shell)?.into())),
            (None, Some(flake)) => Ok(ProjectFile::FlakeNix(Installable {
                context: from_current_dir(&opts.context_dir)?,
                installable: flake,
            })),
        }
    }
}

/// Options for the `direnv` subcommand.
#[derive(Parser, Debug)]
pub struct DirenvOptions {
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: DefaultingSourceOptions,
}

/// Options for the `info` subcommand.
#[derive(Parser, Debug)]
pub struct InfoOptions {
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: SourceOptions,
}

/// Parses a duration from a timestamp like 30d, 2m.
fn human_friendly_duration(s: &str) -> Result<Duration, String> {
    let multiplier = if s.ends_with('d') {
        24 * 60 * 60
    } else if s.ends_with('m') {
        30 * 24 * 60 * 60
    } else if s.ends_with('y') {
        365 * 24 * 60 * 60
    } else {
        return Err(format!(
            "Invalid duration: «{}» should end with d, m or y.",
            s
        ));
    };
    let integer_part = match s.get(0..(s.len() - 1)) {
        Some(x) => x,
        None => return Err(format!("Invalid duration: «{}» has no integer part.", s)),
    };
    let n: Result<u64, std::num::ParseIntError> = integer_part.parse();
    match n {
        Ok(n) => Ok(Duration::from_secs(n * multiplier)),
        Err(e) => Err(format!(
            "Invalid duration: «{}» is not an integer: {}",
            integer_part, e
        )),
    }
}

#[test]
fn test_human_friendly_duration() {
    assert_eq!(
        human_friendly_duration("1d"),
        Ok(Duration::from_secs(24 * 60 * 60))
    );
    assert_eq!(
        human_friendly_duration("2d"),
        Ok(Duration::from_secs(2 * 24 * 60 * 60))
    );
    assert_eq!(
        human_friendly_duration("2m"),
        Ok(Duration::from_secs(2 * 30 * 24 * 60 * 60))
    );
    assert_eq!(
        human_friendly_duration("2y"),
        Ok(Duration::from_secs(2 * 365 * 24 * 60 * 60))
    );
    assert!(human_friendly_duration("1").is_err());
    assert!(human_friendly_duration("1dd").is_err());
    assert!(human_friendly_duration("dd").is_err());
    assert!(human_friendly_duration("d").is_err());
    assert!(human_friendly_duration("1j").is_err());
    assert!(human_friendly_duration("é").is_err());
}

/// Options for the `gc` subcommand.
#[derive(Parser, Debug)]
pub struct GcOptions {
    /// Machine readable output
    #[arg(long)]
    pub json: bool,

    #[command(subcommand)]
    /// Subcommand for lorri gc
    pub action: GcSubcommand,
}

#[derive(Subcommand, Debug)]
/// Subcommand for lorri gc
pub enum GcSubcommand {
    /// Prints the gc roots that lorri created.
    Info,
    /// Removes the gc roots associated to projects whose nix file vanished.
    Rm {
        /// Also delete the root associated with these shell files
        #[arg(long)]
        shell_file: Vec<PathBuf>,
        /// Delete the root of all projects
        #[arg(long)]
        all: bool,
        /// Also delete the root of projects that were last built before this amount of time, e.g. 30d.
        #[arg(long = "older-than", value_parser = clap::builder::ValueParser::new(human_friendly_duration))]
        older_than: Option<Duration>,
    },
}

/// Options for the `shell` subcommand.
#[derive(Parser, Debug)]
pub struct ShellOptions {
    // The source to build the environment from
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: DefaultingSourceOptions,

    /// If true, load environment from cache
    #[arg(long)]
    pub cached: bool,
}

/// Options for the `internal start-user-shell` subcommand.
#[derive(Parser, Debug)]
pub struct StartUserShellOptions_ {
    /// The path of the parent shell's binary
    #[arg(long)]
    pub shell_path: PathBuf,

    // The source to build the environment from
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: DefaultingSourceOptions,
}

/// Options for the `watch` subcommand.
#[derive(Parser, Debug)]
pub struct WatchOptions {
    // The source to build the environment from
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: DefaultingSourceOptions,
    /// Exit after a the first build
    #[arg(long)]
    pub once: bool,
}

/// Options for the `daemon` subcommand
#[derive(Parser, Debug)]
pub struct DaemonOptions {
    #[arg(long)]
    /// JSON value of nix config options to add.
    /// Only a subset is supported:
    /// {
    ///   "builders": <optional list of string>,
    ///   "substituters": <optional list of string>
    /// }
    pub extra_nix_options: Option<NixOptions>,
}

/// The nix options we can parse as json string
#[derive(Deserialize, Debug, Clone)]
// ATTN: If you modify this,
// adjust the help text in DaemonOptions.extra_nix_options
pub struct NixOptions {
    /// `builders` (see `nix::options::NixOptions`)
    pub builders: Option<Vec<String>>,
    /// `substituters` (see `nix::options::NixOptions`)
    pub substituters: Option<Vec<String>>,
}

impl FromStr for NixOptions {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// Sub-commands which lorri can execute for internal features
#[derive(Subcommand, Debug)]
pub enum Internal_ {
    /// (internal) Used internally by `lorri shell`
    StartUserShell_(StartUserShellOptions_),

    /// (plumbing) Tell the lorri daemon to care about the current directory's project
    Ping_(Ping_),

    /// (experimental) Ask the lorri daemon to report build events as they occur.
    ///
    /// This is intended for scripts. However, we don’t guarantee any stability for now,
    /// so if you want to use it in your scripts make sure you follow our changes.
    /// Once it stabilizes a bit more we will start mentioning changes in the changelog,
    /// and eventually ensure backwards compat.
    StreamEvents_(StreamEvents_),
}

/// Send a message with a lorri project.
///
/// Pinging with a project tells the daemon that the project was recently interacted with.
/// If the daemon has not been pinged for a project, it begins listening. If it does not
/// get pinged for a long time, it may stop watching the project for changes.
#[derive(Parser, Debug)]
pub struct Ping_ {
    #[allow(missing_docs)]
    #[command(flatten)]
    pub source: DefaultingSourceOptions,
}

/// Stream events from the daemon.
#[derive(Parser, Debug)]
pub struct StreamEvents_ {
    #[arg(long, default_value = "all")]
    /// The kind of events to report
    pub kind: crate::ops::EventKind,
}

/// A stub struct to represent how what we want to upgrade to.
#[derive(Parser, Debug)]
#[structopt(name = "basic")]
pub struct UpgradeTo {
    /// Where to upgrade to. If no subcommand given, `rolling-release` is assumed.
    #[command(subcommand)]
    pub source: Option<UpgradeSource>,
}

/// Version-specifiers of different upgrade targets.
#[derive(Subcommand, Debug)]
pub enum UpgradeSource {
    /// Upgrade to the current rolling-release version, will be
    /// fetched from git and built locally. rolling-release is
    /// expected to be more stable than canon. (default)
    RollingRelease,

    /// Upgrade to the current version from the canon (previously: master) branch,
    /// which will be fetched from git and built locally.
    Canon,

    /// Alias for `canon`.
    Master,

    /// Upgrade to the specified git branch, which will be fetched
    /// and built locally.
    Branch(BranchDest),

    /// Upgrade to a version in an arbitrary local directory.
    Local(LocalDest),
}

/// Install an arbitrary version of lorri from a local directory.
#[derive(Parser, Debug)]
pub struct LocalDest {
    /// the path to a local check out of lorri.
    #[arg(long)]
    pub path: PathBuf,
}

/// Install an arbitrary version of Lorri from an upstream git branch.
#[derive(Parser, Debug)]
pub struct BranchDest {
    /// the path to git branch of the upstream repository.
    pub branch: String,
}
