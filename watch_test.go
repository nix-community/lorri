package main

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

const watcherTimeout = 2 * time.Second

// watcherSettle is how long to wait after AddPaths before triggering a real
// event, to let macOS fsnotify emit and discard its spurious startup event.
const watcherSettle = 600 * time.Millisecond

// waitForFile blocks until a path whose base name equals fileName appears in
// a batch from w, or until timeout. Returns (found, allSeen).
func waitForFile(t *testing.T, w *Watch, fileName string, timeout time.Duration) (bool, []string) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	var seen []string
	for time.Now().Before(deadline) {
		remaining := time.Until(deadline)
		select {
		case batch := <-w.Events:
			seen = append(seen, batch...)
			for _, p := range batch {
				if filepath.Base(p) == fileName {
					return true, seen
				}
			}
		case <-time.After(remaining):
			return false, seen
		}
	}
	return false, seen
}

// assertNoEvent asserts that no event arrives within timeout.
// If suffixes is non-nil, only events whose paths end with one of the given
// suffixes are considered failures (mirroring Rust's file_suffixes_opt filter).
func assertNoEvent(t *testing.T, w *Watch, timeout time.Duration, suffixes ...string) {
	t.Helper()
	select {
	case batch := <-w.Events:
		if len(suffixes) > 0 {
			// Only fail if an event path ends with one of the given suffixes.
			for _, p := range batch {
				for _, suf := range suffixes {
					if strings.HasSuffix(p, suf) {
						t.Errorf("expected no event matching %v within %v, got: %v", suffixes, timeout, batch)
						return
					}
				}
			}
			// None of the paths matched the filter; ignore.
			return
		}
		t.Errorf("expected no event within %v, got: %v", timeout, batch)
	case <-time.After(timeout):
	}
}

// mkWatchForTest creates a Watch, optionally enabling first-event-drop (macOS).
// On macOS, fsnotify fires a spurious initial event shortly after a path is
// registered; we drop any batch arriving within 500ms of watcher creation.
func mkWatchForTest(t *testing.T) *Watch {
	t.Helper()
	var dropFirst time.Duration
	if runtime.GOOS == "darwin" {
		dropFirst = 500 * time.Millisecond
	}
	w, err := newWatchImpl(dropFirst)
	if err != nil {
		t.Fatalf("NewWatch: %v", err)
	}
	return w
}

// withTestDir creates a temp subdirectory, runs f, then cleans up.
func withTestDir(t *testing.T, f func(dir string)) {
	t.Helper()
	base := t.TempDir()
	dir := filepath.Join(base, "testdir_of_"+t.Name())
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	f(dir)
}

// writeFile writes content to path, creating parent dirs as needed.
func writeFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

// touchFile creates an empty file.
func touchFile(t *testing.T, path string) {
	t.Helper()
	writeFile(t, path, "")
}

// ---------------------------------------------------------------------------
// Tests — mirrors watch.rs test cases
// ---------------------------------------------------------------------------

// TestWatchWholeDirectoryRecursive mirrors trivial_watch_whole_directory.
// A Recursive watch on a directory should fire for files at any depth.
func TestWatchWholeDirectoryRecursive(t *testing.T) {
	w := mkWatchForTest(t)
	defer w.Close()

	withTestDir(t, func(dir string) {
		fooDir := filepath.Join(dir, "foo")
		if err := os.MkdirAll(fooDir, 0o755); err != nil {
			t.Fatal(err)
		}
		touchFile(t, filepath.Join(fooDir, "bar"))

		if err := w.AddPaths([]WatchPathBuf{{Recursive: true, Path: dir}}); err != nil {
			t.Fatalf("AddPaths: %v", err)
		}
		time.Sleep(watcherSettle)

		// Write a file at the top level.
		writeFile(t, filepath.Join(dir, "baz"), "1")
		found, seen := waitForFile(t, w, "baz", watcherTimeout)
		if !found {
			t.Errorf("expected event for baz; saw: %v", seen)
		}

		// Write a file in the subdirectory.
		writeFile(t, filepath.Join(fooDir, "bar"), "1")
		found, seen = waitForFile(t, w, "bar", watcherTimeout)
		if !found {
			t.Errorf("expected event for bar; saw: %v", seen)
		}
	})
}

