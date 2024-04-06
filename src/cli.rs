//! Defines the CLI interface using structopt.

// # Command line interface style guide
//
// Do not use short options unless they are extremely common and expected. A long option takes a
// bit more typing, but the long name makes the intent much more obvious. The only short option
// right now is `-v` for verbosity, and it should probably stay that way.
//
// See MAINTAINERS.md for details on internal and non-internal commands.

use std::{path::PathBuf, time::Duration};

#[derive(StructOpt, Debug)]
#[structopt(name = "lorri")]
/// Global arguments which set global program state. Most
/// arguments will be to sub-commands.
pub struct Arguments {
    /// Activate debug logging. Multiple occurrences are accepted for backwards compatibility, but
    /// have no effect. This will display all messages lorri logs.
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbosity: u8,

    /// Sub-command to execute
    #[structopt(subcommand)]
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

#[derive(StructOpt, Debug)]
/// Sub-commands which lorri can execute
pub enum Command {
    /// Emit shell script intended to be evaluated as part of direnv's .envrc, via: `eval "$(lorri
    /// direnv)"`
    #[structopt(name = "direnv")]
    Direnv(DirenvOptions),

    /// Remove lorri garbage collection roots that point to removed shell.nix files
    #[structopt(name = "gc")]
    Gc(GcOptions),

    /// Show information about a lorri project
    #[structopt(name = "info")]
    Info(InfoOptions),

    /// Open a new project shell
    #[structopt(name = "shell")]
    Shell(ShellOptions),

    /// Build project whenever an input file changes
    #[structopt(name = "watch")]
    Watch(WatchOptions),

    /// Start the multi-project daemon. Replaces `lorri watch`
    #[structopt(name = "daemon")]
    Daemon(DaemonOptions),

    /// Upgrade Lorri
    #[structopt(name = "self-upgrade", alias = "self-update")]
    Upgrade(UpgradeTo),

    /// Write bootstrap files to current directory to create a new lorri project
    #[structopt(name = "init")]
    Init,

    /// Internal commands, only use to experiment with unstable features
    #[structopt(name = "internal")]
    Internal {
        /// Sub-command to execute
        #[structopt(subcommand)]
        command: Internal_,
    },
}

/// Options for the `direnv` subcommand.
#[derive(StructOpt, Debug)]
pub struct DirenvOptions {
    /// The .nix file in the current directory to use
    #[structopt(long = "shell-file", parse(from_os_str), default_value = "shell.nix")]
    pub nix_file: PathBuf,
}

/// Options for the `info` subcommand.
#[derive(StructOpt, Debug)]
pub struct InfoOptions {
    /// The .nix file in the current directory to use
    ///
    /// If this option is not given, the `shell.nix` of the current directory is used.
    #[structopt(long = "shell-file", parse(from_os_str))]
    pub nix_file: Option<PathBuf>,
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
#[derive(StructOpt, Debug)]
pub struct GcOptions {
    /// Machine readable output
    #[structopt(long)]
    pub json: bool,

    #[structopt(subcommand)]
    /// Subcommand for lorri gc
    pub action: GcSubcommand,
}

#[derive(Debug, StructOpt)]
/// Subcommand for lorri gc
pub enum GcSubcommand {
    /// Prints the gc roots that lorri created.
    #[structopt(name = "info")]
    Info,
    /// Removes the gc roots associated to projects whose nix file vanished.
    #[structopt(name = "rm")]
    Rm {
        /// Also delete the root associated with these shell files
        #[structopt(long = "shell-file")]
        shell_file: Vec<PathBuf>,
        /// Delete the root of all projects
        #[structopt(long)]
        all: bool,
        /// Also delete the root of projects that were last built before this amount of time, e.g. 30d.
        #[structopt(long = "older-than", parse(try_from_str = "human_friendly_duration"))]
        older_than: Option<Duration>,
    },
}

/// Options for the `shell` subcommand.
#[derive(StructOpt, Debug)]
pub struct ShellOptions {
    /// The .nix file in the current directory to use
    #[structopt(long = "shell-file", parse(from_os_str), default_value = "shell.nix")]
    pub nix_file: PathBuf,
    /// If true, load environment from cache
    #[structopt(long = "cached")]
    pub cached: bool,
}

/// Options for the `internal start-user-shell` subcommand.
#[derive(StructOpt, Debug)]
pub struct StartUserShellOptions_ {
    /// The path of the parent shell's binary
    #[structopt(long = "shell-path", parse(from_os_str))]
    pub shell_path: PathBuf,
    /// The .nix file in the current directory to use to instantiate the project
    #[structopt(long = "shell-file", parse(from_os_str))]
    pub nix_file: PathBuf,
}

/// Options for the `watch` subcommand.
#[derive(StructOpt, Debug)]
pub struct WatchOptions {
    /// The .nix file in the current directory to use
    #[structopt(long = "shell-file", parse(from_os_str), default_value = "shell.nix")]
    pub nix_file: PathBuf,
    /// Exit after a the first build
    #[structopt(long = "once")]
    pub once: bool,
}

/// Options for the `daemon` subcommand
#[derive(StructOpt, Debug)]
pub struct DaemonOptions {
    #[structopt(
        long = "extra-nix-options",
        parse(try_from_str = "serde_json::from_str")
    )]
    /// JSON value of nix config options to add.
    /// Only a subset is supported:
    /// {
    ///   "builders": <optional list of string>,
    ///   "substituters": <optional list of string>
    /// }
    pub extra_nix_options: Option<NixOptions>,
}

