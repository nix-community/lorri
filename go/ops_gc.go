package main

// op_gc: list and remove lorri GC roots.
// Mirrors src/ops.rs op_gc() + gc_find_roots_to_remove() + gc_remove_roots().

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"time"
)

// GCRootInfo holds metadata about one GC root entry.
// Mirrors project.rs GcRootInfo.
type GCRootInfo struct {
	// GCDir is the per-project directory under ~/.cache/lorri/gc_roots/<hash>/
	GCDir string
	// NixFile is the absolute path to the nix file.
	NixFile string
	// Timestamp is the mtime of the shell_gc_root symlink, or nil if unknown.
	Timestamp *time.Time
	// ProjectExists reports whether NixFile still exists on disk.
	ProjectExists bool
}

// formatPrettyOneline formats a GCRootInfo for human display.
// Mirrors GcRootInfo::format_pretty_oneline().
func (g GCRootInfo) formatPrettyOneline() string {
	age := "sometime in the past"
	if g.Timestamp != nil {
		d := time.Since(*g.Timestamp)
		age = prettyTimeAgo(d)
	}
	alive := ""
	if !g.ProjectExists {
		alive = "[gone] "
	}
	return fmt.Sprintf("%s -> %s %s(%s)", g.GCDir, g.NixFile, alive, age)
}

// prettyTimeAgo formats a duration as a human-friendly string.
func prettyTimeAgo(d time.Duration) string {
	switch {
	case d < time.Minute:
		return fmt.Sprintf("%ds ago", int(d.Seconds()))
	case d < time.Hour:
		return fmt.Sprintf("%dm ago", int(d.Minutes()))
	case d < 24*time.Hour:
		return fmt.Sprintf("%dh ago", int(d.Hours()))
	default:
		return fmt.Sprintf("%dd ago", int(d.Hours()/24))
	}
}

// listGCRoots reads all GC roots from the database and enriches them with
// filesystem metadata.
func listGCRoots(paths *Paths) ([]GCRootInfo, error) {
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return nil, fmt.Errorf("gc: open db: %w", err)
	}
	defer db.Close()

	projects, err := db.ListProjects()
	if err != nil {
		return nil, fmt.Errorf("gc: list projects: %w", err)
	}

	var infos []GCRootInfo
	for _, p := range projects {
		var pf ProjectFile
		if p.IsFlake {
			dir := filepath.Dir(p.NixFile) // context dir from flake.nix path
			pf = NewFlakeProjectFile(mustAbsPath(dir), p.FlakeInstallable)
		} else {
			pf = NewShellNixProjectFile(mustAbsPath(p.NixFile))
		}

		gcRoot := gcRootPathForProject(paths.GCRootDir, pf)
		// GCDir is two levels up from shell_gc_root: <hash>/gc_root/shell_gc_root
		gcDir := gcRoot.Dir().Dir()

		// Read mtime of the shell_gc_root symlink.
		var ts *time.Time
		if fi, err := os.Lstat(gcRoot.String()); err == nil {
			t := fi.ModTime()
			ts = &t
		}

		infos = append(infos, GCRootInfo{
			GCDir:         gcDir.String(),
			NixFile:       p.NixFile,
			Timestamp:     ts,
			ProjectExists: fileExists(p.NixFile),
		})
	}

	// Sort most-recently-built last (mirrors ListRootsSort::MoreRecentLast).
	// Entries with no timestamp sort first.
	sortGCRootInfos(infos)
	return infos, nil
}

func sortGCRootInfos(infos []GCRootInfo) {
	// Simple insertion sort — lists are short.
	for i := 1; i < len(infos); i++ {
		for j := i; j > 0; j-- {
			if gcRootInfoLess(infos[j-1], infos[j]) {
				break
			}
			infos[j-1], infos[j] = infos[j], infos[j-1]
		}
	}
}

