package main

// Daemon: multi-project supervisor.
// Mirrors src/daemon.rs Daemon + src/daemon/server.rs Server.
//
// Architecture:
//   Accept loop (goroutine per connection)
//       │
//       ├─ Ping        → activityCh → buildInstructionHandler goroutine
//       ├─ StreamEvents → hub.Subscribe() → stream events to client
//       ├─ StreamSnap  → hub SnapshotListener → one-shot reply
//       └─ DaemonInfo  → immediate reply
//
//   buildInstructionHandler:
//       map[nixFile → pingCh]
//       on new project: NewBuildLoop → go Forever(hubCh, pingCh)
//       on existing+Always: pingCh <- struct{}{}
//       on existing+OnlyIfNotYetWatching: no-op

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os"
	"os/signal"
	"syscall"
)

// IndicateActivity is sent from connection handlers to buildInstructionHandler.
type IndicateActivity struct {
	ProjectFile ProjectFile
	Rebuild     Rebuild
}

// Daemon supervises multiple BuildLoops and routes IPC.
type Daemon struct {
	extraNixOpts NixOptions
}

// NewDaemon creates a Daemon with the given extra Nix options.
func NewDaemon(opts NixOptions) *Daemon {
	return &Daemon{extraNixOpts: opts}
}

// ServeContext binds the Unix socket and runs the daemon until ctx is cancelled
// or a signal is received.
// Mirrors daemon.rs Daemon::serve().
func (d *Daemon) ServeContext(ctx context.Context, paths *Paths, rtc string, lorriBin string) error {
	// Open SQLite database.
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return fmt.Errorf("daemon: open db: %w", err)
	}
	defer db.Close()

	// Bind socket with flock.
	socketPath := NewSocketPath(paths.DaemonSocketFile)
	listener, lock, err := socketPath.Bind()
	if err != nil {
		return fmt.Errorf("daemon: bind socket: %w", err)
	}
	defer lock.Release()
	defer listener.Close()

	// Start EventHub.
	hub := NewEventHub()
	hubCh := make(chan LoopHandlerEvent, 256)
	go hub.Run(hubCh)

	// Start build instruction handler.
	activityCh := make(chan IndicateActivity, 10)
	handlerDone := make(chan struct{})
	go func() {
		defer close(handlerDone)
		d.buildInstructionHandler(ctx, activityCh, hubCh, db, paths, rtc, lorriBin)
	}()

	// Wrap ctx with signal cancellation.
	ctx, stop := signal.NotifyContext(ctx, syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	// Accept loop — one goroutine per connection.
	acceptErrCh := make(chan error, 1)
	go func() {
		for {
			conn, err := listener.Accept()
			if err != nil {
				acceptErrCh <- err
				return
			}
			go d.handleConn(ctx, conn, activityCh, hub, hubCh)
		}
	}()

	fmt.Fprintln(os.Stderr, "lorri: daemon ready")

	select {
	case <-ctx.Done():
		fmt.Fprintln(os.Stderr, "lorri: daemon shutting down")
		// Stop the build instruction handler and wait for it to close all build loops.
		close(activityCh)
		<-handlerDone
		return nil
	case err := <-acceptErrCh:
		return fmt.Errorf("daemon: accept: %w", err)
	}
}

// handleConn serves one client connection.
// Mirrors server.rs Server::handle_client.
// ctx is the daemon's shutdown context; when it is cancelled the StreamEvents
// handler exits cleanly instead of blocking forever on the subscriber channel.
func (d *Daemon) handleConn(
	ctx context.Context,
	conn net.Conn,
	activityCh chan<- IndicateActivity,
	hub *EventHub,
	hubCh chan<- LoopHandlerEvent,
) {
	defer conn.Close()
	framing := NewFraming(conn)

	// Handshake: read CommunicationType, write ConnectionAccepted.
	var commType CommunicationType
	if err := framing.ReadMsg(defaultReadTimeout, &commType); err != nil {
		return
	}
	if err := framing.WriteMsg(defaultReadTimeout, ConnectionAccepted{}); err != nil {
		return
	}

	log.Printf("connection: %s", commType)

	switch commType {
	case CommPing:
		var req PingRequest
		if err := framing.ReadMsg(defaultReadTimeout, &req); err != nil {
			return
		}
		nixFile := nixFilePathForProject(req.ProjectFile)
		log.Printf("ping: %s (rebuild=%s)", nixFile, req.Rebuild)
		activityCh <- IndicateActivity{
			ProjectFile: req.ProjectFile,
			Rebuild:     req.Rebuild,
		}

	case CommStreamEvents:
		// Client sends an empty StreamEvents{} request first.
		var req struct{}
		if err := framing.ReadMsg(defaultReadTimeout, &req); err != nil {
			return
		}
		log.Printf("stream-events: client connected")
		// Subscribe and stream events until the client disconnects or the
		// daemon shuts down. We select on ctx.Done() so this goroutine
		// exits promptly on graceful shutdown rather than blocking forever.
		subCh, subID := hub.Subscribe()
		defer hub.Unsubscribe(subID)
		for {
			select {
			case <-ctx.Done():
				log.Printf("stream-events: daemon shutting down, closing client")
				return
			case ev, ok := <-subCh:
				if !ok {
					log.Printf("stream-events: subscriber channel closed")
					return
				}
				out, err := buildEventToJSON(ev)
				if err != nil {
					return
				}
				if err := framing.WriteMsg(defaultReadTimeout, json.RawMessage(out)); err != nil {
					log.Printf("stream-events: client disconnected")
					return
				}
			}
		}

	case CommStreamSnapshot:
		// No request body for snapshot — server pushes immediately.
		snapCh := make(chan EventSnapshot, 1)
		hubCh <- LoopHandlerEvent{SnapshotListener: snapCh}
		snap, ok := <-snapCh
		if !ok {
			return
		}
		log.Printf("snapshot: returning %d event(s)", len(snap.Snapshot))
		// Encode each event in the snapshot using the formatted wire shape.
		wireSnap, err := snapshotToWire(snap)
		if err != nil {
			return
		}
		// Best-effort: client may have disconnected.
		_ = framing.WriteMsg(defaultReadTimeout, wireSnap)

	case CommDaemonInfo:
		var req struct{}
		// Best-effort read/write: client may have already disconnected.
		_ = framing.ReadMsg(defaultReadTimeout, &req)
		_ = framing.WriteMsg(defaultReadTimeout, struct{}{})
	}
}

