package main

// op_stream_events: connect to the daemon and print build events as JSON lines.
// Mirrors src/ops.rs op_stream_events().
//
// Three modes:
//   live     — connect as StreamEvents, read indefinitely (until Ctrl-C)
//   snapshot — connect as StreamSnapshot, print already-seen events, exit
//   all      — snapshot first, then switch to live stream

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"
)

// EventKind mirrors cli.rs EventKind.
type EventKind string

const (
	EventKindLive     EventKind = "live"
	EventKindSnapshot EventKind = "snapshot"
	EventKindAll      EventKind = "all"
)

// opStreamEvents connects to the daemon and streams build events to stdout.
// Each event is emitted as a single JSON line (matching Rust output format).
func opStreamEvents(paths *Paths, kind EventKind) error {
	socketPath := NewSocketPath(paths.DaemonSocketFile)

	// Handle Ctrl-C gracefully — exit 0 like the Rust version (killed by signal).
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	done := make(chan struct{})
	go func() {
		select {
		case <-sig:
			os.Exit(0)
		case <-done:
		}
	}()
	defer close(done)

	// ── snapshot phase ──────────────────────────────────────────────────────
	if kind == EventKindSnapshot || kind == EventKindAll {
		if err := printSnapshot(socketPath); err != nil {
			return err
		}
		if kind == EventKindSnapshot {
			return nil
		}
	}

	// ── live phase ───────────────────────────────────────────────────────────
	return streamLive(socketPath)
}

// printSnapshot connects as StreamSnapshot, reads the one-shot snapshot,
// and prints each event as a JSON line.
//
// The snapshot wire format uses the formatted event shape (buildEventToJSON),
// so we decode it as raw JSON and print each event directly — no re-encoding.
func printSnapshot(socketPath SocketPath) error {
	framing, err := connectClient(socketPath, CommStreamSnapshot, defaultReadTimeout)
	if err != nil {
		return fmt.Errorf("stream-events snapshot: %w", err)
	}
	defer framing.Close()

	// The server pushes the snapshot immediately after the handshake.
	// Decode as {"snapshot": [<raw-json-event>, ...]} to avoid trying to
	// unmarshal the formatted wire shape into the typed EventSnapshot struct.
	var wireSnap struct {
		Snapshot []json.RawMessage `json:"snapshot"`
	}
	if err := framing.ReadMsg(defaultReadTimeout, &wireSnap); err != nil {
		return fmt.Errorf("stream-events snapshot: read: %w", err)
	}

	for _, raw := range wireSnap.Snapshot {
		line := append([]byte(raw), '\n')
		if _, err := os.Stdout.Write(line); err != nil {
			return err
		}
	}
	return nil
}

// streamLive connects as StreamEvents and reads events indefinitely.
func streamLive(socketPath SocketPath) error {
	// Infinite timeout for the live stream.
	framing, err := connectClient(socketPath, CommStreamEvents, defaultReadTimeout)
	if err != nil {
		return fmt.Errorf("stream-events live: %w", err)
	}
	defer framing.Close()

	// StreamEvents: client sends StreamEvents{} request first, then reads.
	if err := framing.WriteMsg(defaultReadTimeout, struct{}{}); err != nil {
		return fmt.Errorf("stream-events live: send request: %w", err)
	}

	for {
		// Decode as raw JSON and write directly — no re-encoding needed since
		// the daemon already sends the formatted wire shape.
		var raw json.RawMessage
		if err := framing.ReadMsg(0, &raw); err != nil {
			if err == io.EOF {
				return nil // daemon closed connection
			}
			return fmt.Errorf("stream-events live: read: %w", err)
		}
		line := append([]byte(raw), '\n')
		if _, err := os.Stdout.Write(line); err != nil {
			return err
		}
	}
}
