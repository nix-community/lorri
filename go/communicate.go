package main

// IPC protocol types and client handshake.
// Mirrors src/socket/communicate.rs.
//
// Wire protocol (both directions):
//   1. Client connects.
//   2. Client sends: CommunicationType (JSON line).
//   3. Server replies: ConnectionAccepted (JSON line).
//   4. Client sends its typed request; server sends typed response(s).

import (
	"encoding/json"
	"fmt"
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
