package main

// IPC protocol types, client handshake, and Unix socket transport.
// Mirrors src/socket/communicate.rs and src/socket/path.rs.
//
// Wire protocol (both directions):
//   1. Client connects.
//   2. Client sends: CommunicationType (JSON line).
//   3. Server replies: ConnectionAccepted (JSON line).
//   4. Client sends its typed request; server sends typed response(s).

import (
	"bufio"
	"context"
	"crypto/md5" //nolint:gosec // MD5 used for path-keying, not cryptographic security
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
	"syscall"
	"time"
)

// CommunicationType identifies which kind of IPC exchange the client wants.
// Must match the Rust enum exactly (serde rename is default CamelCase).
type CommunicationType string

const (
	CommDaemonInfo     CommunicationType = "DaemonInfo"
	CommPing           CommunicationType = "Ping"
	CommStreamEvents   CommunicationType = "StreamEvents"
	CommStreamSnapshot CommunicationType = "StreamSnapshot"
)

// ConnectionAccepted is the server's handshake acknowledgement.
// Mirrors listener::ConnectionAccepted in Rust.
type ConnectionAccepted struct {
	Accepted string `json:"accepted,omitempty"`
}

// ---------------------------------------------------------------------------
// Request / response message types
// ---------------------------------------------------------------------------

// PingRequest is sent by the client to tell the daemon to watch a project.
// Mirrors communicate::Ping.
type PingRequest struct {
	ProjectFile ProjectFile `json:"project_file"`
	Rebuild     Rebuild     `json:"rebuild"`
}

// Rebuild controls when a ping triggers a rebuild.
type Rebuild string

const (
	RebuildOnlyIfNotYetWatching Rebuild = "OnlyIfNotYetWatching"
	RebuildAlways               Rebuild = "Always"
)

// ProjectFile is the build source descriptor for a watched project.
// Mirrors project::ProjectFile — a serde-tagged enum with two variants.
//
// In JSON, Rust's externally-tagged enums look like:
//
//	{"ShellNix": {"path": "..."}}
//	{"FlakeNix": {"context": "...", "installable": "..."}}
type ProjectFile struct {
	ShellNix *ShellNixFile `json:"ShellNix,omitempty"`
	FlakeNix *FlakeOutput  `json:"FlakeNix,omitempty"`
}

// ShellNixFile wraps an absolute path to a shell.nix (or similar).
// In the Rust wire format NixFile serializes as its inner AbsPathBuf,
// which serializes as a JSON string.
type ShellNixFile struct {
	// The inner NixFile is an AbsPathBuf which serializes as a plain string.
	// We mirror that by embedding it directly.
	path string
}

// MarshalJSON serializes as the bare JSON string (matching Rust's AbsPathBuf).
func (s ShellNixFile) MarshalJSON() ([]byte, error) {
	return json.Marshal(s.path)
}

// UnmarshalJSON deserializes from a bare JSON string.
func (s *ShellNixFile) UnmarshalJSON(b []byte) error {
	var path string
	if err := json.Unmarshal(b, &path); err != nil {
		return fmt.Errorf("ShellNixFile: expected a JSON string: %w", err)
	}
	s.path = path
	return nil
}

// FlakeOutput is a flake installable descriptor.
// Mirrors project::FlakeOutput.
type FlakeOutput struct {
	Context     string `json:"context"`
	Installable string `json:"installable"`
}

// NewShellNixProjectFile creates a ProjectFile for a shell.nix path.
func NewShellNixProjectFile(nixFilePath AbsPath) ProjectFile {
	return ProjectFile{
		ShellNix: &ShellNixFile{path: string(nixFilePath)},
	}
}

// NewFlakeProjectFile creates a ProjectFile for a flake.
func NewFlakeProjectFile(context AbsPath, installable string) ProjectFile {
	return ProjectFile{
		FlakeNix: &FlakeOutput{
			Context:     string(context),
			Installable: installable,
		},
	}
}

// ---------------------------------------------------------------------------
// Event types (mirroring build_loop::Event)
// ---------------------------------------------------------------------------

// Event is a build event streamed from the daemon to clients.
// Mirrors build_loop::Event (externally-tagged serde enum).
type Event struct {
	Started   *EventStarted   `json:"Started,omitempty"`
	Completed *EventCompleted `json:"Completed,omitempty"`
	Failure   *EventFailure   `json:"Failure,omitempty"`
}

