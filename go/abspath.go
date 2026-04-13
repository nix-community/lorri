package main

import (
	"fmt"
	"path/filepath"
)

// AbsPath is a path guaranteed to be absolute by construction.
// It is also always clean (no double slashes, no extra ./), but symlinks
// are NOT resolved (matching Rust's AbsPathBuf behaviour).
type AbsPath string

// NewAbsPath validates that path is absolute and returns an AbsPath.
// Returns an error if path is not absolute.
func NewAbsPath(path string) (AbsPath, error) {
	if !filepath.IsAbs(path) {
		return "", fmt.Errorf("not an absolute path: %q", path)
	}
	return AbsPath(filepath.Clean(path)), nil
}

// NewAbsPathFromCwd returns an AbsPath for the given path.
// If path is relative, it is joined to the current working directory.
// Does not check whether the file exists.
func NewAbsPathFromCwd(path string) (AbsPath, error) {
	if filepath.IsAbs(path) {
		return AbsPath(filepath.Clean(path)), nil
	}
	cwd, err := cwdFunc()
	if err != nil {
		return "", fmt.Errorf("cannot get current directory: %w", err)
	}
	return AbsPath(filepath.Clean(filepath.Join(cwd, path))), nil
}

// mustAbsPath panics if path is not absolute. Convenience for known-absolute paths.
func mustAbsPath(path string) AbsPath {
	a, err := NewAbsPath(path)
	if err != nil {
		panic(err)
	}
	return a
}

// String returns the underlying path string.
func (a AbsPath) String() string {
	return string(a)
}

// Join appends elem to a, cleaning the result.
func (a AbsPath) Join(elem ...string) AbsPath {
	parts := make([]string, 0, 1+len(elem))
	parts = append(parts, string(a))
	parts = append(parts, elem...)
	return AbsPath(filepath.Join(parts...))
}

// Dir returns the directory component of a.
func (a AbsPath) Dir() AbsPath {
	return AbsPath(filepath.Dir(string(a)))
}

// Base returns the last element of a.
func (a AbsPath) Base() string {
	return filepath.Base(string(a))
}

// cwdFunc is a variable so tests can override it.
var cwdFunc = func() (string, error) {
	return filepath.Abs(".")
}
