package main

// op_ping: tell the lorri daemon to start watching (and building) a project.
// Mirrors src/ops.rs op_ping().
//
// The Ping communication type is one-way: client sends the Ping message,
// the daemon does not reply (Response = NoMessage in Rust).

import (
	"fmt"
)

// opPing connects to the daemon and sends a Ping with RebuildAlways.
// It is a fire-and-forget: the daemon does not reply to pings.
func opPing(paths *Paths, projectFile ProjectFile) error {
	return opPingWithRebuild(paths, projectFile, RebuildAlways)
}

// opPingWithRebuild is the general form of opPing, allowing the caller to
// choose the rebuild policy.  Used by opDirenv (OnlyIfNotYetWatching) and
// opPing (Always).
func opPingWithRebuild(paths *Paths, projectFile ProjectFile, rebuild Rebuild) error {
	socketPath := NewSocketPath(paths.DaemonSocketFile)

	framing, err := connectClient(socketPath, CommPing, defaultReadTimeout)
	if err != nil {
		return fmt.Errorf("lorri ping: %w", err)
	}
	defer framing.Close()

	req := PingRequest{
		ProjectFile: projectFile,
		Rebuild:     rebuild,
	}

	if err := framing.WriteMsg(defaultReadTimeout, req); err != nil {
		return fmt.Errorf("lorri ping: send ping: %w", err)
	}

	// No reply expected — Ping::Response is NoMessage in Rust.
	return nil
}
