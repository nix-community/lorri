package main

// Filesystem watcher: wraps fsnotify with debouncing, path filtering,
// recursive expansion, and /nix/store suppression.
//
// Mirrors src/watch.rs Watch + Filter + EventHandler.

import (
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"time"

	"github.com/fsnotify/fsnotify"
)

const watchDebounce = 200 * time.Millisecond

// Watch holds the fsnotify watcher and delivers batches of changed paths.
// Read from Events; each receive is a non-empty []string of absolute paths.
// Call Close when done.
type Watch struct {
	// Events delivers batches of changed absolute paths after debouncing.
	Events chan []string
	filter *watchFilter
}

type watchFilter struct {
	watcher *fsnotify.Watcher

	mu             sync.Mutex
	currentWatched map[string]struct{} // canonicalized absolute paths

	eventsCh chan []string
	stopCh   chan struct{}

	// dropFirstEventWithin: if non-zero, the very first batch arriving within
	// this duration of watcher creation is silently dropped.
	// Mirrors watch.rs EventHandlerKind::FirstEvent (used on macOS in tests).
	dropFirstEventWithin time.Duration
	startedAt            time.Time
	firstEventSeen       bool
}

// NewWatch creates a Watch with no paths registered yet.
func NewWatch() (*Watch, error) {
	return newWatchImpl(0)
}

// newWatchImpl is the internal constructor; dropFirst > 0 enables the macOS
// first-event-drop behaviour used in tests.
func newWatchImpl(dropFirstEventWithin time.Duration) (*Watch, error) {
	fw, err := fsnotify.NewWatcher()
	if err != nil {
		return nil, err
	}

	f := &watchFilter{
		watcher:              fw,
		currentWatched:       make(map[string]struct{}),
		eventsCh:             make(chan []string, 64),
		stopCh:               make(chan struct{}),
		dropFirstEventWithin: dropFirstEventWithin,
		startedAt:            time.Now(),
	}

	go f.run()

	return &Watch{
		Events: f.eventsCh,
		filter: f,
	}, nil
}

// Close shuts down the watcher and the background goroutine.
func (w *Watch) Close() {
	close(w.filter.stopCh)
	w.filter.watcher.Close()
}

// AddPaths registers additional paths for watching.
// Recursive paths are expanded to all files and subdirectories.
// Paths under /nix/store are silently skipped (immutable).
// Already-watched paths are not re-registered.
// Mirrors Filter::add_to_watch / Filter::extend.
func (w *Watch) AddPaths(paths []WatchPathBuf) error {
	return w.filter.addPaths(paths)
}

func (f *watchFilter) addPaths(paths []WatchPathBuf) error {
	for _, wp := range paths {
		// Expand recursive paths to all children; Normal paths stay as-is.
		var expanded []string
		if wp.Recursive {
			children, err := walkPathTopo(wp.Path)
			if err != nil {
				// Non-fatal: log and continue, matching Rust's warn-and-continue.
				continue
			}
			expanded = children
		} else {
			expanded = []string{wp.Path}
		}

		for _, raw := range expanded {
			canon, err := filepath.EvalSymlinks(raw)
			if err != nil {
				// Path may not exist yet; skip.
				continue
			}

			// Skip anything inside the Nix store — it is immutable.
			if strings.HasPrefix(canon, "/nix/store") {
				continue
			}

			// Hold the lock for the entire check-then-register operation to
			// avoid a TOCTOU race where two goroutines both observe a path as
			// unwatched and both call watcher.Add.
			// Mirrors Filter::extend in src/watch.rs.
			f.mu.Lock()
			if _, ok := f.currentWatched[canon]; !ok {
				if err := f.watcher.Add(canon); err == nil {
					f.currentWatched[canon] = struct{}{}
				}
			}
			// Also watch the parent directory so that rename-into-place
			// (Vim-style atomic writes) fire an event on the parent dir.
			// Mirrors the parent-watching logic in Filter::extend.
			if parent := filepath.Dir(canon); parent != canon {
				if _, ok := f.currentWatched[parent]; !ok {
					if err := f.watcher.Add(parent); err == nil {
						f.currentWatched[parent] = struct{}{}
					}
				}
			}
			f.mu.Unlock()
		}
	}
	return nil
}

