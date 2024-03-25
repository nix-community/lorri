//! Builds a nix derivation file (like a `shell.nix` file).
//!
//! It is a wrapper around `nix-build`.
//!
//! Note: this does not build the Nix expression as-is.
//! It instruments various nix builtins in a way that we
//! can parse additional information from the `nix-build`
//! `stderr`, like which source files are used by the evaluator.

use crate::cas::ContentAddressable;
use crate::nix::{options::NixOptions, StorePath};
use crate::watch::WatchPathBuf;
use crate::{osstrlines, AbsDirPathBuf, Installable};
use crate::{DrvFile, NixFile};
use regex::Regex;
use slog::{debug, trace};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, BufReader};
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::{fmt, thread};

/// An error that can occur during a build.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BuildError {
    /// A system-level IO error occurred during the build.
    Io {
        /// Error message of the underlying error. Stored as a string because we need `BuildError`
        /// to implement `Copy`, but `io::Error` does not implement `Copy`.
        msg: String,
    },

    /// An error occurred while spawning a Nix process.
    ///
    /// Usually this means that the relevant Nix executable was not on the $PATH.
    Spawn {
        /// The command that failed. Stored as a string because we need `BuildError` to implement
        /// `Copy`, but `Command` does not implement `Copy`.
        cmd: String,

        /// Error message of the underlying error. Stored as a string because we need `BuildError`
        /// to implement `Copy`, but `io::Error` does not implement `Copy`.
        msg: String,
    },

    /// The Nix process returned with a non-zero exit code.
    Exit {
        /// The command that failed. Stored as a string because we need `BuildError` to implement
        /// `Copy`, but `Command` does not implement `Copy`.
        cmd: String,

        /// The `ExitStatus` of the command. The smart constructor `BuildError::exit` asserts that
        /// it is non-successful.
        status: Option<i32>,

        /// Error logs of the failed process.
        logs: Vec<LogLine>,
    },

    /// There was something wrong with the output of the Nix command.
    ///
    /// This error may for example indicate that the wrong number of outputs was produced.
    Output {
        /// Error message explaining the nature of the output error.
        msg: String,
    },
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> BuildError {
        BuildError::io(e)
    }
}

impl From<notify::Error> for BuildError {
    fn from(e: notify::Error) -> BuildError {
        BuildError::io(e)
    }
}

impl From<serde_json::Error> for BuildError {
    fn from(e: serde_json::Error) -> BuildError {
        BuildError::io(e)
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Io { msg } => write!(f, "I/O error: {}", msg),
            BuildError::Spawn { cmd, msg } => write!(
                f,
                "failed to spawn Nix process. Is Nix installed and on the $PATH?\n\
                 $ {}\n\
                 {}",
                cmd, msg,
            ),
            BuildError::Exit { cmd, status, logs } => write!(
                f,
                "Nix process returned exit code {}.\n\
                 $ {}\n\
                 {}",
                status.map_or("<unknown>".to_string(), |c| i32::to_string(&c)),
                cmd,
                LogLinesDisplay(logs)
            ),
            BuildError::Output { msg } => write!(f, "{}", msg),
        }
    }
}

// TODO: rethink these constructors
impl BuildError {
    /// Smart constructor for `BuildError::Io`
    pub fn io<D>(e: D) -> BuildError
    where
        D: fmt::Debug,
    {
        BuildError::Io {
            msg: format!("{:?}", e),
        }
    }

    /// Smart constructor for `BuildError::Spawn`
    pub fn spawn<D>(cmd: &Command, e: D) -> BuildError
    where
        D: fmt::Display,
    {
        BuildError::Spawn {
            cmd: format!("{:?}", cmd),
            msg: format!("{}", e),
        }
    }

    /// Smart constructor for `BuildError::Exit`
    pub fn exit(
        cmd: &Command,
        status: ExitStatus,
        logs: Vec<impl Into<LogLine> + Clone>,
    ) -> BuildError {
        assert!(
            !status.success(),
            "cannot create an exit error from a successful status code"
        );
        BuildError::Exit {
            cmd: format!("{:?}", cmd),
            status: status.code(),
            logs: logs.iter().map(|l| (*l).clone().into()).collect(),
        }
    }