func gcRootInfoLess(a, b GCRootInfo) bool {
	// nil timestamp sorts first (oldest).
	if a.Timestamp == nil && b.Timestamp == nil {
		return a.NixFile < b.NixFile
	}
	if a.Timestamp == nil {
		return true
	}
	if b.Timestamp == nil {
		return false
	}
	// More recent = larger time = sorts last.
	if !a.Timestamp.Equal(*b.Timestamp) {
		return a.Timestamp.Before(*b.Timestamp)
	}
	return a.NixFile < b.NixFile
}

// opGCInfo lists GC roots, optionally as JSON.
func opGCInfo(paths *Paths, jsonOutput bool) error {
	infos, err := listGCRoots(paths)
	if err != nil {
		return err
	}
	if jsonOutput {
		return writeGCInfoJSON(infos)
	}
	for _, info := range infos {
		fmt.Println(info.formatPrettyOneline())
	}
	return nil
}

// wireTimestamp mirrors Rust's SystemTime serde serialization default:
// {"secs_since_epoch": N, "nanos_since_epoch": N}.
type wireTimestamp struct {
	Secs  int64 `json:"secs_since_epoch"`
	Nanos int64 `json:"nanos_since_epoch"`
}

func toWireTimestamp(t *time.Time) *wireTimestamp {
	if t == nil {
		return nil
	}
	return &wireTimestamp{
		Secs:  t.Unix(),
		Nanos: int64(t.Nanosecond()),
	}
}

func writeGCInfoJSON(infos []GCRootInfo) error {
	type jsonRoot struct {
		GCDir     string         `json:"gc_dir"`
		NixFile   string         `json:"nix_file"`
		Timestamp *wireTimestamp `json:"timestamp"`
		Alive     bool           `json:"alive"`
	}
	out := make([]jsonRoot, len(infos))
	for i, info := range infos {
		out[i] = jsonRoot{
			GCDir:     info.GCDir,
			NixFile:   info.NixFile,
			Timestamp: toWireTimestamp(info.Timestamp),
			Alive:     info.ProjectExists,
		}
	}
	return json.NewEncoder(os.Stdout).Encode(out)
}

// GCRmOptions holds options for `lorri gc rm`.
type GCRmOptions struct {
	ShellFiles []string
	All        bool
	OlderThan  *time.Duration
	DryRun     bool
	JSON       bool
}

// gcFilterRoots returns the subset of infos that should be removed given opts.
// Extracted from opGCRm so tests can call it directly.
func gcFilterRoots(infos []GCRootInfo, opts GCRmOptions) []GCRootInfo {
	shellFileSet := make(map[string]bool, len(opts.ShellFiles))
	for _, f := range opts.ShellFiles {
		if abs, err := NewAbsPathFromCwd(f); err == nil {
			shellFileSet[abs.String()] = true
		}
	}

	var toRemove []GCRootInfo
	for _, info := range infos {
		if opts.All {
			toRemove = append(toRemove, info)
			continue
		}
		if !info.ProjectExists {
			toRemove = append(toRemove, info)
			continue
		}
		if shellFileSet[info.NixFile] {
			toRemove = append(toRemove, info)
			continue
		}
		if opts.OlderThan != nil && info.Timestamp != nil {
			if time.Since(*info.Timestamp) > *opts.OlderThan {
				toRemove = append(toRemove, info)
				continue
			}
		}
		if opts.OlderThan != nil && info.Timestamp == nil {
			toRemove = append(toRemove, info)
		}
	}
	return toRemove
}

