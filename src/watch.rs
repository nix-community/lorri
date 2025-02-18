//! Recursively watch paths for changes, in an extensible and
//! cross-platform way.

use crate::watch::EventHandlerKind::{FirstEvent, FollowingEvent};
use notify::event::ModifyKind;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap};
use slog::{debug, info, warn};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::Instant;

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
///
pub struct Watch {
    /// Receives watch events. When receiving events, run `Watch::process` on them
    pub watch_events_rx: UnboundedReceiver<Vec<PathBuf>>,
    /// holds the filsystem watcher (thread)
    pub filter: Filter,
}

impl Watch {
    /// Instantiate a new Watch.
    ///
    /// Don’t forget to call `stop` once you don’t want to watch anymore.
    pub fn try_new(logger: &slog::Logger) -> Result<Watch, notify::Error> {
        Self::new_impl(logger, None)
    }

    /// Stop the filesystem watcher, nonblocking
    /// (will take another few ms to actually tick to completion).
    pub fn stop_nonblocking(self) {
        self.filter.filesystem_watcher.stop_nonblocking()
    }

    fn new_impl(
        logger: &slog::Logger,
        drop_first_event_within: Option<Duration>,
    ) -> Result<Watch, notify::Error> {
        let (filtered_events_tx, filtered_events_rx) = unbounded_channel();

        let filter = Filter::new(filtered_events_tx, drop_first_event_within, logger)?;

        Ok(Watch {
            watch_events_rx: filtered_events_rx,
            filter,
        })
    }
}

/// A debug message string that can only be displayed via `Debug`.
#[derive(Clone, Debug, Serialize)]
pub struct DebugMessage(pub String);

/// Holds the filesystem watcher thread & filters interesting events.
pub struct Filter {
    /// The low-level watcher
    filesystem_watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    /// Set of currently watched paths
    current_watched: Arc<Mutex<HashSet<PathBuf>>>,
    logger: slog::Logger,
}

impl Filter {
    fn new(
        // Channel we send filtered messages to
        filtered_events_tx: tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>,
        drop_first_event_within: Option<Duration>,
        logger: &slog::Logger,
    ) -> notify::Result<Self> {
        let current_watched = Arc::new(Mutex::new(HashSet::new()));

        Ok(Filter {
            filesystem_watcher: notify_debouncer_full::new_debouncer_opt(
                Duration::from_millis(200),
                None,
                EventHandler {
                    logger: logger.clone(),
                    kind: FirstEvent {
                        handler_start: Instant::now(),
                        drop_first_event_within,
                    },
                    filtered_events_tx,
                    current_watched: current_watched.clone(),
                },
                FileIdMap::new(),
                Config::default()
                    .with_follow_symlinks(true)
                    .with_poll_interval(Duration::from_secs(5)),
            )?,
            current_watched,
            logger: logger.clone(),
        })
    }

    /// Extend the watch list with an additional list of paths.
    ///
    /// Note: Watch maintains a list of already watched paths, and
    /// will not add duplicates.
    pub fn add_to_watch(&mut self, paths: Vec<WatchPathBuf>) {
        let path_log = format!("{:?}", paths);
        if let Err(e) = self.extend(paths) {
            warn!(self.logger, "error extending watch paths:"; "error" => ?e, "paths" => path_log)
        }
    }

    /// Extend the watch list with an additional list of paths.
    ///
    /// Note: Watch maintains a list of already watched paths, and
    /// will not add duplicates.
    fn extend(&mut self, paths: Vec<WatchPathBuf>) -> Result<(), notify::Error> {
        struct WatchingPaths {
            parent_paths: Vec<OsString>,
            paths: Vec<OsString>,
        }
        let mut watching_paths = WatchingPaths {
            parent_paths: vec![],
            paths: vec![],
        };

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
                    if !{ this.current_watched.lock()?.contains(&p) } {
                        watching_paths.paths.push(p.clone().into_os_string());

                        this.filesystem_watcher
                            .watch(&p, RecursiveMode::NonRecursive)?;
                        {
                            this.current_watched.lock()?.insert(p.clone())
                        };
                    }

                    if let Some(parent) = p.parent() {
                        if !{ this.current_watched.lock()?.contains(parent) } {
                            watching_paths
                                .parent_paths
                                .push(parent.to_owned().into_os_string());

                            this.filesystem_watcher
                                .watch(parent, RecursiveMode::NonRecursive)?;
                        }
                    }
                }
            }
        }
        debug!(self.logger, "watching paths";
            "paths" => ?watching_paths.paths,
            "parent_paths" => ?watching_paths.parent_paths);
        Ok(())
    }
}

struct EventHandler {
    logger: slog::Logger,
    kind: EventHandlerKind,
    /// Channel we send filtered messages to
    filtered_events_tx: tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>,
    /// Set of currently watched paths
    current_watched: Arc<Mutex<HashSet<PathBuf>>>,
}