    /// Smart constructor for `BuildError::Output`
    pub fn output(msg: String) -> BuildError {
        BuildError::Output { msg }
    }

    /// Is there something the user can do about this error?
    pub fn is_actionable(&self) -> bool {
        match self {
            BuildError::Io { .. } => false,
            BuildError::Spawn { .. } => true, // install Nix or fix $PATH
            BuildError::Exit { .. } => true,  // fix Nix expression
            BuildError::Output { .. } => true, // fix Nix expression
        }
    }
}

/// A line from stderr log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine(pub OsString);

impl From<LogDatum> for LogLine {
    fn from(dbg: LogDatum) -> LogLine {
        LogLine(format!("{dbg:?}").into())
    }
}

impl From<OsString> for LogLine {
    fn from(oss: OsString) -> Self {
        LogLine(oss)
    }
}

impl From<String> for LogLine {
    fn from(s: String) -> Self {
        LogLine(OsString::from(s))
    }
}

struct LogLinesDisplay<'a>(&'a [LogLine]);

impl<'a> fmt::Display for LogLinesDisplay<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for l in self.0 {
            let mut s = String::from_utf8_lossy(l.0.as_bytes()).into_owned();
            s.push('\n');
            formatter.write_str(&s)?;
        }
        Ok(())
    }
}

struct RootedDrv {
    _gc_handle: GcRootTempDir,
    path: DrvFile,
}

/// Represents a path which is temporarily rooted in a temporary directory.
/// Users are required to keep the gc_handle value alive for as long as
/// StorePath is alive, _or_ re-root the StorePath using project::Roots
/// before dropping gc_handle.
#[derive(Debug)]
pub struct RootedPath {
    /// The handle to a temporary directory keeping `.path` alive.
    pub gc_handle: crate::nix::GcRootTempDir,
    /// The realized store path
    pub path: StorePath,
    /// Other GC root files that need pinning
    pub extra_paths: Vec<StorePath>,
}

struct BuildOutput {
    output: RootedPath,
}

/// The result of a single instantiation and build.
#[derive(Debug)]
pub struct RunResult {
    /// All the paths identified during the instantiation
    pub referenced_paths: Vec<WatchPathBuf>,
    /// The primary result of the build run
    pub result: RootedPath, // XXX split out the tempdir handle as separate field
}

struct InstantiateOutput {
    referenced_paths: Vec<WatchPathBuf>,
    output: RootedDrv,
}

