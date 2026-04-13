package main

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
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

	// Directory for the content-addressable store.
	// ~/.cache/lorri/cas/
	CASDir AbsPath

	// Path to the SQLite database.
	// ~/.cache/lorri/lorri.sqlite
	SQLiteDB AbsPath
}

// InitPaths computes all lorri paths from the user's XDG/home directories,
// creating the necessary directories.
func InitPaths() (*Paths, error) {
	cacheDir, err := lorriCacheDir()
	if err != nil {
		return nil, fmt.Errorf("cannot determine lorri cache directory: %w", err)
	}

	gcRootDir := AbsPath(filepath.Join(cacheDir, "gc_roots"))
	casDir := AbsPath(filepath.Join(cacheDir, "cas"))
	sqliteDB := AbsPath(filepath.Join(cacheDir, "lorri.sqlite"))

	// Create gc_roots dir
	if err := os.MkdirAll(string(gcRootDir), 0o755); err != nil {
		return nil, fmt.Errorf("could not create GC roots directory %s: %w", gcRootDir, err)
	}

	// Create CAS dir
	if err := os.MkdirAll(string(casDir), 0o755); err != nil {
		return nil, fmt.Errorf("could not create CAS directory %s: %w", casDir, err)
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
		CASDir:           casDir,
		SQLiteDB:         sqliteDB,
	}, nil
}

// lorriCacheDir returns the lorri-specific cache directory.
// Uses os.UserCacheDir() + "/lorri".
func lorriCacheDir() (string, error) {
	base, err := os.UserCacheDir()
	if err != nil {
		return "", err
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
