package main

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
)

// Paths holds all the filesystem locations lorri uses.
// Mirrors src/constants.rs Paths struct.
type Paths struct {
	// Directory where per-project GC root symlinks are stored.
	// ~/.cache/lorri/gc_roots/
	GCRootDir AbsPath

	// Path to the Unix domain socket for the daemon.
	// $XDG_RUNTIME_DIR/lorri/daemon.socket  (Linux)
	// ~/.cache/lorri/daemon.socket           (macOS fallback)
	DaemonSocketFile AbsPath

	// Path to the SQLite database.
	// ~/.cache/lorri/lorri.sqlite
	SQLiteDB AbsPath

	// Path to the logged-evaluation.nix instrumentation file.
	// Written from the embedded copy on startup if the content has changed.
	// ~/.cache/lorri/logged-evaluation.nix
	LoggedEvalFile AbsPath
}

// InitPaths computes all lorri paths from the user's XDG/home directories,
// creating the necessary directories.
func InitPaths() (*Paths, error) {
	cacheDir, err := lorriCacheDir()
	if err != nil {
		return nil, fmt.Errorf("cannot determine lorri cache directory: %w", err)
	}

	gcRootDir := AbsPath(filepath.Join(cacheDir, "gc_roots"))
	sqliteDB := AbsPath(filepath.Join(cacheDir, "lorri.sqlite"))

	// Create gc_roots dir
	if err := os.MkdirAll(string(gcRootDir), 0o755); err != nil {
		return nil, fmt.Errorf("could not create GC roots directory %s: %w", gcRootDir, err)
	}

	// Write logged-evaluation.nix only if the content has changed.
	loggedEvalFile := AbsPath(filepath.Join(cacheDir, "logged-evaluation.nix"))
	if err := writeFileIfChanged(string(loggedEvalFile), loggedEvaluationNix, 0o644); err != nil {
		return nil, fmt.Errorf("could not write logged-evaluation.nix: %w", err)
	}

	// Determine socket path
	socketDir, err := lorriRuntimeDir(cacheDir)
	if err != nil {
		return nil, fmt.Errorf("cannot determine lorri runtime directory: %w", err)
	}
	if err := os.MkdirAll(socketDir, 0o755); err != nil {
		return nil, fmt.Errorf("could not create socket directory %s: %w", socketDir, err)
	}
	socketFile := AbsPath(filepath.Join(socketDir, "daemon.socket"))

	return &Paths{
		GCRootDir:        gcRootDir,
		DaemonSocketFile: socketFile,
		SQLiteDB:         sqliteDB,
		LoggedEvalFile:   loggedEvalFile,
	}, nil
}

// writeFileIfChanged writes content to path only if the existing file differs
// (or does not exist). Uses an atomic rename so readers never see partial content.
func writeFileIfChanged(path, content string, perm os.FileMode) error {
	existing, err := os.ReadFile(path)
	if err == nil && bytes.Equal(existing, []byte(content)) {
		return nil // already up to date
	}

	dir := filepath.Dir(path)
	tmp, err := os.CreateTemp(dir, ".lorri-tmp-")
	if err != nil {
		return err
	}
	tmpName := tmp.Name()
	needsRemove := true
	defer func() {
		if needsRemove {
			os.Remove(tmpName)
		}
	}()

	if _, err := tmp.WriteString(content); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Chmod(perm); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	if err := os.Rename(tmpName, path); err != nil {
		return err
	}
	needsRemove = false // rename succeeded, file is now at its destination
	return nil
}

// lorriCacheDir returns the lorri-specific cache directory.
// Respects XDG_CACHE_HOME if set (all platforms), otherwise falls back to
// os.UserCacheDir() + "/lorri".
func lorriCacheDir() (string, error) {
	var base string
	if xdg := os.Getenv("XDG_CACHE_HOME"); xdg != "" {
		base = xdg
	} else {
		var err error
		base, err = os.UserCacheDir()
		if err != nil {
			return "", err
		}
	}
	dir := filepath.Join(base, "lorri")
	// canonicalize if possible (mirrors Rust's canonicalize attempt)
	if canon, err := filepath.EvalSymlinks(dir); err == nil {
		return canon, nil
	}
	return dir, nil
}

// lorriRuntimeDir returns the directory for the daemon socket.
// On Linux: $XDG_RUNTIME_DIR/lorri  (or cacheDir fallback if unset)
// On other platforms: cacheDir
func lorriRuntimeDir(cacheDir string) (string, error) {
	if runtime.GOOS == "linux" {
		if xdg := os.Getenv("XDG_RUNTIME_DIR"); xdg != "" {
			return filepath.Join(xdg, "lorri"), nil
		}
	}
	// macOS / fallback: use cache dir
	return cacheDir, nil
}

// ---------------------------------------------------------------------------
// Path reduction
// ---------------------------------------------------------------------------

// WatchPathBuf mirrors watch.rs WatchPathBuf.
// Recursive == true means the subtree should be watched recursively;
// false means only the directory listing (non-recursive).
type WatchPathBuf struct {
	Recursive bool
	Path      string
}