/// The nix options we can parse as json string
#[derive(Deserialize, Debug)]
// ATTN: If you modify this,
// adjust the help text in DaemonOptions.extra_nix_options
pub struct NixOptions {
    /// `builders` (see `nix::options::NixOptions`)
    pub builders: Option<Vec<String>>,
    /// `substituters` (see `nix::options::NixOptions`)
    pub substituters: Option<Vec<String>>,
}

/// Sub-commands which lorri can execute for internal features
#[derive(StructOpt, Debug)]
pub enum Internal_ {
    /// (internal) Used internally by `lorri shell`
    #[structopt(name = "start-user-shell")]
    StartUserShell_(StartUserShellOptions_),

    /// (plumbing) Tell the lorri daemon to care about the current directory's project
    #[structopt(name = "ping")]
    Ping_(Ping_),

    /// (experimental) Ask the lorri daemon to report build events as they occur.
    ///
    /// This is intended for scripts. However, we don’t guarantee any stability for now,
    /// so if you want to use it in your scripts make sure you follow our changes.
    /// Once it stabilizes a bit more we will start mentioning changes in the changelog,
    /// and eventually ensure backwards compat.
    #[structopt(name = "stream-events")]
    StreamEvents_(StreamEvents_),
}

/// Send a message with a lorri project.
///
/// Pinging with a project tells the daemon that the project was recently interacted with.
/// If the daemon has not been pinged for a project, it begins listening. If it does not
/// get pinged for a long time, it may stop watching the project for changes.
#[derive(StructOpt, Debug)]
pub struct Ping_ {
    /// The .nix file to watch and build on changes.
    #[structopt(parse(from_os_str))]
    pub nix_file: PathBuf,
}

/// Stream events from the daemon.
#[derive(StructOpt, Debug)]
pub struct StreamEvents_ {
    #[structopt(long, default_value = "all")]
    /// The kind of events to report
    pub kind: crate::ops::EventKind,
}

/// A stub struct to represent how what we want to upgrade to.
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct UpgradeTo {
    /// Where to upgrade to. If no subcommand given, `rolling-release` is assumed.
    #[structopt(subcommand)]
    pub source: Option<UpgradeSource>,
}

/// Version-specifiers of different upgrade targets.
#[derive(StructOpt, Debug)]
pub enum UpgradeSource {
    /// Upgrade to the current rolling-release version, will be
    /// fetched from git and built locally. rolling-release is
    /// expected to be more stable than canon. (default)
    #[structopt(name = "rolling-release")]
    RollingRelease,

    /// Upgrade to the current version from the canon (previously: master) branch,
    /// which will be fetched from git and built locally.
    #[structopt(name = "canon")]
    Canon,

    /// Alias for `canon`.
    #[structopt(name = "master")]
    Master,

    /// Upgrade to the specified git branch, which will be fetched
    /// and built locally.
    #[structopt(name = "branch")]
    Branch(BranchDest),

    /// Upgrade to a version in an arbitrary local directory.
    #[structopt(name = "local")]
    Local(LocalDest),
}

/// Install an arbitrary version of lorri from a local directory.
#[derive(StructOpt, Debug)]
pub struct LocalDest {
    /// the path to a local check out of lorri.
    #[structopt(parse(from_os_str))]
    pub path: PathBuf,
}

/// Install an arbitrary version of Lorri from an upstream git branch.
#[derive(StructOpt, Debug)]
pub struct BranchDest {
    /// the path to git branch of the upstream repository.
    pub branch: String,
}