// TestWatchDirectoryNonRecursive mirrors trivial_watch_directory_not_recursively.
// A Normal watch on a directory should NOT fire for files in subdirectories.
func TestWatchDirectoryNonRecursive(t *testing.T) {
	w := mkWatchForTest(t)
	defer w.Close()

	withTestDir(t, func(dir string) {
		fooDir := filepath.Join(dir, "foo")
		if err := os.MkdirAll(fooDir, 0o755); err != nil {
			t.Fatal(err)
		}
		touchFile(t, filepath.Join(fooDir, "bar"))

		if err := w.AddPaths([]WatchPathBuf{{Recursive: false, Path: dir}}); err != nil {
			t.Fatalf("AddPaths: %v", err)
		}
		time.Sleep(watcherSettle)

		// A file directly in dir should fire.
		touchFile(t, filepath.Join(dir, "baz"))
		found, seen := waitForFile(t, w, "baz", watcherTimeout)
		if !found {
			t.Errorf("expected event for baz; saw: %v", seen)
		}

		// A file in a subdirectory should NOT fire.
		writeFile(t, filepath.Join(fooDir, "bar"), "changed")
		assertNoEvent(t, w, watcherTimeout)
	})
}

// TestWatchSpecificFile mirrors trivial_watch_specific_file.
// Watching a single file path fires when that file changes.
func TestWatchSpecificFile(t *testing.T) {
	w := mkWatchForTest(t)
	defer w.Close()

	withTestDir(t, func(dir string) {
		fooPath := filepath.Join(dir, "foo")
		touchFile(t, fooPath)

		if err := w.AddPaths([]WatchPathBuf{{Recursive: true, Path: fooPath}}); err != nil {
			t.Fatalf("AddPaths: %v", err)
		}
		time.Sleep(watcherSettle)

		writeFile(t, fooPath, "changed")
		found, seen := waitForFile(t, w, "foo", watcherTimeout)
		if !found {
			t.Errorf("expected event for foo; saw: %v", seen)
		}
	})
}

// TestWatchRenameOverVim mirrors rename_over_vim (Linux only).
// Vim-style atomic write: write to a temp file, rename it over the target.
// The watcher should fire for the target (foo) after the rename, but must
// NOT fire for writes to sibling files (bar) — those are unrelated files
// in the same directory and should not trigger rebuilds.
func TestWatchRenameOverVim(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("rename-over test is Linux-only")
	}

	w := mkWatchForTest(t)
	defer w.Close()

	withTestDir(t, func(dir string) {
		fooPath := filepath.Join(dir, "foo")
		barPath := filepath.Join(dir, "bar")
		touchFile(t, fooPath)

		if err := w.AddPaths([]WatchPathBuf{{Recursive: true, Path: fooPath}}); err != nil {
			t.Fatalf("AddPaths: %v", err)
		}

		// Write bar (sibling of foo): must NOT produce an event.
		writeFile(t, barPath, "1")
		assertNoEvent(t, w, watchDebounce+50*time.Millisecond)

		// Rename bar→foo: must fire a foo event.
		if err := os.Rename(barPath, fooPath); err != nil {
			t.Fatalf("rename: %v", err)
		}
		found, seen := waitForFile(t, w, "foo", watcherTimeout)
		if !found {
			t.Errorf("expected event for foo after rename; saw: %v", seen)
		}

		// Second round.
		writeFile(t, barPath, "2")
		assertNoEvent(t, w, watchDebounce+50*time.Millisecond)

		if err := os.Rename(barPath, fooPath); err != nil {
			t.Fatalf("rename: %v", err)
		}
		found, seen = waitForFile(t, w, "foo", watcherTimeout)
		if !found {
			t.Errorf("expected event for foo after second rename; saw: %v", seen)
		}
	})
}