// ReducePaths reduces a slice of WatchPathBufs to the minimal set needed to
// detect all relevant changes. Mirrors pathreduction.rs reduce_paths().
func ReducePaths(paths []WatchPathBuf) []WatchPathBuf {
	// Phase 1: apply per-path reducers, collecting survivors directly.
	survivors := make([]WatchPathBuf, 0, len(paths))
	for _, p := range paths {
		if r, ok := reduceChannelPath(p); ok {
			survivors = append(survivors, r)
		} else if !reduceNixStorePath(p) {
			survivors = append(survivors, p)
		}
	}

	// Phase 2: sort by path length (shortest first) so parent paths come first.
	sort.Slice(survivors, func(i, j int) bool {
		pi, pj := survivors[i].Path, survivors[j].Path
		if len(pi) != len(pj) {
			return len(pi) < len(pj)
		}
		return pi < pj
	})

	// Phase 3: dedup exact duplicates first.
	survivors = dedupWatchPaths(survivors)

	// Phase 4: fold — drop any path that starts with a path already in the set.
	// Mirrors the Rust fold with starts_with check.
	result := make([]WatchPathBuf, 0, len(survivors))
	for _, candidate := range survivors {
		dominated := false
		for _, accepted := range result {
			if isSubPath(candidate.Path, accepted.Path) {
				dominated = true
				break
			}
		}
		if !dominated {
			result = append(result, candidate)
		}
	}

	return result
}

// isSubPath returns true if child is equal to parent or is nested under parent.
// Uses a trailing-slash check to avoid false positives (/foo vs /foobar).
func isSubPath(child, parent string) bool {
	if child == parent {
		return true
	}
	// Ensure parent ends with separator before prefix check.
	if !strings.HasSuffix(parent, string(filepath.Separator)) {
		parent += string(filepath.Separator)
	}
	return strings.HasPrefix(child, parent)
}

func dedupWatchPaths(paths []WatchPathBuf) []WatchPathBuf {
	seen := make(map[WatchPathBuf]struct{}, len(paths))
	out := paths[:0]
	for _, p := range paths {
		if _, ok := seen[p]; !ok {
			seen[p] = struct{}{}
			out = append(out, p)
		}
	}
	return out
}

// reduceChannelPath collapses a path under /nix/var/nix/profiles/per-user/
// to the user-level directory that contains the channel symlink.
// Returns (reduced, true) if it has an opinion, (zero, false) otherwise.
// Mirrors pathreduction.rs reduce_channel_path().
func reduceChannelPath(path WatchPathBuf) (WatchPathBuf, bool) {
	const nixProfilePrefix = "/nix/var/nix/profiles/per-user"
	// We need at least 9 path segments:
	// / nix var nix profiles per-user <user> channels <channel>
	const channelVersionRootSegments = 9

	if !strings.HasPrefix(path.Path, nixProfilePrefix) {
		return WatchPathBuf{}, false
	}

	// Split into segments (skip leading empty string from splitting "/foo")
	segments := strings.Split(path.Path, string(filepath.Separator))
	// segments[0] is "" (before the leading slash)
	// real segments start at [1]
	if len(segments) < channelVersionRootSegments+1 {
		return WatchPathBuf{}, false
	}

	// Reconstruct the channel root path: first channelVersionRootSegments components.
	channelRootParts := segments[:channelVersionRootSegments+1] // +1 for leading ""
	channelRootPath := strings.Join(channelRootParts, string(filepath.Separator))

	// Check that canonicalizing both paths gives the same result.
	canonicalChannelRoot, err := filepath.EvalSymlinks(channelRootPath)
	if err != nil {
		return WatchPathBuf{}, false
	}
	canonicalPath, err := filepath.EvalSymlinks(path.Path)
	if err != nil {
		return WatchPathBuf{}, false
	}

	if !strings.HasPrefix(canonicalPath, canonicalChannelRoot) {
		return WatchPathBuf{}, false
	}

	// Reduce to the directory two levels up from channelRootPath:
	// /nix/var/nix/profiles/per-user/<user>  (the user directory)
	reduceTo := filepath.Dir(filepath.Dir(channelRootPath))
	return WatchPathBuf{Recursive: path.Recursive, Path: reduceTo}, true
}

// reduceNixStorePath drops paths that are under /nix/store and whose
// canonical form is also under /nix/store (i.e. truly immutable).
// Returns true if the path should be removed.
// Mirrors pathreduction.rs reduce_nix_store_path().
func reduceNixStorePath(path WatchPathBuf) bool {
	const nixStore = "/nix/store"

	if !strings.HasPrefix(path.Path, nixStore) {
		return false
	}

	canonical, err := filepath.EvalSymlinks(path.Path)
	if err != nil {
		// Can't canonicalize (e.g. path doesn't exist yet) — keep it.
		return false
	}

	return strings.HasPrefix(canonical, nixStore)
}
