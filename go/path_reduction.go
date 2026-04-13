package main

// ReducePaths removes redundant watched paths.
// Mirrors src/pathreduction.rs reduce_paths().
//
// Rules applied in order to each path:
//   1. reduceChannelPath  — collapse nix channel paths to their switchable root
//   2. reduceNixStorePath — discard /nix/store/… paths (immutable, no need to watch)
//
// After individual reduction, the set is deduplicated and any path that is a
// sub-path of another already-accepted path is dropped.

import (
	"path/filepath"
	"sort"
	"strings"
)

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
