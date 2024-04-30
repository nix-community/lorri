//! Recursively watch paths for changes, in an extensible and
//! cross-platform way.

use chan::{select, Receiver, Sender};
use crossbeam_channel as chan;
use notify::event::ModifyKind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap};
use slog::{debug, info, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::run_async::{Async, StopSignal};

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
///
/// It runs a thread, which is stopped once this struct is dropped.
pub struct Watch {
    /// Receives watch events. When receiving events, run `Watch::process` on them
    pub watch_events_rx: chan::Receiver<Vec<PathBuf>>,
    /// Extend the watch list with an additional list of paths.
    ///
    /// Note: Watch maintains a list of already watched paths, and
    /// will not add duplicates.
    pub add_to_watch_tx: chan::Sender<Vec<WatchPathBuf>>,
    /// Thread that waits for events.
    #[allow(dead_code)]
    watch_thread: Async<()>,
}

impl Watch {
    /// Instantiate a new Watch.
    pub fn new(logger: &slog::Logger) -> Result<Watch, notify::Error> {
        Self::new_impl(logger, None)
    }

    fn new_impl(
        logger: &slog::Logger,
        drop_first_event_within: Option<Duration>,
    ) -> Result<Watch, notify::Error> {
        let (filtered_events_tx, filtered_events_rx) = chan::unbounded();
        let (user_requests_tx, user_requests_rx) = chan::unbounded();

        let mut filter = Mutex::new(Filter::new(
            user_requests_rx,
            filtered_events_tx,
            drop_first_event_within,
            logger,
        )?);
        let watch_thread = Async::run_with_stop_signal(logger, move |stop_signal_rx| {
            filter
                .get_mut()
                .expect("watcher mutex poisoned")
                .loop_on_events(stop_signal_rx)
        });

        Ok(Watch {
            watch_events_rx: filtered_events_rx,
            add_to_watch_tx: user_requests_tx,
            watch_thread,
        })
    }
}

/// A debug message string that can only be displayed via `Debug`.
#[derive(Clone, Debug, Serialize)]
pub struct DebugMessage(pub String);

#[derive(Debug, PartialEq, Eq)]
struct FilteredOut<'a> {
    reason: &'a str,
    path: PathBuf,
}

struct Filter {
    /// The low-level watcher
    filesystem_watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    /// Unfiltered events from `notify` library
    filesystem_events_rx: Receiver<DebounceEventResult>,
    /// User requests to add more paths to our watcher
    user_requests_rx: Receiver<Vec<WatchPathBuf>>,
    /// Channel we send filtered messages to
    filtered_events_tx: Sender<Vec<PathBuf>>,
    /// Set of currently watched paths
    current_watched: HashSet<PathBuf>,
    // Whether to drop the first event if it arrives faster than the given duration (hack for macos tests)
    drop_first_event_within: Option<Duration>,
    logger: slog::Logger,
}

impl Filter {
    fn new(
        user_requests_rx: Receiver<Vec<WatchPathBuf>>,
        filtered_events_tx: Sender<Vec<PathBuf>>,
        drop_first_event_within: Option<Duration>,
        logger: &slog::Logger,
    ) -> notify::Result<Self> {
        let (filesystem_events_tx, filesystem_events_rx) = chan::unbounded();

        Ok(Filter {
            filesystem_watcher: notify_debouncer_full::new_debouncer(
                Duration::from_millis(200),
                None,
                filesystem_events_tx,
            )?,
            filesystem_events_rx,
            user_requests_rx,
            filtered_events_tx,
            current_watched: HashSet::new(),
            logger: logger.clone(),
            drop_first_event_within,
        })
    }