// EventStarted carries the nix file and reason for a build start.
type EventStarted struct {
	NixFile string `json:"nix_file"`
	Reason  Reason `json:"reason"`
}

// EventCompleted carries the build output paths.
type EventCompleted struct {
	NixFile           string     `json:"nix_file"`
	RootedOutputPaths OutputPath `json:"rooted_output_paths"`
}

// EventFailure carries the build error.
// Wire: {"nix_file":"...", "failure": <BuildError serde encoding>}
// We store failure as raw JSON and extract the message string for display.
type EventFailure struct {
	NixFile    string          `json:"nix_file"`
	FailureRaw json.RawMessage `json:"failure"`
}

// Message extracts a human-readable string from the failure payload.
// Rust encodes BuildError as a string (Display impl) in build_event_to_json,
// but on the wire it uses serde which may produce a richer structure.
// We try common shapes and fall back to the raw JSON.
func (e *EventFailure) Message() string {
	// Try plain string
	var s string
	if err := json.Unmarshal(e.FailureRaw, &s); err == nil {
		return s
	}
	// Try {"message": "..."}
	var obj struct {
		Message string `json:"message"`
	}
	if err := json.Unmarshal(e.FailureRaw, &obj); err == nil && obj.Message != "" {
		return obj.Message
	}
	return string(e.FailureRaw)
}

// Reason describes why a build was triggered.
// Serde's default externally-tagged encoding:
//
//	unit variant  → bare string:        "PingReceived"
//	tuple variant → {"FilesChanged": [...]};
//
// We use json.RawMessage to decode lazily and inspect ourselves.
type Reason struct {
	raw json.RawMessage
}

func (r *Reason) UnmarshalJSON(b []byte) error {
	r.raw = b
	return nil
}

func (r Reason) MarshalJSON() ([]byte, error) {
	if r.raw == nil {
		return []byte(`"PingReceived"`), nil
	}
	return r.raw, nil
}

// IsPingReceived returns true when the reason is a ping.
func (r Reason) IsPingReceived() bool {
	return string(r.raw) == `"PingReceived"`
}

// FilesChanged returns the list of changed files, or nil for PingReceived.
func (r Reason) FilesChanged() []string {
	// wire: {"FilesChanged": ["path1", "path2"]}
	var wrapper struct {
		FilesChanged []string `json:"FilesChanged"`
	}
	if err := json.Unmarshal(r.raw, &wrapper); err != nil {
		return nil
	}
	return wrapper.FilesChanged
}

// OutputPath mirrors builder::OutputPath wire encoding.
// The wire field is "shell_gc_root" (from OutputPath::serialize via serde).
type OutputPath struct {
	ShellGCRoot string `json:"shell_gc_root"`
}

// EventSnapshot is a snapshot of already-seen events.
type EventSnapshot struct {
	Snapshot []Event `json:"snapshot"`
}

// ---------------------------------------------------------------------------
// ProjectFile helpers
// ---------------------------------------------------------------------------

// nixFilePathForProject returns the nix file path string used for hashing.
// Mirrors ProjectFile::as_absolute_path() → as_nix_file() in Rust:
//
//	ShellNix(path)         → path
//	FlakeNix{context, ...} → context/flake.nix
func nixFilePathForProject(projectFile ProjectFile) string {
	if projectFile.ShellNix != nil {
		return projectFile.ShellNix.path
	}
	if projectFile.FlakeNix != nil {
		return AbsPath(projectFile.FlakeNix.Context).Join("flake.nix").String()
	}
	panic("ProjectFile has neither ShellNix nor FlakeNix set")
}

// gcRootPathForProject computes the shell GC root path for a project.
// Mirrors project.rs Project::new_internal + Project::shell_gc_root:
//
//	hash = md5(nixFilePath bytes)
//	path = <gcRootDir>/<hexhash>/gc_root/shell_gc_root
func gcRootPathForProject(gcRootDir AbsPath, projectFile ProjectFile) AbsPath {
	nixFilePath := nixFilePathForProject(projectFile)
	//nolint:gosec // MD5 for path-keying only
	hash := md5.Sum([]byte(nixFilePath))
	hexHash := fmt.Sprintf("%x", hash)
	return gcRootDir.Join(hexHash, "gc_root", "shell_gc_root")
}