// run is the background goroutine: reads raw fsnotify events, debounces
// them over 200 ms, filters, and sends batches to eventsCh.
// Mirrors EventHandler::handle_event + process_watch_events.
func (f *watchFilter) run() {
	var (
		timer   *time.Timer
		mu      sync.Mutex
		pending = make(map[string]struct{})
	)

	flush := func() {
		mu.Lock()
		if len(pending) == 0 {
			mu.Unlock()
			return
		}
		batch := make([]string, 0, len(pending))
		for p := range pending {
			batch = append(batch, p)
		}
		pending = make(map[string]struct{})
		timer = nil
		mu.Unlock()

		// First-event-drop logic (mirrors Rust's FirstEvent state machine).
		if !f.firstEventSeen {
			f.firstEventSeen = true
			if f.dropFirstEventWithin > 0 &&
				time.Since(f.startedAt) < f.dropFirstEventWithin {
				return
			}
		}

		select {
		case f.eventsCh <- batch:
		case <-f.stopCh:
		}
	}

	for {
		select {
		case <-f.stopCh:
			return

		case err, ok := <-f.watcher.Errors:
			if !ok {
				return
			}
			_ = err // log in production; ignore in translation

		case event, ok := <-f.watcher.Events:
			if !ok {
				return
			}

			path := event.Name

			// Mirror Rust: ignore pure access events.
			// fsnotify maps inotify IN_ACCESS / IN_OPEN to Chmod on Linux;
			// a Chmod-only event with no write/create/remove/rename is access-like.
			if event.Op == fsnotify.Chmod {
				// Additional filter: ignore metadata-only events under the
				// nix profiles dir (Nix unconditionally touches symlink metadata
				// there, which would cause rebuild loops).
				// Mirrors the ModifyKind::Metadata + /nix/var/nix/profiles/per-user check.
				if strings.HasPrefix(path, "/nix/var/nix/profiles/per-user") {
					continue
				}
				// On non-Linux, a Chmod may be a real event; on Linux it is
				// almost always spurious metadata. Skip it.
				if runtime.GOOS == "linux" {
					continue
				}
			}

			// Check whether the event path (or its parent) is in our watch set.
			if !f.pathMatch(path) {
				continue
			}

			mu.Lock()
			pending[path] = struct{}{}
			if timer == nil {
				timer = time.AfterFunc(watchDebounce, flush)
			}
			mu.Unlock()
		}
	}
}

// pathMatch returns true if path or its parent directory is in currentWatched.
// Mirrors EventHandler::path_match.
func (f *watchFilter) pathMatch(path string) bool {
	parent := filepath.Dir(path)
	f.mu.Lock()
	defer f.mu.Unlock()
	if _, ok := f.currentWatched[path]; ok {
		return true
	}
	if parent != path {
		if _, ok := f.currentWatched[parent]; ok {
			return true
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// walkPathTopo — mirrors watch.rs walk_path_topo
// ---------------------------------------------------------------------------

// walkPathTopo returns path itself followed by a topologically-ordered list
// of all its descendants (parent directories appear before their children).
// If path is not a directory it returns [path].
// Mirrors watch.rs walk_path_topo.
func walkPathTopo(path string) ([]string, error) {
	result := []string{path}

	info, err := os.Lstat(path)
	if err != nil {
		return nil, err
	}
	if !info.IsDir() {
		return result, nil
	}

	// BFS queue of directories to expand.
	queue := []string{path}
	for len(queue) > 0 {
		dir := queue[0]
		queue = queue[1:]

		entries, err := os.ReadDir(dir)
		if err != nil {
			return nil, err
		}

		var subdirs []string
		for _, e := range entries {
			child := filepath.Join(dir, e.Name())
			if e.Type()&fs.ModeSymlink != 0 {
				// Treat symlinks as files (don't recurse into them).
				result = append(result, child)
				continue
			}
			if e.IsDir() {
				subdirs = append(subdirs, child)
			} else {
				result = append(result, child)
			}
		}

		// Append subdirs to result before recursing (topological order:
		// parent before children), then enqueue them.
		result = append(result, subdirs...)
		queue = append(queue, subdirs...)
	}

	return result, nil
}