/// state machine for figuring out whether we want to drop the first event.
enum EventHandlerKind {
    FirstEvent {
        handler_start: Instant,
        drop_first_event_within: Option<Duration>,
    },
    FollowingEvent,
}

impl notify_debouncer_full::DebounceEventHandler for EventHandler {
    fn handle_event(&mut self, event: DebounceEventResult) {
        // handle the weird logic for dropping the first event if necessary
        match self.kind {
            FirstEvent {
                handler_start,
                drop_first_event_within,
            } => {
                self.kind = FollowingEvent;
                if let Some(within) = drop_first_event_within {
                    if Instant::now()
                        .saturating_duration_since(handler_start)
                        .lt(&within)
                    {
                        // dropping the first event
                        debug!(self.logger, "Dropping first event, within initial drop timeout."; "duration" => ?drop_first_event_within);
                        return;
                    }
                }
            }
            FollowingEvent => {}
        }

        // now actually handle the event
        match event {
            Ok(event) => {
                let paths = self.process_watch_events(event);
                if !paths.is_empty() {
                    if let Err(e) = self.filtered_events_tx.send(paths) {
                        warn!(self.logger, "filtered_events_tx send error"; "error" => ?e)
                    }
                }
            }
            Err(errs) => {
                warn!(self.logger, "notify library threw errors: {:#?}", errs);
            }
        }
    }
}
impl EventHandler {
    /// Process `notify::Event`s coming in via `Watch::rx`.
    ///
    /// Returns a list of „interesting“ paths, if any.
    fn process_watch_events(&self, events: Vec<DebouncedEvent>) -> Vec<PathBuf> {
        let mut interesting_paths = vec![];
        for event in &events {
            {
                match event.kind {
                    EventKind::Remove(_) if !event.paths.is_empty() => {
                        info!(self.logger, "identified removal: {:?}", &event.paths);
                    }
                    // we only want to pick up changes to files, not access
                    EventKind::Access(_) => continue,
                    _ => {
                        // debug!(self.logger, "watch event"; "event" => ?event);
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
            // debug!(self.logger, "generated no interesting paths");
        } else {
            // debug!(self.logger, "generated interesting paths"; "paths" => ?interesting_paths);
        }
        interesting_paths
            .into_iter()
            .map(|(path, _)| path.clone())
            .collect()
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

        let lock = self.current_watched.lock().unwrap();
        let res = lock.iter().any(|watched: &PathBuf| {
            if event_path == watched {
                // debug!(
                // self.logger,
                // "event path directly matches watched path";
                // "event_path" => event_path.to_str());

                return true;
            }

            if let Some(parent) = event_parent {
                if parent == watched {
                    // debug!(
                    // self.logger,
                    // "event path parent matches watched path";
                    // "event_path" => event_path.to_str(), "parent_path" => parent.to_str());
                    return true;
                }
            }

            false
        });
        drop(lock);
        res
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
    use std::future::Future;
    use std::path::PathBuf;
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
    async fn assert_one_within<F>(
        watch: &mut Watch,
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
            let files = tokio::time::timeout(rest, watch.watch_events_rx.recv())
                .await
                .expect("working notify in tests")
                .expect("watch event rx closed");
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
    async fn assert_none_within(
        watch: &mut Watch,
        timeout: Duration,
        file_suffixes_opt: Option<&[&str]>,
        logger: &slog::Logger,
    ) {
        let res = tokio::time::timeout(timeout, async {
            watch.watch_events_rx.recv().await.unwrap()
        })
        .await;
        match res {
            Err(_) => (),
            Ok(watch_result) => {
                if let Some(file_suffixes) = file_suffixes_opt {
                    if !watch_result
                        .clone()
                        .into_iter()
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
    async fn file_changed_within(
        watch: &mut Watch,
        file_name: &str,
        timeout: Duration,
    ) -> (bool, Vec<PathBuf>) {
        let (seen, found) = assert_one_within(watch, timeout, |file| {
            file.file_name() == Some(OsStr::new(file_name))
        })
        .await;
        (found.is_some(), seen)
    }

    async fn assert_file_changed_within(watch: &mut Watch, file_name: &str, timeout: Duration) {
        let (file_changed, changed) = file_changed_within(watch, file_name, timeout).await;
        assert!(
            file_changed,
            "no file change notification for '{}'; these files changed instead: {:?}",
            file_name, changed
        );
    }

    /// Create a tempdir for our test and drop it after the function runs.
    async fn with_test_tempdir<F, Fut>(test_name: &str, f: F)
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = ()>,
    {
        let temp: TempDir = tempdir().unwrap();

        // TODO: We use a subdirectory for our tests, because the watcher (for whatever reason) also watches the parent directory, which means we start watching `/tmp` in our tests …
        f(temp.path().join("testdir_of_".to_string() + test_name)).await;
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

    #[tokio::test]
    async fn trivial_watch_whole_directory() {
        let logger = crate::logging::test_logger("trivial_watch_whole_directory");
        let mut watcher = mk_test_watch(&logger);
        with_test_tempdir("trivial_watch_whole_directory", |t2| async move {
            let t = &t2;
            expect_bash(r#"mkdir -p "$1"/foo"#, [t]);
            expect_bash(r#"touch "$1"/foo/bar"#, [t]);
            watcher
                .filter
                .add_to_watch(vec![WatchPathBuf::Recursive(t.to_path_buf())]);

            expect_bash(r#"echo 1 > "$1/baz""#, [t]);
            assert_file_changed_within(&mut watcher, "baz", WATCHER_TIMEOUT).await;

            expect_bash(r#"echo 1 > "$1/foo/bar""#, [t]);
            assert_file_changed_within(&mut watcher, "bar", WATCHER_TIMEOUT).await;

            watcher.stop_nonblocking()
        })
        .await;
    }

    #[tokio::test]
    async fn trivial_watch_directory_not_recursively() {
        let logger = crate::logging::test_logger("trivial_watch_directory_not_recursively");
        let mut watcher = mk_test_watch(&logger);
        with_test_tempdir("trivial_watch_directory_not_recursively", |t| async move {
            expect_bash(r#"mkdir -p "$1"/foo"#, [&t]);
            expect_bash(r#"touch "$1"/foo/bar"#, [&t]);
            watcher
                .filter
                .add_to_watch(vec![WatchPathBuf::Normal((&t).to_path_buf())]);

            expect_bash(r#"touch "$1/baz""#, [&t]);
            assert_file_changed_within(&mut watcher, "baz", WATCHER_TIMEOUT).await;

            expect_bash(r#"echo 1 > "$1/foo/bar""#, [&t]);
            assert_none_within(&mut watcher, WATCHER_TIMEOUT, None, &logger).await;

            watcher.stop_nonblocking()
        })
        .await;
    }
    #[tokio::test]
    async fn trivial_watch_specific_file() {
        let logger = crate::logging::test_logger("trivial_watch_specific_file");
        let mut watcher = mk_test_watch(&logger);

        with_test_tempdir("trivial_watch_specific_file", |t| async move {
            expect_bash(r#"mkdir -p "$1""#, [&t]);
            expect_bash(r#"touch "$1/foo""#, [&t]);
            watcher
                .filter
                .add_to_watch(vec![WatchPathBuf::Recursive((&t).join("foo"))]);

            expect_bash(r#"echo 1 > "$1/foo""#, [&t]);
            tokio::time::sleep(WATCHER_TIMEOUT).await;
            assert_file_changed_within(&mut watcher, "foo", WATCHER_TIMEOUT).await;

            watcher.stop_nonblocking()
        })
        .await;
    }

    // TODO: this test is bugged, but in order to figure out what is wrong,
    // we should add some sort of provenance to our watcher filter functions first.
    #[tokio::test]
    #[cfg(not(target_os = "macos"))]
    async fn rename_over_vim() {
        // Vim renames files in to place for atomic writes
        let logger = crate::logging::test_logger("rename_over_vim");
        let mut watcher = mk_test_watch(&logger);

        with_test_tempdir("rename_over_vim", |t| async move {
            expect_bash(r#"mkdir -p "$1""#, [&t]);
            expect_bash(r#"touch "$1/foo""#, [&t]);
            watcher
                .filter
                .add_to_watch(vec![WatchPathBuf::Recursive((&t).join("foo"))]);

            info!(&logger, "bar is not watched, expect error");
            expect_bash(r#"echo 1 > "$1/bar""#, [&t]);
            assert_none_within(&mut watcher, WATCHER_TIMEOUT, Some(&vec!["/bar"]), &logger).await;

            info!(&logger, "Rename bar to foo, expect a notification");
            expect_bash(r#"mv "$1/bar" "$1/foo""#, [&t]);
            assert_file_changed_within(&mut watcher, "foo", WATCHER_TIMEOUT).await;

            info!(&logger, "Do it a second time");
            expect_bash(r#"echo 1 > "$1/bar""#, [&t]);
            assert_none_within(&mut watcher, WATCHER_TIMEOUT, None, &logger).await;

            info!(&logger, "Rename bar to foo, expect a notification");
            expect_bash(r#"mv "$1/bar" "$1/foo""#, [&t]);
            assert_file_changed_within(&mut watcher, "foo", WATCHER_TIMEOUT).await;

            watcher.stop_nonblocking()
        })
        .await;
    }

    #[tokio::test]
    async fn walk_path_topo_filetree() {
        with_test_tempdir("walk_path_topo_filetree", |t| async move {
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
        }).await
    }
}
