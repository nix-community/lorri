package main

// JSON-newline framing over a net.Conn.
// Mirrors src/socket/read_writer.rs ReadWriter<R,W>.
//
// Protocol: each message is a single line of JSON terminated by '\n'.
// A context.Context is used for timeouts instead of Rust's subtractive Timeout.

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"time"
)

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
