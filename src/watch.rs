//! Recursively watch paths for changes, in an extensible and
//! cross-platform way.

use crossbeam_channel as chan;
use notify::event::ModifyKind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use slog::{debug, info};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Represents if a path to watch should be watched recursively by the watcher or not
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum WatchPathBuf {
    /// This path should be watched recursively. Equivalent to Normal for non-directory.
    Recursive(PathBuf),
    /// This path should not be watched recursively. For directories, only the list of files is
    /// watched.
    Normal(PathBuf),
}

impl AsRef<Path> for WatchPathBuf {
    fn as_ref(&self) -> &Path {
        match self {
            WatchPathBuf::Recursive(path) => path.as_ref(),
            WatchPathBuf::Normal(path) => path.as_ref(),
        }
    }
}

impl AsMut<PathBuf> for WatchPathBuf {
    fn as_mut(&mut self) -> &mut PathBuf {
        match self {
            WatchPathBuf::Recursive(ref mut path) => path,
            WatchPathBuf::Normal(ref mut path) => path,
        }
    }
}

impl WatchPathBuf {
    /// Create a new WatchPathBuf of the same variant, but with this PathBuf instead.
    pub fn replace(&self, path: PathBuf) -> WatchPathBuf {
        match self {
            WatchPathBuf::Normal(_) => WatchPathBuf::Normal(path),
            WatchPathBuf::Recursive(_) => WatchPathBuf::Recursive(path),
        }
    }
}

/// A dynamic list of paths to watch for changes, and
/// react to changes when they occur.
pub struct Watch {
    /// Event receiver. Process using `Watch::process`.
    pub rx: chan::Receiver<notify::Result<notify::Event>>,
    notify: RecommendedWatcher,
    watches: HashSet<PathBuf>,
    logger: slog::Logger,
}

/// A debug message string that can only be displayed via `Debug`.
#[derive(Clone, Debug, Serialize)]
pub struct DebugMessage(pub String);

#[derive(Debug, PartialEq, Eq)]
struct FilteredOut<'a> {
    reason: &'a str,
    path: PathBuf,
}

impl Watch {
    /// Instantiate a new Watch.
    pub fn try_new(logger: slog::Logger) -> Result<Watch, notify::Error> {
        let (tx, rx) = chan::unbounded();

        Ok(Watch {
            notify: Watcher::new(tx, Duration::from_millis(100))?,
            watches: HashSet::new(),
            rx,
            logger,
        })
    }

    /// Process `notify::Event`s coming in via `Watch::rx`.
    ///
    /// Returns a list of „interesting“ paths.
    ///
    /// `None` if there were no relevant changes.
    pub fn process_watch_events(
        &self,
        event: notify::Result<notify::Event>,
    ) -> Option<Vec<PathBuf>> {
        match event {
            Ok(event) => {
                {
                    let event = &event;
                    debug!(self.logger, "Watch Event: {:#?}", event);
                    match &event.kind {
                        notify::event::EventKind::Remove(_) if !event.paths.is_empty() => {
                            info!(self.logger, "identified removal: {:?}", &event.paths);
                        }
                        _ => {
                            debug!(self.logger, "watch event"; "event" => ?event);
                        }
                    }
                };
                let notify::Event { paths, kind, .. } = event;
                let interesting_paths: Vec<PathBuf> = paths
                    .into_iter()
                    .filter(|path| {
                        path_match(&self.watches, path, &self.logger)
                            && match kind {
                                // We ignore metadata modification events for the profiles directory
                                // tree as it is a symlink forest that is used to keep track of
                                // channels and nix will uconditionally update the metadata of each
                                // link in this forest. See https://github.com/NixOS/nix/blob/629b9b0049363e091b76b7f60a8357d9f94733cc/src/libstore/local-store.cc#L74-L80
                                // for the unconditional update. These metadata modification events are
                                // spurious annd they can easily cause a rebuild-loop when a shell.nix
                                // file does not pin its version of nixpkgs or other channels. When
                                // a Nix channel is updated we receive many other types of events, so
                                // ignoring these metadata modifications will not impact lorri's
                                // ability to correctly watch for channel changes.
                                EventKind::Modify(ModifyKind::Metadata(_)) => {
                                    if path.starts_with(Path::new("/nix/var/nix/profiles/per-user")) {
                                        debug!(self.logger, "ignoring spurious metadata change event within the profiles dir"; "path" => path.to_str());
                                        false
                                    } else {
                                        true
                                    }
                                }
                                _ => true,
                            }
                    })
                    .collect();
                match interesting_paths.is_empty() {
                    true => None,
                    false => Some(interesting_paths),
                }
            }
            Err(err) => {
                slog::warn!(self.logger, "notify library threw error: {}", err);
                None
            }
        }
    }