    fn loop_on_events(&mut self, stop_signal_rx: chan::Receiver<StopSignal>) {
        loop {
            let mut drop_event = false;
            let drop_first_event_within_rx = if let Some(dur) = self.drop_first_event_within {
                drop_event = true;
                chan::after(dur)
            } else {
                chan::never()
            };
            // make sure this only happens during the first loop
            self.drop_first_event_within = None;

            select! {

                // stop this watcher
                recv(stop_signal_rx) -> _ => {
                    debug!(self.logger, "watch filter loop received stop signal, stopping");
                    break;
                },

                // potentially drop the first message
                recv(drop_first_event_within_rx) -> _ => {
                    debug!(self.logger, "No event arrived within the initial drop timeout."; "duration" => ?self.drop_first_event_within);
                }

                // Handle file events
                recv(self.filesystem_events_rx) -> msg => match msg {
                    Ok( DebounceEventResult::Ok(event)) => {
                        if drop_event {
                            debug!(self.logger, "dropping event because drop_event was true"; "event" => ?event);
                            continue
                        }
                        let paths = self.process_watch_events(event);
                        if !paths.is_empty() {
                            if let Err(e) = self.filtered_events_tx.send(paths) {
                                warn!(self.logger, "filtered_events_tx send error"; "error" => ?e)
                            }
                    }},
                    Ok(DebounceEventResult::Err(errs)) => {
                        warn!(self.logger, "notify library threw errors: {:#?}", errs);
                        continue
                    },
                    Err(_recv_error) => {
                        debug!(self.logger, "filesystem notify channel was disconnected");
                        return
                    }
                },

                // Add new files to watch
                recv(self.user_requests_rx) -> msg => match msg {
                    Ok(paths) => {
                        let path_log = format!("{:?}", paths);
                        if let Err(e) = self.extend(paths) {
                                warn!(self.logger, "error extending watch paths:"; "error" => ?e, "paths" => path_log)
                        }
                    },
                    Err(chan::RecvError) => {
                        debug!(self.logger, "watch extension channel was disconnected");
                        return
                    }
                }
            }
        }
    }