fn instrumented_instantiation(
    nix_file: &NixFile,
    cas: &ContentAddressable,
    extra_nix_options: &NixOptions,
    logger: &slog::Logger,
) -> Result<InstantiateOutput, BuildError> {
    // We're looking for log lines matching:
    //
    //     copied source '...' -> '/nix/store/...'
    //     evaluating file '...'
    //
    // to determine which files we should setup watches on.
    // Increasing verbosity by two levels via `-vv` satisfies that.

    let mut cmd = Command::new("nix-instantiate");

    let logged_evaluation_nix = cas.file_from_string(include_str!("./logged-evaluation.nix"))?;

    // TODO: see ::nix::CallOpts::paths for the problem with this
    let gc_root_dir = tempfile::TempDir::new()?;

    cmd.args([
        // verbose mode prints the files we track
        OsStr::new("-vv"),
    ]);
    // put the passed extra options at the front
    // to make them more visible in traces
    cmd.args(extra_nix_options.to_nix_arglist());
    cmd.args([
        // we add a temporary indirect GC root
        OsStr::new("--add-root"),
        gc_root_dir.path().join("result").as_os_str(),
        OsStr::new("--indirect"),
        OsStr::new("--argstr"),
        // runtime nix paths to needed dependencies that come with lorri
        OsStr::new("runTimeClosure"),
        OsStr::new(crate::RUN_TIME_CLOSURE),
        // the source file
        OsStr::new("--argstr"),
    ]);
    cmd.args([OsStr::new("src"), nix_file.as_absolute_path().as_os_str()]);
    cmd.args([
        // instrumented by `./logged-evaluation.nix`
        OsStr::new("--"),
        logged_evaluation_nix.as_path().as_os_str(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    debug!(logger, "nix-instantiate"; "command" => ?cmd, "line" => ?line!());

    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => BuildError::spawn(&cmd, e),
        _ => BuildError::io(e),
    })?;

    let stdout = child
        .stdout
        .take()
        .expect("we must be able to access the stdout of nix-instantiate");
    let stderr = child
        .stderr
        .take()
        .expect("we must be able to access the stderr of nix-instantiate");

    let stderr_results = thread::spawn(move || {
        osstrlines::Lines::from(BufReader::new(stderr))
            .map(|line| line.map(parse_evaluation_line))
            .collect::<Result<Vec<LogDatum>, _>>()
    });

    let build_products = thread::spawn(move || {
        osstrlines::Lines::from(BufReader::new(stdout))
            .map(|line| line.map(|os_string| DrvFile::from(PathBuf::from(os_string))))
            .collect::<Result<Vec<DrvFile>, _>>()
    });

    let (exec_result, mut build_products, results) = (
        child.wait()?,
        build_products
            .join()
            .expect("Failed to join stdout processing thread")?,
        stderr_results
            .join()
            .expect("Failed to join stderr processing thread")?,
    );

    // TODO: this can move entirely into the stderr thread,
    // meaning we don’t have to keep the outputs in memory (fold directly)

    if !exec_result.success() {
        return Err(BuildError::exit(&cmd, exec_result, results));
    }

    let paths = extract_paths(results);

    let shell_gc_root = match build_products.len() {
        0 => panic!("logged_evaluation.nix did not return a build product."),
        1 => build_products.pop().unwrap(),
        n => panic!(
            "got more than one build product ({}) from logged_evaluation.nix: {:#?}",
            n, build_products
        ),
    };

    Ok(InstantiateOutput {
        referenced_paths: paths,
        output: RootedDrv {
            _gc_handle: GcRootTempDir(gc_root_dir),
            path: shell_gc_root,
        },
    })
}

/// Builds the Nix expression in `root_nix_file`.
///
/// Instruments the nix file to gain extra information, which is valuable even if the build fails.
fn build(drv_path: DrvFile, logger: &slog::Logger) -> Result<BuildOutput, BuildError> {
    let (path, gc_handle) = crate::nix::CallOpts::file(drv_path.as_path()).path(logger)?;
    Ok(BuildOutput {
        output: RootedPath {
            gc_handle,
            path,
            extra_paths: vec![],
        },
    })
}

/// Builds the Nix expression in `root_nix_file`.
///
/// Instruments the nix file to gain extra information,
/// which is valuable even if the build fails.
pub fn run(
    root_nix_file: &NixFile,
    cas: &ContentAddressable,
    extra_nix_options: &NixOptions,
    logger: &slog::Logger,
) -> Result<RunResult, BuildError> {
    let inst_info = instrumented_instantiation(root_nix_file, cas, extra_nix_options, logger)?;
    let buildoutput = build(inst_info.output.path, logger)?;
    Ok(RunResult {
        referenced_paths: inst_info.referenced_paths,
        result: buildoutput.output,
    })
}

/// Execute a command (presumably a Nix command :)). stderr output
/// is passed line-based to the CallOpts' stderr_line_tx receiver.
/// Stdout is passed as a BufReader to `stdout_fn`.
fn execute<OF: 'static, EF: 'static, O: 'static>(
    nickname: &str,
    mut cmd: Command,
    logger: &slog::Logger,
    stdout_fn: OF,
    stderr_fn: EF,
) -> Result<(O, Vec<LogDatum>), BuildError>
where
    OF: Send + FnOnce(ChildStdout) -> O,
    O: Send,
    EF: Send + FnOnce(ChildStderr) -> Result<Vec<LogDatum>, io::Error>,
{
    cmd.stderr(Stdio::piped());
    cmd.stdout(Stdio::piped());

    debug!(logger, "{}", nickname; "command" => ?cmd, "dir" => ?cmd.get_current_dir());
    // 0. spawn the process
    let mut nix_proc = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => BuildError::spawn(&cmd, e),
        _ => BuildError::io(e),
    })?;

    // 1. spawn a stderr handling thread
    let stderr_handle = nix_proc.stderr.take().expect("failed to take stderr");
    let stderr_thread = thread::spawn(move || stderr_fn(stderr_handle));

    // 2. spawn a stdout handling thread (?)
    let stdout_handle = nix_proc.stdout.take().expect("failed to take stdout");
    let stdout_thread = thread::spawn(move || stdout_fn(stdout_handle));

    // 3. wait on the process
    let nix_proc_result = nix_proc.wait()?;

    debug!(logger, "(complete) {}", nickname; "command" => ?cmd, "result" => ?nix_proc_result);

    // 4. join the stderr handler
    let stderr_result = stderr_thread
        .join()
        .expect("stderr handling thread panicked")?;

    // 5. join the stdout handler
    let stdout_result = stdout_thread
        .join()
        .expect("stderr handling thread panicked");

    if !nix_proc_result.success() {
        Err(BuildError::exit(&cmd, nix_proc_result, stderr_result))
    } else {
        Ok((stdout_result, stderr_result))
    }
}

