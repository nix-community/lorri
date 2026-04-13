package main

// Unix domain socket bind/connect with flock(2) exclusivity.
// Mirrors src/socket/path.rs SocketPath.

import (
	"fmt"
	"net"
	"os"
	"syscall"
)

// SocketPath wraps an absolute path for a Unix domain socket.
type SocketPath struct {
	path AbsPath
}

// NewSocketPath creates a SocketPath from an absolute path.
func NewSocketPath(p AbsPath) SocketPath {
	return SocketPath{path: p}
}

// String returns the socket file path.
func (s SocketPath) String() string {
	return string(s.path)
}

// lockfilePath returns the path of the lock file (<socket>.lock).
func (s SocketPath) lockfilePath() AbsPath {
	return AbsPath(string(s.path) + ".lock")
}

// SocketLock holds an open, exclusively-flocked file descriptor.
// Drop it (call Release) to release the lock.
type SocketLock struct {
	f *os.File
}

// Release releases the flock and closes the lock file.
func (l *SocketLock) Release() error {
	if l.f == nil {
		return nil
	}
	// flock is released automatically on close; explicit unlock for clarity.
	syscall.Flock(int(l.f.Fd()), syscall.LOCK_UN) //nolint:errcheck
	err := l.f.Close()
	l.f = nil
	return err
}

// Lock tries to acquire an exclusive non-blocking flock on the lock file.
// Returns ErrDaemonAlreadyRunning if another process holds the lock.
func (s SocketPath) Lock() (*SocketLock, error) {
	lockPath := string(s.lockfilePath())
	f, err := os.OpenFile(lockPath, os.O_RDWR|os.O_CREATE, 0o600)
	if err != nil {
		return nil, fmt.Errorf("socket lock: open %s: %w", lockPath, err)
	}

	err = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
	if err != nil {
		f.Close()
		if err == syscall.EWOULDBLOCK {
			return nil, &ErrDaemonAlreadyRunning{LockFile: lockPath}
		}
		return nil, fmt.Errorf("socket lock: flock %s: %w", lockPath, err)
	}

	return &SocketLock{f: f}, nil
}

// ErrDaemonAlreadyRunning is returned when another lorri daemon holds the socket lock.
type ErrDaemonAlreadyRunning struct {
	LockFile string
}

func (e *ErrDaemonAlreadyRunning) Error() string {
	return fmt.Sprintf(
		"another lorri daemon is already running (lock file: %s); is another lorri daemon running?",
		e.LockFile,
	)
}

// Bind acquires the lock, removes any stale socket file, and binds a new
// Unix listener. The caller is responsible for releasing the lock.
//
// Mirrors SocketPath::bind() in Rust.
func (s SocketPath) Bind() (net.Listener, *SocketLock, error) {
	lock, err := s.Lock()
	if err != nil {
		return nil, nil, err
	}

	// Remove stale socket file, ignoring "not found".
	if err := os.Remove(string(s.path)); err != nil && !os.IsNotExist(err) {
		lock.Release() //nolint:errcheck
		return nil, nil, fmt.Errorf("socket bind: remove stale socket: %w", err)
	}

	l, err := net.Listen("unix", string(s.path))
	if err != nil {
		lock.Release() //nolint:errcheck
		return nil, nil, fmt.Errorf("socket bind: listen: %w", err)
	}

	return l, lock, nil
}

// Connect connects to the Unix socket.
// Mirrors SocketPath::connect() in Rust.
func (s SocketPath) Connect() (net.Conn, error) {
	conn, err := net.Dial("unix", string(s.path))
	if err != nil {
		return nil, fmt.Errorf("socket connect %s: %w (is the lorri daemon running?)", s.path, err)
	}
	return conn, nil
}