// opGCRm removes GC roots matching the given criteria.
func opGCRm(paths *Paths, opts GCRmOptions) error {
	infos, err := listGCRoots(paths)
	if err != nil {
		return err
	}

	// Resolve --shell-file paths before filtering (needs error handling).
	shellFileSet := make(map[string]bool, len(opts.ShellFiles))
	for _, f := range opts.ShellFiles {
		abs, err := NewAbsPathFromCwd(f)
		if err != nil {
			return fmt.Errorf("--shell-file: %w", err)
		}
		shellFileSet[abs.String()] = true
	}

	// opts.ShellFiles are already resolved into shellFileSet above;
	// pass them as absolute paths to gcFilterRoots.
	resolvedOpts := opts
	resolvedOpts.ShellFiles = make([]string, 0, len(shellFileSet))
	for k := range shellFileSet {
		resolvedOpts.ShellFiles = append(resolvedOpts.ShellFiles, k)
	}
	toRemove := gcFilterRoots(infos, resolvedOpts)

	if opts.DryRun {
		if len(toRemove) == 0 {
			fmt.Println("--dry-run: Would not delete any GC roots")
		} else {
			fmt.Println("--dry-run: Would delete the following GC roots:")
			for _, info := range toRemove {
				fmt.Println(info.formatPrettyOneline())
			}
		}
		return nil
	}

	// Actually remove.
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return fmt.Errorf("gc rm: open db: %w", err)
	}
	defer db.Close()

	type rmResult struct {
		info GCRootInfo
		err  error
	}
	var results []rmResult
	for _, info := range toRemove {
		var rerr error
		if err := os.RemoveAll(info.GCDir); err != nil {
			rerr = fmt.Errorf("remove %s: %w", info.GCDir, err)
		} else if err := db.DeleteProject(info.NixFile); err != nil {
			rerr = fmt.Errorf("db delete %s: %w", info.NixFile, err)
		}
		results = append(results, rmResult{info, rerr})
	}

	if opts.JSON {
		type jsonRmRoot struct {
			GCDir     string         `json:"gc_dir"`
			NixFile   string         `json:"nix_file"`
			Timestamp *wireTimestamp `json:"timestamp"`
			Alive     bool           `json:"alive"`
		}
		type jsonRmEntry struct {
			Root  jsonRmRoot `json:"root"`
			Error *string    `json:"error"`
		}
		out := make([]jsonRmEntry, len(results))
		for i, r := range results {
			var errStr *string
			if r.err != nil {
				s := r.err.Error()
				errStr = &s
			}
			out[i] = jsonRmEntry{
				Root: jsonRmRoot{
					GCDir:     r.info.GCDir,
					NixFile:   r.info.NixFile,
					Timestamp: toWireTimestamp(r.info.Timestamp),
					Alive:     r.info.ProjectExists,
				},
				Error: errStr,
			}
		}
		return json.NewEncoder(os.Stdout).Encode(out)
	}

	var ok, errCount int
	for _, r := range results {
		if r.err != nil {
			fmt.Fprintf(os.Stderr, "lorri: failed to remove gc root %s: %v\n", r.info.GCDir, r.err)
			errCount++
		} else {
			ok++
		}
	}
	fmt.Printf("Removed %d gc roots.\n", ok)
	if ok > 0 {
		fmt.Println("Remember to run nix-collect-garbage to actually free space.")
	}
	return nil
}

// parseDuration parses human-friendly durations: 30d, 2m, 1y.
// Mirrors cli.rs human_friendly_duration().
func parseDuration(s string) (time.Duration, error) {
	if len(s) < 2 {
		return 0, fmt.Errorf("invalid duration %q: should end with d, m or y", s)
	}
	suffix := s[len(s)-1]
	numStr := s[:len(s)-1]
	n, err := strconv.ParseUint(numStr, 10, 64)
	if err != nil {
		return 0, fmt.Errorf("invalid duration %q: %q is not an integer: %v", s, numStr, err)
	}
	var secs uint64
	switch suffix {
	case 'd':
		secs = n * 24 * 60 * 60
	case 'm':
		secs = n * 30 * 24 * 60 * 60
	case 'y':
		secs = n * 365 * 24 * 60 * 60
	default:
		return 0, fmt.Errorf("invalid duration %q: should end with d, m or y", s)
	}
	return time.Duration(secs) * time.Second, nil
}
