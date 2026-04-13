package main

// Content-addressable store for lorri.
// Mirrors src/cas.rs: ContentAddressable.
//
// Files are stored under <storeDir>/<md5hex> and written atomically.
// The same content always produces the same filename, so concurrent
// writes of identical content are harmless (last write wins, same bytes).

import (
	"crypto/md5" //nolint:gosec // MD5 used for content-addressing, not cryptographic security
	"fmt"
	"os"
	"path/filepath"
)

// CAS is a content-addressable file store keyed by MD5 hash.
type CAS struct {
	storeDir AbsPath
}

// NewCAS creates a CAS rooted at storeDir, creating the directory if needed.
func NewCAS(storeDir AbsPath) (*CAS, error) {
	if err := os.MkdirAll(string(storeDir), 0o755); err != nil {
		return nil, fmt.Errorf("cannot create CAS directory %s: %w", storeDir, err)
	}
	return &CAS{storeDir: storeDir}, nil
}

// FileFromString stores content in the CAS and returns the absolute path to
// the stored file. The write is atomic: a temp file is written and then
// renamed into place, so readers never see partial content.
//
// If a file with the same content already exists, the existing path is
// returned without any I/O (idempotent).
func (c *CAS) FileFromString(content string) (AbsPath, error) {
	//nolint:gosec // MD5 for content-addressing only
	hash := md5.Sum([]byte(content))
	name := fmt.Sprintf("%x", hash)
	dest := AbsPath(filepath.Join(string(c.storeDir), name))

	// Fast path: already exists.
	if _, err := os.Stat(string(dest)); err == nil {
		return dest, nil
	}

	// Write to a temp file in the same directory, then rename atomically.
	tmp, err := os.CreateTemp(string(c.storeDir), "cas-tmp-")
	if err != nil {
		return "", fmt.Errorf("CAS: cannot create temp file: %w", err)
	}
	tmpName := tmp.Name()

	if _, err := tmp.WriteString(content); err != nil {
		tmp.Close()
		os.Remove(tmpName)
		return "", fmt.Errorf("CAS: cannot write temp file: %w", err)
	}
	if err := tmp.Close(); err != nil {
		os.Remove(tmpName)
		return "", fmt.Errorf("CAS: cannot close temp file: %w", err)
	}

	// os.Rename is atomic on Linux/macOS (same filesystem, which is guaranteed
	// because tmpName is in storeDir).
	if err := os.Rename(tmpName, string(dest)); err != nil {
		os.Remove(tmpName)
		return "", fmt.Errorf("CAS: cannot rename into place: %w", err)
	}

	return dest, nil
}