/// Builds the devShell of a flake
pub fn flake(installable: &Installable, logger: &slog::Logger) -> Result<RunResult, BuildError> {
    let gc_root_dir = tempfile::TempDir::new()?;

    let env_path = gc_root_dir.path().join("bash-export");
    let profile_path = gc_root_dir.path().join("profile");

    let mut cmd = Command::new("nix");
    cmd.current_dir(installable.context.as_path());
    cmd.args([
        OsStr::new("develop"),
        OsStr::new("--debug"),
        OsStr::new("--profile"),
        profile_path.as_path().as_ref(),
        OsStr::new(&installable.installable),
        OsStr::new("-c"), // nix develop, please run ..
        OsStr::new("bash"),
        OsStr::new("-c"), // and bash, you as well, please run...
        OsStr::new("export"),
    ])
    .stdin(Stdio::null());

    let build_env_path = env_path.clone();
    let l2 = logger.clone();
    let (_, results) = execute(
        "nix develop",
        cmd,
        logger,
        move |mut stdout| -> Result<u64, _> {
            let mut f = File::create(build_env_path)?;
            io::copy(&mut stdout, &mut f)
        },
        move |stderr| {
            let mut parser = NixDevParser::new(l2);
            osstrlines::Lines::from(BufReader::new(stderr))
                .map(|line| line.map(|i| parser.parse(i)))
                .collect::<Result<Vec<LogDatum>, _>>()
        },
    )?;

    let referenced_paths = extract_paths(results);

    let mut profile_root = profile_path;
    for _ in 1..10 {
        debug!(logger, "follow symlink"; "path" => ?profile_root, "metadata" => ?std::fs::symlink_metadata(&profile_root));
        if !std::fs::symlink_metadata(&profile_root)?.is_symlink() {
            break;
        }
        let profile_dir = profile_root.parent().expect("never to be /");
        profile_root = profile_dir.join(std::fs::read_link(&profile_root)?);
    }
    debug!(logger, "profile resolved"; "path" => ?profile_root);

    let mut cmd = Command::new("nix");
    cmd.current_dir(installable.context.as_path());
    cmd.args([
        OsStr::new("store"),
        OsStr::new("add-file"), // XXX deprecated for add --mode flat
        env_path.as_os_str(),
    ])
    .stdin(Stdio::null());

    let (store_paths, _) = execute(
        "nix store add",
        cmd,
        logger,
        move |stdout| {
            osstrlines::Lines::from(BufReader::new(stdout))
                .map(|line| line.map(PathBuf::from))
                .collect::<Result<Vec<_>, _>>()
        },
        move |stderr| {
            osstrlines::Lines::from(BufReader::new(stderr))
                .map(|line| line.map(|t| LogDatum::Text(t.to_string_lossy().into())))
                .collect::<Result<Vec<LogDatum>, _>>()
        },
    )?;

    let store_path = store_paths?
        .first()
        .ok_or_else(|| BuildError::output("nix store add: no store path reported".into()))?
        .clone();

    let result = RootedPath {
        gc_handle: gc_root_dir.into(),
        path: StorePath::from(store_path),
        extra_paths: vec![profile_root.into()],
    };

    Ok(RunResult {
        referenced_paths,
        result,
    })
}