// buildInstructionHandler processes IndicateActivity messages and manages
// per-project BuildLoops.
// Mirrors daemon.rs Daemon::build_instruction_handler.
// Runs in a single goroutine — no locking needed for handlerPings.
// Returns when activityCh is closed; on return all build loop ping channels
// are closed to stop the build loop goroutines.
func (d *Daemon) buildInstructionHandler(
	ctx context.Context,
	activityCh <-chan IndicateActivity,
	hubCh chan<- LoopHandlerEvent,
	db *LorriDB,
	paths *Paths,
	rtc string,
	lorriBin string,
) {
	// nixFile path → ping channel for the running BuildLoop.
	handlerPings := make(map[string]chan<- struct{})
	defer func() {
		// Stop all build loops by closing their ping channels.
		for _, ch := range handlerPings {
			close(ch)
		}
	}()

	for activity := range activityCh {
		nixFile := nixFilePathForProject(activity.ProjectFile)
		isFlake := activity.ProjectFile.FlakeNix != nil
		installable := ""
		if isFlake {
			installable = activity.ProjectFile.FlakeNix.Installable
		}

		// Persist to SQLite (idempotent).
		if err := db.UpsertProject(nixFile, isFlake, installable); err != nil {
			fmt.Fprintf(os.Stderr, "lorri: db upsert %s: %v\n", nixFile, err)
		}
		// Auto-GC projects whose nix files have been deleted.
		if err := db.AutoGCRemovedProjects(); err != nil {
			fmt.Fprintf(os.Stderr, "lorri: auto-gc: %v\n", err)
		}

		pingCh, exists := handlerPings[nixFile]

		switch {
		case exists && activity.Rebuild == RebuildAlways:
			// Wake the existing build loop.
			select {
			case pingCh <- struct{}{}:
			default:
				// Channel full — build loop already has a ping pending.
			}

		case exists && activity.Rebuild == RebuildOnlyIfNotYetWatching:
			// Already watching; no-op.

		case !exists:
			log.Printf("new project: %s", nixFile)
			cfg := BuildLoopConfig{
				ProjectFile:    activity.ProjectFile,
				NixFile:        nixFile,
				LoggedEvalFile: paths.LoggedEvalFile,
				Opts:           d.extraNixOpts,
				RunTimeClosure: rtc,
				LorriBin:       lorriBin,
				GCRootDir:      paths.GCRootDir,
			}
			bl, err := NewBuildLoop(cfg)
			if err != nil {
				hubCh <- LoopHandlerEvent{BuildEvent: &Event{
					Failure: &EventFailure{
						NixFile:    nixFile,
						FailureRaw: marshalFailureRaw(fmt.Errorf("could not start watcher for %s: %w", nixFile, err)),
					},
				}}
				continue
			}
			rx := make(chan struct{}, 10)
			handlerPings[nixFile] = rx
			go bl.Forever(hubCh, rx)
			// Initial ping.
			rx <- struct{}{}
		}
	}
}

// ---------------------------------------------------------------------------
// Wire-format helpers
// ---------------------------------------------------------------------------

// buildEventToJSON encodes an Event in the formatted wire shape that both
// the Rust daemon and our stream-events_ client expect.
// Mirrors ops.rs build_event_to_json.
func buildEventToJSON(ev Event) ([]byte, error) {
	var out any
	switch {
	case ev.Started != nil:
		var reason any
		if ev.Started.Reason.IsPingReceived() {
			reason = map[string]any{"PingReceived": map[string]any{}}
		} else {
			reason = map[string]any{"FilesChanged": ev.Started.Reason.FilesChanged()}
		}
		out = map[string]any{
			"Started": map[string]any{
				"nix_file": ev.Started.NixFile,
				"reason":   reason,
			},
		}
	case ev.Completed != nil:
		out = map[string]any{
			"Completed": map[string]any{
				"nix_file": ev.Completed.NixFile,
				"rooted_output_paths": map[string]any{
					"shell_gc_root": ev.Completed.RootedOutputPaths.ShellGCRoot,
				},
			},
		}
	case ev.Failure != nil:
		out = map[string]any{
			"Failure": map[string]any{
				"nix_file": ev.Failure.NixFile,
				"failure":  map[string]any{"message": ev.Failure.Message()},
			},
		}
	default:
		return nil, fmt.Errorf("unknown event shape")
	}
	return json.Marshal(out)
}

// snapshotToWire encodes an EventSnapshot so each Event uses the formatted shape.
func snapshotToWire(snap EventSnapshot) (any, error) {
	type wireSnap struct {
		Snapshot []json.RawMessage `json:"snapshot"`
	}
	ws := wireSnap{}
	for _, ev := range snap.Snapshot {
		b, err := buildEventToJSON(ev)
		if err != nil {
			return nil, err
		}
		ws.Snapshot = append(ws.Snapshot, json.RawMessage(b))
	}
	return ws, nil
}