    /// Process `notify::Event`s coming in via `Watch::rx`.
    ///
    /// Returns a list of „interesting“ paths, if any.
    fn process_watch_events(&self, events: Vec<DebouncedEvent>) -> Vec<PathBuf> {
        let mut interesting_paths = vec![];
        for event in &events {
            {
                match event.kind {
                    notify::event::EventKind::Remove(_) if !event.paths.is_empty() => {
                        info!(self.logger, "identified removal: {:?}", &event.paths);
                    }
                    _ => {
                        debug!(self.logger, "watch event"; "event" => ?event);
                    }
                }
            };
            let notify::Event {
                ref paths, kind, ..
            } = event.event;
            for path in paths {
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
                if let EventKind::Modify(ModifyKind::Metadata(_)) = kind {
                    if path.starts_with(Path::new("/nix/var/nix/profiles/per-user")) {
                        continue;
                    }
                }

                if self.path_match(path) {
                    interesting_paths.push((path, event))
                }
            }
        }
        if interesting_paths.is_empty() {
            debug!(self.logger, "generated no interesting paths");
        } else {
            debug!(self.logger, "generated interesting paths"; "paths" => ?interesting_paths);
        }
        interesting_paths
            .into_iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Extend the watch list with an additional list of paths.
    ///
    /// Note: Watch maintains a list of already watched paths, and
    /// will not add duplicates.
    pub fn extend(&mut self, paths: Vec<WatchPathBuf>) -> Result<(), notify::Error> {
        for path in paths {
            // NOTE: notify.watch supports recursively watching directories itself, but we
            // 1) want to canonicalize each path we watch
            // 2) ignore everything in /nix/store and pointing to something in /nix/store
            // Plus, notify.watch will itself just walk the directories and watch things one-by-one
            // (at least for the `inotify` backend), so all is good on the performance front.
            let recursive_paths = match path {
                WatchPathBuf::Recursive(path) => walk_path_topo(path)?,
                WatchPathBuf::Normal(path) => vec![path],
            };

            for p_raw in recursive_paths {
                let p = p_raw.canonicalize()?;
                if p.starts_with(Path::new("/nix/store")) {
                    debug!(
                        self.logger,
                        "Skipping watching {}: {}",
                        p.display(),
                        "starts with /nix/store"
                    )
                } else {
                    let this = &mut *self;
                    if !this.current_watched.contains(&p) {
                        debug!(this.logger, "watching path"; "path" => p.to_str());

                        this.filesystem_watcher
                            .watcher()
                            .watch(&p, RecursiveMode::NonRecursive)?;
                        this.current_watched.insert(p.clone());
                    }

                    if let Some(parent) = p.parent() {
                        if !this.current_watched.contains(parent) {
                            debug!(this.logger, "watching parent path"; "parent_path" => parent.to_str());

                            this.filesystem_watcher
                                .watcher()
                                .watch(parent, RecursiveMode::NonRecursive)?;
                        }
                    }
                }
            }
        }
        Ok(())
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
    fn path_match(&self, event_path: &Path) -> bool {
        let event_parent = event_path.parent();

        self.current_watched.iter().any(|watched: &PathBuf| {
            if event_path == watched {
                debug!(
                self.logger,
                "event path directly matches watched path";
                "event_path" => event_path.to_str());

                return true;
            }

            if let Some(parent) = event_parent {
                if parent == watched {
                    debug!(
                    self.logger,
                    "event path parent matches watched path";
                    "event_path" => event_path.to_str(), "parent_path" => parent.to_str());
                    return true;
                }
            }

            false
        })
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

#[cfg(test)]
mod tests {
    use super::{Watch, WatchPathBuf};
    use slog::{debug, info};
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::{self, Duration};
    use tempfile::{tempdir, TempDir};

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
        let mut rest = timeout;
        let mut seen: Vec<PathBuf> = vec![];
        let mut i = 0;
        loop {
            println!("loop {} rest: {}ms, seen: {:?}", i, rest.as_millis(), seen);
            i += 1;
            let files = watch
                .watch_events_rx
                .recv_timeout(rest)
                .expect("working notify in tests");
            println!("files: {:#?}", files);
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
    ///
    /// If file_suffixes_opt is given, only these files will be checked for.
    fn assert_none_within(
        watch: &Watch,
        timeout: Duration,
        file_suffixes_opt: Option<&[&str]>,
        logger: &slog::Logger,
    ) {
        let res = watch.watch_events_rx.recv_timeout(timeout);
        match res {
            Err(_) => (),
            Ok(watch_result) => {
                if let Some(file_suffixes) = file_suffixes_opt {
                    if !watch_result
                        .iter()
                        .any(|res| file_suffixes.into_iter().any(|suff| res.ends_with(suff)))
                    {
                        debug!(logger, "ignoring event not part of ignore filter"; "watch_result" => ?watch_result, "file_suffixes" => ?file_suffixes);
                    }
                }
                panic!(
                    "expected no file change notification for suffixes {:?}; but these files changed: {:?}",
                    file_suffixes_opt,
                    watch_result
                );
            }
        }
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

    /// Create a tempdir for our test and drop it after the function runs.
    fn with_test_tempdir<F>(test_name: &str, f: F)
    where
        F: FnOnce(&std::path::Path),
    {
        let temp: TempDir = tempdir().unwrap();

        // TODO: We use a subdirectory for our tests, because the watcher (for whatever reason) also watches the parent directory, which means we start watching `/tmp` in our tests …
        f(&temp.path().join("testdir_of_".to_string() + test_name));
        drop(temp);
    }

    fn mk_test_watch(logger: &slog::Logger) -> Watch {
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
        (if cfg!(target_os = "macos") {
            Watch::new_impl(logger, Some(WATCHER_TIMEOUT))
        } else {
            Watch::new_impl(logger, None)
        })
        .expect("failed creating watch")
    }

    #[test]
    fn trivial_watch_whole_directory() {
        let logger = crate::logging::test_logger("trivial_watch_whole_directory");
        let watcher = mk_test_watch(&logger);
        with_test_tempdir("trivial_watch_whole_directory", |t| {
            expect_bash(r#"mkdir -p "$1"/foo"#, [t]);
            expect_bash(r#"touch "$1"/foo/bar"#, [t]);
            watcher
                .add_to_watch_tx
                .send(vec![WatchPathBuf::Recursive(t.to_path_buf())])
                .unwrap();

            expect_bash(r#"echo 1 > "$1/baz""#, [t]);
            assert_file_changed_within(&watcher, "baz", WATCHER_TIMEOUT);

            expect_bash(r#"echo 1 > "$1/foo/bar""#, [t]);
            assert_file_changed_within(&watcher, "bar", WATCHER_TIMEOUT);
        })
    }

    #[test]
    fn trivial_watch_directory_not_recursively() {
        let logger = crate::logging::test_logger("trivial_watch_directory_not_recursively");
        let watcher = mk_test_watch(&logger);
        with_test_tempdir("trivial_watch_directory_not_recursively", |t| {
            expect_bash(r#"mkdir -p "$1"/foo"#, [t]);
            expect_bash(r#"touch "$1"/foo/bar"#, [t]);
            watcher
                .add_to_watch_tx
                .send(vec![WatchPathBuf::Normal(t.to_path_buf())])
                .unwrap();

            expect_bash(r#"touch "$1/baz""#, [t]);
            assert_file_changed_within(&watcher, "baz", WATCHER_TIMEOUT);

            expect_bash(r#"echo 1 > "$1/foo/bar""#, [t]);
            assert_none_within(&watcher, WATCHER_TIMEOUT, None, &logger);
        })
    }
    #[test]
    fn trivial_watch_specific_file() {
        let logger = crate::logging::test_logger("trivial_watch_specific_file");
        let watcher = mk_test_watch(&logger);

        with_test_tempdir("trivial_watch_specific_file", |t| {
            expect_bash(r#"mkdir -p "$1""#, [t]);
            expect_bash(r#"touch "$1/foo""#, [t]);
            watcher
                .add_to_watch_tx
                .send(vec![WatchPathBuf::Recursive(t.join("foo"))])
                .unwrap();

            expect_bash(r#"echo 1 > "$1/foo""#, [t]);
            sleep(WATCHER_TIMEOUT);
            assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);
        })
    }

    // TODO: this test is bugged, but in order to figure out what is wrong,
    // we should add some sort of provenance to our watcher filter functions first.
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn rename_over_vim() {
        // Vim renames files in to place for atomic writes
        let logger = crate::logging::test_logger("rename_over_vim");
        let watcher = mk_test_watch(&logger);

        with_test_tempdir("rename_over_vim", |t| {
            expect_bash(r#"mkdir -p "$1""#, [t]);
            expect_bash(r#"touch "$1/foo""#, [t]);
            watcher
                .add_to_watch_tx
                .send(vec![WatchPathBuf::Recursive(t.join("foo"))])
                .unwrap();

            info!(&logger, "bar is not watched, expect error");
            expect_bash(r#"echo 1 > "$1/bar""#, [t]);
            assert_none_within(&watcher, WATCHER_TIMEOUT, Some(&vec!["/bar"]), &logger);

            info!(&logger, "Rename bar to foo, expect a notification");
            expect_bash(r#"mv "$1/bar" "$1/foo""#, [t]);
            assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);

            info!(&logger, "Do it a second time");
            expect_bash(r#"echo 1 > "$1/bar""#, [t]);
            assert_none_within(&watcher, WATCHER_TIMEOUT, None, &logger);

            info!(&logger, "Rename bar to foo, expect a notification");
            expect_bash(r#"mv "$1/bar" "$1/foo""#, [t]);
            assert_file_changed_within(&watcher, "foo", WATCHER_TIMEOUT);
        })
    }

    #[test]
    fn walk_path_topo_filetree() {
        with_test_tempdir("walk_path_topo_filetree", |t| {
            let files = vec![("a", "b"), ("a", "c"), ("a/d", "e"), ("x/y", "z")];
            for (dir, file) in files {
                std::fs::create_dir_all(t.join(dir)).unwrap();
                std::fs::write(t.join(dir).join(file), []).unwrap();
            }

            let res = super::walk_path_topo(t.to_owned()).unwrap();

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
            .map(|p| t.join(p))
            .collect::<Vec<_>>();
            all_paths.sort();
            assert_eq!(res2, all_paths);
        })
    }
}