fn extract_paths(results: impl IntoIterator<Item = LogDatum>) -> Vec<WatchPathBuf> {
    results
        .into_iter()
        .filter_map(|result| match result {
            LogDatum::CopiedSource(src) | LogDatum::ReadRecursively(src) => {
                Some(WatchPathBuf::Recursive(src))
            }
            LogDatum::ReadDir(src) => Some(WatchPathBuf::Normal(src)),
            LogDatum::NixSourceFile(mut src) => {
                // We need to emulate nix’s `default.nix` mechanism here.
                // That is, if the user uses something like
                // `import ./foo`
                // and `foo` is a directory, nix will actually import
                // `./foo/default.nix`
                // but still print `./foo`.
                // Since this is the only time directories are printed,
                // we can just manually re-implement that behavior.
                if src.is_dir() {
                    src.push("default.nix");
                }
                Some(WatchPathBuf::Normal(src))
            }
            LogDatum::Text(_) | LogDatum::NonUtf(_) => None,
        })
        .collect()
}

/// Classifies the output of nix-instantiate -vv.
#[derive(Clone, Debug, PartialEq)]
enum LogDatum {
    /// Nix source file (which should be tracked)
    NixSourceFile(PathBuf),
    /// A file/directory copied verbatim to the nix store
    CopiedSource(PathBuf),
    /// A `builtins.readFile` or `builtins.filterSource` invocation (at eval time)
    /// Means this subtree should be recursively watched.
    ReadRecursively(PathBuf),
    /// A `builtins.readDir` invocation (at eval time).
    /// The subtree must not be recursively watched, only the file listing of the directory.
    ReadDir(PathBuf),
    /// Arbitrary text (which we couldn’t otherwise classify)
    Text(String),
    /// Text which we coudn’t decode from UTF-8
    NonUtf(OsString),
}

/// Examine a line of output and extract interesting log items in to
/// structured data.
fn parse_evaluation_line<T>(line: T) -> LogDatum
where
    T: AsRef<OsStr>,
{
    lazy_static::lazy_static! {
        // These are the .nix files that are opened for evaluation.
        static ref EVAL_FILE: Regex =
            Regex::new("^evaluating file '(?P<source>.*)'$").expect("invalid regex!");
        // When you reference a source file, nix copies it to the store and prints this.
        // This the same is true for directories (e.g. `foo = ./abc` in a derivation).
        static ref COPIED_SOURCE: Regex =
            Regex::new("^copied source '(?P<source>.*)' -> '(?:.*)'$").expect("invalid regex!");
        // These are printed for `builtins.readFile` and `builtins.filterSource`,
        // by our instrumentation in `./logged-evaluation.nix`.
        // They mean we should watch recursively this file or directory
        static ref LORRI_READ: Regex =
            Regex::new("^trace: lorri read: '(?P<source>.*)'$").expect("invalid regex!");
        // These are printed for `builtins.readDir` and `builtins.filterSource`,
        // by our instrumentation in `./logged-evaluation.nix`.
        // They mean we should watch the file listing of this directory, but not
        // its children.
        static ref LORRI_READDIR: Regex =
            Regex::new("^trace: lorri readdir: '(?P<source>.*)'$").expect("invalid regex!");
    }

    // see the regexes above for explanations of the nix outputs
    match line.as_ref().to_str() {
        // If we can’t decode the output line to an UTF-8 string,
        // we cannot match against regexes, so just pass it through.
        None => LogDatum::NonUtf(line.as_ref().to_owned()),
        Some(linestr) => {
            // Lines about evaluating a file are much more common, so looking
            // for them first will reduce comparisons.
            if let Some(matches) = EVAL_FILE.captures(linestr) {
                LogDatum::NixSourceFile(PathBuf::from(&matches["source"]))
            } else if let Some(matches) = COPIED_SOURCE.captures(linestr) {
                LogDatum::CopiedSource(PathBuf::from(&matches["source"]))
            } else if let Some(matches) = LORRI_READ.captures(linestr) {
                LogDatum::ReadRecursively(PathBuf::from(&matches["source"]))
            } else if let Some(matches) = LORRI_READ.captures(linestr) {
                LogDatum::ReadDir(PathBuf::from(&matches["source"]))
            } else {
                LogDatum::Text(linestr.to_owned())
            }
        }
    }
}