// TestWatchSiblingFileNoSpuriousEvent is a regression test for the spin-loop
// bug: a file in the same directory as a watched file must not trigger events.
//
// Previously, addPaths added parent directories to currentWatched so that
// pathMatch would fire for any file written alongside a watched file (e.g. a
// cast recorder writing demo.cast next to shell.nix, or lorri.sqlite-wal
// being written in the same directory as logged-evaluation.nix).
func TestWatchSiblingFileNoSpuriousEvent(t *testing.T) {
	w := mkWatchForTest(t)
	defer w.Close()

	withTestDir(t, func(dir string) {
		fooPath := filepath.Join(dir, "foo")
		siblingPath := filepath.Join(dir, "sibling")
		touchFile(t, fooPath)

		if err := w.AddPaths([]WatchPathBuf{{Recursive: false, Path: fooPath}}); err != nil {
			t.Fatalf("AddPaths: %v", err)
		}
		time.Sleep(watcherSettle)

		// Writing a sibling file must NOT produce an event.
		writeFile(t, siblingPath, "unrelated content")
		assertNoEvent(t, w, watchDebounce+100*time.Millisecond, "sibling")

		// Writing the watched file itself must still produce an event.
		writeFile(t, fooPath, "changed")
		found, seen := waitForFile(t, w, "foo", watcherTimeout)
		if !found {
			t.Errorf("expected event for foo; saw: %v", seen)
		}
	})
}

// drainEvents discards any pending events already in the Events channel.
func drainEvents(w *Watch) {
	for {
		select {
		case <-w.Events:
		default:
			return
		}
	}
}

// ---------------------------------------------------------------------------
// walkPathTopo tests — mirrors walk_path_topo_filetree
// ---------------------------------------------------------------------------

// TestWalkPathTopoFileTree mirrors walk_path_topo_filetree.
func TestWalkPathTopoFileTree(t *testing.T) {
	dir := t.TempDir()

	// Create:  a/b  a/c  a/d/e  x/y/z
	for _, pair := range [][2]string{
		{"a", "b"}, {"a", "c"}, {"a/d", "e"}, {"x/y", "z"},
	} {
		d := filepath.Join(dir, pair[0])
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
		touchFile(t, filepath.Join(d, pair[1]))
	}

	result, err := walkPathTopo(dir)
	if err != nil {
		t.Fatalf("walkPathTopo: %v", err)
	}

	// Verify topological ordering: no entry may appear *after* all of its
	// children, i.e. for every pair (i < j), result[j] must not be a prefix
	// of result[i]. In other words: if A is an ancestor of B, A comes first.
	// Mirrors the Rust test's "no later path is a prefix of a previous path".
	for i := range result {
		for j := i + 1; j < len(result); j++ {
			// result[j] (later) must NOT be a prefix of result[i] (earlier).
			if isSubPath(result[i], result[j]) && result[i] != result[j] {
				t.Errorf("topological order violated: %q (index %d) is a prefix of %q (index %d)\nFull list: %v",
					result[j], j, result[i], i, result)
			}
		}
	}

	// Verify all expected paths are present.
	expected := []string{
		dir,
		filepath.Join(dir, "a"),
		filepath.Join(dir, "a", "b"),
		filepath.Join(dir, "a", "c"),
		filepath.Join(dir, "a", "d"),
		filepath.Join(dir, "a", "d", "e"),
		filepath.Join(dir, "x"),
		filepath.Join(dir, "x", "y"),
		filepath.Join(dir, "x", "y", "z"),
	}

	resultSet := make(map[string]struct{}, len(result))
	for _, p := range result {
		resultSet[p] = struct{}{}
	}
	for _, exp := range expected {
		if _, ok := resultSet[exp]; !ok {
			t.Errorf("expected path missing from result: %s", exp)
		}
	}

	if len(result) != len(expected) {
		t.Errorf("expected %d paths, got %d: %v", len(expected), len(result), result)
	}
}