    /// Extend the watch list with an additional list of paths.
    /// Note: Watch maintains a list of already watched paths, and
    /// will not add duplicates.
    pub fn extend(&mut self, paths: Vec<WatchPathBuf>) -> Result<(), notify::Error> {
        for path in paths {
            let recursive_paths = match path {
                WatchPathBuf::Recursive(path) => walk_path_topo(path)?,
                WatchPathBuf::Normal(path) => vec![path],
            };
            for p in recursive_paths {
                let p = p.canonicalize()?;
                match Self::extend_filter(p) {
                    Err(FilteredOut { reason, path }) => {
                        debug!(
                            self.logger,
                            "Skipping watching {}: {}",
                            path.display(),
                            reason
                        )
                    }
                    Ok(p) => {
                        self.add_path(p)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn extend_filter(path: PathBuf) -> Result<PathBuf, FilteredOut<'static>> {
        if path.starts_with(Path::new("/nix/store")) {
            Err(FilteredOut {
                path,
                reason: "starts with /nix/store",
            })
        } else {
            Ok(path)
        }
    }

    fn add_path(&mut self, path: PathBuf) -> Result<(), notify::Error> {
        if !self.watches.contains(&path) {
            debug!(self.logger, "watching path"; "path" => path.to_str());

            self.notify.watch(&path, RecursiveMode::NonRecursive)?;
            self.watches.insert(path.clone());
        }

        if let Some(parent) = path.parent() {
            if !self.watches.contains(parent) {
                debug!(self.logger, "watching parent path"; "parent_path" => parent.to_str());

                self.notify.watch(parent, RecursiveMode::NonRecursive)?;
            }
        }

        Ok(())
    }
}

/// Lists the dirs and files in a directory, as two vectors.
/// Given path must be a readable directory.
fn list_dir(dir: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::io::Error> {
    let mut dirs = vec![];
    let mut files = vec![];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path())
        } else {
            files.push(entry.path())
        }
    }
    Ok((dirs, files))
}

/// List all children of the given path.
/// Recurses into directories.
///
/// Returns the given path first, then a topologically sorted list of children, if any.
///
/// All files have to be readable, or the function aborts.
/// TODO: gracefully skip unreadable files.
fn walk_path_topo(path: PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
    // push our own path first
    let mut res = vec![path.clone()];

    // nothing to list
    if !path.is_dir() {
        return Ok(res);
    }

    let (dirs, mut files) = list_dir(&path)?;
    // plain files
    res.append(&mut files);

    // now to go through the list, appending new
    // directories to the work queue as you find them.
    let mut work = std::collections::VecDeque::from(dirs);
    loop {
        match work.pop_front() {
            // no directories remaining
            None => break,
            Some(dir) => {
                res.push(dir.clone());
                let (dirs, mut files) = list_dir(&dir)?;
                res.append(&mut files);
                work.append(&mut std::collections::VecDeque::from(dirs));
            }
        }
    }
    Ok(res)
}

/// Determine if the event path is covered by our list of watched
/// paths.
///
/// Returns true if:
///   - the event's path directly names a path in our
///     watch list
///   - the event's path names a canonicalized path in our watch list
///   - the event's path's parent directly names a path in our watch
///     list
///   - the event's path's parent names a canonicalized path in our
///     watch list
fn path_match(watched_paths: &HashSet<PathBuf>, event_path: &Path, logger: &slog::Logger) -> bool {
    let event_parent = event_path.parent();

    let matches = |watched: &Path| {
        if event_path == watched {
            debug!(
                logger,
                "event path directly matches watched path";
                "event_path" => event_path.to_str());

            return true;
        }

        if let Some(parent) = event_parent {
            if parent == watched {
                debug!(
                    logger,
                    "event path parent matches watched path";
                    "event_path" => event_path.to_str(), "parent_path" => parent.to_str());
                return true;
            }
        }

        false
    };

    watched_paths.iter().any(|watched| {
        if matches(watched) {
            return true;
        }

        if let Ok(canonicalized_watch) = watched.canonicalize() {
            if matches(&canonicalized_watch) {
                return true;
            }
        }

        false
    })
}

#[cfg(test)]
mod tests {
    use super::{Watch, WatchPathBuf};
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::{self, Duration};
    use tempfile::tempdir;

    // A test helper function for setting up shell workspaces for testing.
    //
    // Command must be static because it guarantees there is no user
    // interpolation of shell commands.
    //
    // The command string is intentionally difficult to interpolate code
    // in to, for safety. Instead, pass variable arguments in `args` and
    // refer to them as `"$1"`, `"$2"`, etc.
    //
    // Watch your quoting, though, as you can still hurt yourself there.
    //
    // # Examples
    //
    //     expect_bash(r#"exit "$1""#, &["0"]);
    //
    // Make sure to properly quote your variables in the command string,
    // so bash can properly escape your code. This is safe, despite the
    // attempt at pwning my machine:
    //
    //     expect_bash(r#"echo "$1""#, &[r#"hi"; touch ./pwnd"#]);
    //
    fn expect_bash<I, S>(command: &'static str, args: I)
    where
        I: IntoIterator<Item = S> + std::fmt::Debug,
        S: AsRef<OsStr>,
    {
        let ret = std::process::Command::new("bash")
            .args(["-euc", command, "--"])
            .args(args)
            .status()
            .expect("bash should start properly, regardless of exit code");

        if !ret.success() {
            panic!("{:#?}", ret);
        }
    }

    // CI for macOS has been failing with an error like
    // `fatal runtime error: failed to initiate panic, error 5`
    // which appears to originate from this test.
    // In the interest of having a CI that works for our purposes,
    // I'm chopping out this one test in that environment.
    #[cfg_attr(target_os = "macos", ignore)]
    #[test]
    #[should_panic]
    fn expect_bash_can_fail() {
        expect_bash(r#"exit "$1""#, ["1"]);
    }

    #[test]
    fn expect_bash_can_pass() {
        expect_bash(r#"exit "$1""#, ["0"]);
    }
    /// upper bound of watcher (if it’s hit, something is broken) (CI machines are very slow around here …)
    const WATCHER_TIMEOUT: Duration = Duration::from_millis(2000);

    /// Watch for events, and return the first for which `pred` returns `Some()`. But only wait at most until timeout runs out.
    fn assert_one_within<F>(
        watch: &Watch,
        timeout: Duration,
        pred: F,
    ) -> (Vec<PathBuf>, Option<PathBuf>)
    where
        F: Fn(&PathBuf) -> bool,
    {
        let start = time::Instant::now();
        let mut rest = timeout.clone();
        let mut seen: Vec<PathBuf> = vec![];
        let mut i = 0;
        loop {
            println!("loop {} rest: {}ms, seen: {:?}", i, rest.as_millis(), seen);
            i = i + 1;
            let recv = watch.rx.recv_timeout(rest);
            println!("recv: {:#?}", recv);
            let files = recv
                .map_or(None, |e| watch.process_watch_events(e))
                .unwrap_or(vec![]);
            seen.extend(files.clone());
            for f in files {
                if pred(&f) {
                    return (seen, Some(f));
                }
            }
            rest = timeout - time::Instant::now().duration_since(start);
            if rest > Duration::from_nanos(0) {
                println!("breaking");
                break;
            }
        }
        (seen, None)
    }

    /// Assert no watcher event happens until the timeout
    fn assert_none_within(watch: &Watch, timeout: Duration) -> bool {
        watch.rx.recv_timeout(timeout).is_err()
    }

    /// Returns true iff the given file has changed
    fn file_changed_within(
        watch: &Watch,
        file_name: &str,
        timeout: Duration,
    ) -> (bool, Vec<PathBuf>) {
        let (seen, found) = assert_one_within(watch, timeout, |file| {
            file.file_name() == Some(OsStr::new(file_name))
        });
        (found.is_some(), seen)
    }

    fn assert_file_changed_within(watch: &Watch, file_name: &str, timeout: Duration) {
        let (file_changed, changed) = file_changed_within(watch, file_name, timeout);
        assert!(
            file_changed,
            "no file change notification for '{}'; these files changed instead: {:?}",
            file_name, changed
        );
    }

    #[cfg(target_os = "macos")]
    fn macos_eat_late_notifications(watcher: &mut Watch) {
        // Sometimes a brand new watch will send a CREATE notification
        // for a file which was just created, even if the watch was
        // created after the file was made.
        //
        // Our tests want to be very precise about which events are
        // received when, so expect these initial events and swallow
        // them.
        //
        // Note, this is racey in the kernel. Otherwise I'd assert
        // this is empty.
        sleep(WATCHER_TIMEOUT);
        watcher.rx.try_iter();
    }

    #[cfg(not(target_os = "macos"))]
    fn macos_eat_late_notifications(watcher: &mut Watch) {
        // If we're supposedly dealing with a late notification on
        // macOS, we'd better not receive any messages on other
        // platforms.
        //
        // If we do receive any notifications, our test is broken.
        assert!(assert_none_within(watcher, WATCHER_TIMEOUT));
    }

    #[test]
    fn trivial_watch_whole_directory() {
        let mut watcher =
            Watch::try_new(crate::logging::test_logger()).expect("failed creating Watch");
        let temp = tempdir().unwrap();
        let t = temp.path().as_os_str();

        expect_bash(r#"mkdir -p "$1"/foo"#, [t]);
        expect_bash(r#"touch "$1"/foo/bar"#, [t]);
        watcher
            .extend(vec![WatchPathBuf::Recursive(temp.path().to_path_buf())])
            .unwrap();

        expect_bash(r#"echo 1 > "$1/baz""#, [t]);
        assert_file_changed_within(&watcher, "baz", WATCHER_TIMEOUT);

        expect_bash(r#"echo 1 > "$1/foo/bar""#, [t]);
        assert_file_changed_within(&watcher, "bar", WATCHER_TIMEOUT);
    }

    #[test]
    fn trivial_watch_directory_not_recursively() {
        let mut watcher =
            Watch::try_new(crate::logging::test_logger()).expect("failed creating Watch");
        let temp = tempdir().unwrap();
        let t = temp.path().as_os_str();

        expect_bash(r#"mkdir -p "$1"/foo"#, [t]);
        expect_bash(r#"touch "$1"/foo/bar"#, [t]);
        watcher
            .extend(vec![WatchPathBuf::Normal(temp.path().to_path_buf())])
            .unwrap();

        expect_bash(r#"touch "$1/baz""#, [t]);
        assert_file_changed_within(&watcher, "baz", WATCHER_TIMEOUT);

        expect_bash(r#"echo 1 > "$1/foo/bar""#, [t]);
        assert!(assert_none_within(&watcher, WATCHER_TIMEOUT));
    }

    #[test]
    fn trivial_watch_specific_file() {
        let mut watcher =
            Watch::try_new(crate::logging::test_logger()).expect("failed creating Watch");
        let temp = tempdir().unwrap();
        let t = temp.path().as_os_str();

        expect_bash(r#"mkdir -p "$1""#, [t]);
        expect_bash(r#"touch "$1/foo""#, [t]);
        watcher
            .extend(vec![WatchPathBuf::Recursive(temp.path().join("foo"))])
            .unwrap();
        macos_eat_late_notifications(&mut watcher);

        expect_bash(r#"echo 1 > "$1/foo""#, [t]);
        sleep(WATCHER_TIMEOUT);
        assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);
    }

    #[test]
    fn rename_over_vim() {
        // Vim renames files in to place for atomic writes
        let mut watcher =
            Watch::try_new(crate::logging::test_logger()).expect("failed creating Watch");
        let temp = tempdir().unwrap();
        let t = temp.path().as_os_str();

        expect_bash(r#"mkdir -p "$1""#, [t]);
        expect_bash(r#"touch "$1/foo""#, [t]);
        watcher
            .extend(vec![WatchPathBuf::Recursive(temp.path().join("foo"))])
            .unwrap();
        macos_eat_late_notifications(&mut watcher);

        // bar is not watched, expect error
        expect_bash(r#"echo 1 > "$1/bar""#, [t]);
        assert!(assert_none_within(&watcher, WATCHER_TIMEOUT));

        // Rename bar to foo, expect a notification
        expect_bash(r#"mv "$1/bar" "$1/foo""#, [t]);
        assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);

        // Do it a second time
        expect_bash(r#"echo 1 > "$1/bar""#, [t]);
        assert!(assert_none_within(&watcher, WATCHER_TIMEOUT));

        // Rename bar to foo, expect a notification
        expect_bash(r#"mv "$1/bar" "$1/foo""#, [t]);
        assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);
    }

    #[test]
    fn walk_path_topo_filetree() -> std::io::Result<()> {
        let temp = tempdir().unwrap();

        let files = vec![("a", "b"), ("a", "c"), ("a/d", "e"), ("x/y", "z")];
        for (dir, file) in files {
            std::fs::create_dir_all(temp.path().join(dir))?;
            std::fs::write(temp.path().join(dir).join(file), [])?;
        }

        let res = super::walk_path_topo(temp.path().to_owned())?;

        // check that the list is topolocially sorted
        // by making sure *no* later path is a prefix of a previous path.
        let mut inv = res.clone();
        inv.reverse();
        for i in 0..inv.len() {
            for predecessor in inv.iter().skip(i + 1) {
                assert!(
                !predecessor.starts_with(&inv[i]),
                "{:?} is a prefix of {:?}, even though it comes later in list, thus topological order is not given!\nFull list: {:#?}",
                inv[i], predecessor, res
            )
            }
        }

        // make sure the resulting list contains the same
        // paths as the original list.
        let mut res2 = res.clone();
        res2.sort();
        let mut all_paths = [
            "", "a", // direct files come before nested directories
            "a/b", "a/c", "x", "a/d", "a/d/e", "x/y", "x/y/z",
        ]
        .iter()
        .map(|p| temp.path().join(p).to_owned())
        .collect::<Vec<_>>();
        all_paths.sort();
        assert_eq!(res2, all_paths);
        Ok(())
    }

    #[test]
    fn extend_filter() {
        let nix = PathBuf::from("/nix/store/njlavpa90laywf22b1myif5101qhln8r-hello-2.10");
        match super::Watch::extend_filter(nix.clone()) {
            Ok(path) => panic!("{:?} should be filtered!", path),
            Err(super::FilteredOut { path, .. }) => {
                assert_eq!(path, nix)
            }
        }

        let other = PathBuf::from("/home/foo/project/foobar.nix");
        assert_eq!(super::Watch::extend_filter(other.clone()), Ok(other));
    }
}