struct NixDevParser {
    flake_rees: HashMap<String, (Regex, AbsDirPathBuf)>,
    tree_rees: HashMap<String, (Regex, AbsDirPathBuf)>,
    logger: slog::Logger,
}

impl NixDevParser {
    fn new(logger: slog::Logger) -> Self {
        let flake_rees = HashMap::new();
        let tree_rees = HashMap::new();
        Self {
            flake_rees,
            tree_rees,
            logger,
        }
    }
    /// Examine a line of output and extract interesting log items in to
    /// structured data.
    fn parse<T>(&mut self, line: T) -> LogDatum
    where
        T: AsRef<OsStr>,
    {
        // see the regexes above for explanations of the nix outputs
        match line.as_ref().to_str() {
            // If we can’t decode the output line to an UTF-8 string,
            // we cannot match against regexes, so just pass it through.
            None => LogDatum::NonUtf(line.as_ref().to_owned()),
            Some(linestr) => self.parse_str(linestr),
        }
    }

    fn parse_str(&mut self, line: &str) -> LogDatum {
        use regex::escape;

        trace!(self.logger, "parsing"; "line" => ?line, "flake_res" => ?self.flake_rees.len(), "tree_res" => ?self.tree_rees.len());
        // evaluating derivation 'git+file:///home/judson/dev/picklist#devShells.x86_64-linux.default'...
        // got tree '/nix/store/47b9j9bnhi1n7larnn6wy40xcyfbz9mr-source' from 'git+file:///home/judson/dev/picklist'
        // checking access to '/nix/store/47b9j9bnhi1n7larnn6wy40xcyfbz9mr-source/flake.nix'
        lazy_static::lazy_static! {
            static ref EVAL_DRV: Regex = Regex::new(r#"evaluating derivation '(?P<flake>git\+file://(?P<path>[^?]*)[^#]*)#\S*'\.\.\."#).expect("regex to compile!");
        }

        for (re, path) in self.tree_rees.values() {
            if let Some(matches) = re.captures(line) {
                debug!(self.logger, "tree match"; "re" => ?re, "line" => ?line, "matches" => ?matches);
                return LogDatum::ReadRecursively(
                    path.relative_to(PathBuf::from(&matches["file"]))
                        .expect("paths to join")
                        .as_path()
                        .to_path_buf(),
                );
            }
        }
        for (re, path) in self.flake_rees.values() {
            if let Some(matches) = re.captures(line) {
                debug!(self.logger, "flake match"; "re" => ?re, "line" => ?line, "matches" => ?matches);
                let nre = Regex::new(&format!(
                    "checking access to '{}/(?P<file>[^']*)'",
                    escape(&matches["tree"])
                ))
                .expect("tree regex to compile!");
                let tree_name = matches["tree"].to_string();
                debug!(self.logger, "adding tree RE"; "name" => ?tree_name, "re" => ?re);
                self.tree_rees.insert(tree_name, (nre, path.clone()));
                return LogDatum::Text(line.to_string());
            }
        }
        // Lines about evaluating a file are much more common, so looking
        // for them first will reduce comparisons.
        if let Some(matches) = EVAL_DRV.captures(line) {
            debug!(self.logger, "drv match"; "line" => ?line, "matches" => ?matches);
            let re = Regex::new(&format!(
                "got tree '(?P<tree>[^']*)' from '{}'",
                escape(&matches["flake"])
            ))
            .expect("flake regex to compile");
            let flake_name = matches["flake"].to_string();
            let source_path = matches["path"].to_string();
            debug!(self.logger, "adding flake RE"; "name" => ?flake_name, "path" => ?source_path, "re" => ?re);
            self.flake_rees.insert(
                flake_name,
                (
                    re,
                    AbsDirPathBuf::new(source_path.into())
                        .expect("flake name to include absolute path"),
                ),
            );
        }
        LogDatum::Text(line.to_string())
    }
}

/// Output path generated by `logged-evaluation.nix`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPath<T> {
    /// Shell path modified to work as a gc root
    pub shell_gc_root: T,
}