// fileExists reports whether path exists (as any filesystem object).
func fileExists(path string) bool {
	_, err := os.Lstat(path)
	return err == nil
}

// ---------------------------------------------------------------------------
// Client helper: connect and perform handshake
// ---------------------------------------------------------------------------

// connectClient connects to the daemon socket and performs the handshake for
// the given CommunicationType. Returns an open Framing ready for typed I/O.
func connectClient(socketPath SocketPath, commType CommunicationType, timeout time.Duration) (*Framing, error) {
	conn, err := socketPath.Connect()
	if err != nil {
		return nil, err
	}

	framing := NewFraming(conn)

	// Send CommunicationType
	if err := framing.WriteMsg(timeout, commType); err != nil {
		framing.Close()
		return nil, fmt.Errorf("handshake send comm type: %w", err)
	}

	// Read ConnectionAccepted
	var ack ConnectionAccepted
	if err := framing.ReadMsg(timeout, &ack); err != nil {
		framing.Close()
		return nil, fmt.Errorf("handshake read ack: %w", err)
	}

	return framing, nil
}

// ---------------------------------------------------------------------------
// SocketPath — bind/connect
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Framing — JSON-newline read/write
// ---------------------------------------------------------------------------

// defaultReadTimeout mirrors communicate.rs DEFAULT_READ_TIMEOUT (1 second).
const defaultReadTimeout = 1 * time.Second

// Framing wraps a net.Conn and provides typed JSON-newline read/write.
type Framing struct {
	conn    net.Conn
	scanner *bufio.Scanner
}

// NewFraming creates a Framing wrapping conn.
func NewFraming(conn net.Conn) *Framing {
	return &Framing{
		conn:    conn,
		scanner: bufio.NewScanner(conn),
	}
}

// WriteMsg serializes msg as JSON and writes it as a single line.
// If timeout > 0 a write deadline is set.
func (f *Framing) WriteMsg(timeout time.Duration, msg any) error {
	line, err := json.Marshal(msg)
	if err != nil {
		return fmt.Errorf("socket write: marshal: %w", err)
	}
	line = append(line, '\n')

	if timeout > 0 {
		if err := f.conn.SetWriteDeadline(time.Now().Add(timeout)); err != nil {
			return fmt.Errorf("socket write: set deadline: %w", err)
		}
		defer f.conn.SetWriteDeadline(time.Time{}) //nolint:errcheck
	}

	if _, err := f.conn.Write(line); err != nil {
		return fmt.Errorf("socket write: %w", err)
	}
	return nil
}

// ReadMsg reads one JSON line and unmarshals it into dst.
// If timeout > 0 a read deadline is set; if timeout == 0 the connection
// blocks indefinitely (no deadline).
func (f *Framing) ReadMsg(timeout time.Duration, dst any) error {
	if timeout > 0 {
		if err := f.conn.SetReadDeadline(time.Now().Add(timeout)); err != nil {
			return fmt.Errorf("socket read: set deadline: %w", err)
		}
		defer f.conn.SetReadDeadline(time.Time{}) //nolint:errcheck
	} else {
		// Explicitly clear any existing deadline so we block forever.
		f.conn.SetReadDeadline(time.Time{}) //nolint:errcheck
	}

	if !f.scanner.Scan() {
		if err := f.scanner.Err(); err != nil {
			return fmt.Errorf("socket read: %w", err)
		}
		return io.EOF
	}

	if err := json.Unmarshal(f.scanner.Bytes(), dst); err != nil {
		return fmt.Errorf("socket read: unmarshal: %w", err)
	}
	return nil
}

// Communicate sends msg then reads a reply into dst.
// The timeout covers the whole roundtrip.
func (f *Framing) Communicate(ctx context.Context, timeout time.Duration, msg any, dst any) error {
	if err := f.WriteMsg(timeout, msg); err != nil {
		return err
	}
	// remaining time for the read
	remaining := timeout
	if deadline, ok := ctx.Deadline(); ok {
		remaining = time.Until(deadline)
		if remaining <= 0 {
			return fmt.Errorf("socket communicate: context deadline exceeded before read")
		}
	}
	return f.ReadMsg(remaining, dst)
}

// Close closes the underlying connection.
func (f *Framing) Close() error {
	return f.conn.Close()
}
