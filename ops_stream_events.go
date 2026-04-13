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
	"errors"
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

	// Handle Ctrl-C gracefully — return nil so the caller can clean up normally.
	// Mirrors the Rust version which exits 0 on SIGINT.
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	defer signal.Stop(sig) // prevent the channel from leaking after return

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
	return streamLive(socketPath, sig)
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
// It returns nil when the daemon closes the connection or a signal is received.
func streamLive(socketPath SocketPath, sig <-chan os.Signal) error {
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

	// Read events in a background goroutine so we can also select on sig.
	type readResult struct {
		raw json.RawMessage
		err error
	}
	readCh := make(chan readResult, 1)
	go func() {
		for {
			var raw json.RawMessage
			err := framing.ReadMsg(0, &raw)
			readCh <- readResult{raw, err}
			if err != nil {
				return
			}
		}
	}()

	for {
		select {
		case <-sig:
			// SIGINT/SIGTERM — exit cleanly, mirroring Rust's exit(0).
			return nil
		case r := <-readCh:
			if r.err != nil {
				if errors.Is(r.err, io.EOF) {
					return nil // daemon closed connection
				}
				return fmt.Errorf("stream-events live: read: %w", r.err)
			}
			line := append([]byte(r.raw), '\n')
			if _, err := os.Stdout.Write(line); err != nil {
				return err
			}
		}
	}
}