impl<T> OutputPath<T> {
    /// map over the inner type.
    pub fn map<F, T2>(self, f: F) -> OutputPath<T2>
    where
        F: Fn(T) -> T2,
    {
        OutputPath {
            shell_gc_root: f(self.shell_gc_root),
        }
    }
}

/// Opaque type to keep a temporary GC root directory alive.
/// Once it is dropped, the GC root is removed.
/// Copied from `nix`, because the type should stay opaque.
#[derive(Debug)]
struct GcRootTempDir(tempfile::TempDir);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cas::ContentAddressable;
    use crate::nix::options::NixOptions;
    use crate::AbsPathBuf;
    use std::path::PathBuf;

    /// Parsing of `LogDatum`.
    #[test]
    fn evaluation_line_to_log_datum() {
        assert_eq!(
                parse_evaluation_line("evaluating file '/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/lib/systems/for-meta.nix'"),
                LogDatum::NixSourceFile(PathBuf::from("/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/lib/systems/for-meta.nix"))
            );

        assert_eq!(
                parse_evaluation_line("copied source '/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/pkgs/stdenv/generic/default-builder.sh' -> '/nix/store/9krlzvny65gdc8s7kpb6lkx8cd02c25b-default-builder.sh'"),
                LogDatum::CopiedSource(PathBuf::from("/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/pkgs/stdenv/generic/default-builder.sh"))
            );

        assert_eq!(
            parse_evaluation_line(
                "trace: lorri read: '/home/grahamc/projects/grahamc/lorri/nix/nixpkgs.json'"
            ),
            LogDatum::ReadRecursively(PathBuf::from(
                "/home/grahamc/projects/grahamc/lorri/nix/nixpkgs.json"
            ))
        );

        assert_eq!(
            parse_evaluation_line(
                "downloading 'https://static.rust-lang.org/dist/channel-rust-stable.toml'..."
            ),
            LogDatum::Text(String::from(
                "downloading 'https://static.rust-lang.org/dist/channel-rust-stable.toml'..."
            ))
        );
    }

    /// Create a locally built base derivation expression.
    /// `args` is just interpolated into the derivation fields.
    fn drv(name: &str, args: &str) -> String {
        format!(
            r##"
derivation {{
  name = "{}";
  builder = "/bin/sh";
  allowSubstitutes = false;
  preferLocalBuild = true;
  system = builtins.currentSystem;
  # this is to make nix rebuild for every test
  random = builtins.currentTime;
  {}
}}"##,
            name, args
        )
    }

    /// Some nix builds can output non-UTF-8 encoded text
    /// (arbitrary binary output). We should not crash in that case.
    #[test]
    fn non_utf8_nix_output() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let cas = ContentAddressable::new(crate::AbsPathBuf::new(tmp.path().to_owned()).unwrap())?;

        let inner_drv = drv(
            "dep",
            r##"
args = [
    "-c"
    ''
    # non-utf8 sequence to stdout (which is nix stderr)
    printf '"\xab\xbc\xcd\xde\xde\xef"'
    echo > $out
    ''
];"##,
        );

        let nix_drv = format!(
            r##"
let dep = {};
in {}
"##,
            inner_drv,
            drv("shell", "inherit dep;")
        );

        print!("{}", nix_drv);

        // build, because instantiate doesn’t return the build output (obviously …)
        run(
            &crate::NixFile::from(cas.file_from_string(&nix_drv)?),
            &cas,
            &NixOptions::empty(),
            &crate::logging::test_logger(),
        )
        .expect("should not crash!");
        Ok(())
    }

    /// If the build fails, we shouldn’t crash in the process.
    #[test]
    fn gracefully_handle_failing_build() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let cas = ContentAddressable::new(crate::AbsPathBuf::new(tmp.path().to_owned()).unwrap())?;

        let d = crate::NixFile::from(cas.file_from_string(&drv(
            "shell",
            &format!("dep = {};", drv("dep", r##"args = [ "-c" "exit 1" ];"##)),
        ))?);

        if let Err(BuildError::Exit { .. }) = run(
            &d,
            &cas,
            &NixOptions::empty(),
            &crate::logging::test_logger(),
        ) {
        } else {
            panic!("builder::run should have failed with BuildError::Exit");
        }
        Ok(())
    }

    // TODO: builtins.fetchTarball and the like? What happens with those?
    // Are they directories and if yes, should we watch them?
    /// The paths that are returned by the nix-instantiate call
    /// must not contain directories, otherwise the watcher will
    /// watch those recursively, which leads to a lot of wasted resources
    /// and often exhausts the amount of available file handles
    /// (especially on macOS).
    #[test]
    fn no_unnecessary_files_or_directories_watched() -> std::io::Result<()> {
        let root_tmp = tempfile::tempdir()?;
        let cas_tmp = tempfile::tempdir()?;
        let root = root_tmp.path();
        let shell = root.join("shell.nix");
        std::fs::write(
            &shell,
            drv(
                "shell",
                r##"
# The `foo/default.nix` is implicitely imported
# (we only want to watch that one, not the whole directory)
foo = import ./foo;
# `dir` is imported as source directory (no `import`).
# We *do* want to watch this directory, because we need to react
# when the user updates it.
dir-as-source = ./dir;
"##,
            ),
        )?;

        // ./foo
        // ./foo/default.nix
        // ./foo/bar <- should not be watched
        // ./foo/baz <- should be watched
        // ./dir <- should be watched, because imported as source
        let foo = root.join("foo");
        std::fs::create_dir(&foo)?;
        let dir = root.join("dir");
        std::fs::create_dir(dir)?;
        let foo_default = &foo.join("default.nix");
        std::fs::write(foo_default, "import ./baz")?;
        let foo_bar = &foo.join("bar");
        std::fs::write(foo_bar, "This file should not be watched")?;
        let foo_baz = &foo.join("baz");
        std::fs::write(foo_baz, "\"This file should be watched\"")?;

        let cas =
            ContentAddressable::new(crate::AbsPathBuf::new(cas_tmp.path().join("cas")).unwrap())?;

        let inst_info = instrumented_instantiation(
            &NixFile::from(AbsPathBuf::new(shell).unwrap()),
            &cas,
            &NixOptions::empty(),
            &crate::logging::test_logger(),
        )
        .unwrap();
        let ends_with = |end| {
            inst_info
                .referenced_paths
                .iter()
                .any(|p| p.as_ref().ends_with(end))
        };
        assert!(
            ends_with("foo/default.nix"),
            "foo/default.nix should be watched!"
        );
        assert!(!ends_with("foo/bar"), "foo/bar should not be watched!");
        assert!(ends_with("foo/baz"), "foo/baz should be watched!");
        assert!(ends_with("dir"), "dir should be watched!");
        assert!(
            !ends_with("foo"),
            "No imported directories must exist in watched paths: {:#?}",
            inst_info.referenced_paths
        );
        Ok(())
    }
}
