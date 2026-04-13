package main

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// TestDaemonFullStack is an end-to-end integration test:
//  1. Starts a Go daemon in a goroutine (temp paths, temp socket)
//  2. Sends a ping via opPing
//  3. Opens a stream-events connection via connectClient
//  4. Asserts Started + Completed arrive within timeout
//
// Requires: nix-instantiate, nix-build, nix-store on $PATH and RUN_TIME_CLOSURE set.
func TestDaemonFullStack(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	dir := t.TempDir()

	// Write a minimal shell.nix.
	shellNix := filepath.Join(dir, "shell.nix")
	if err := os.WriteFile(shellNix, []byte(`
derivation {
  name = "lorri-daemon-test";
  builder = "/bin/sh";
  args = [ "-c" "echo > $out" ];
  system = builtins.currentSystem;
  allowSubstitutes = false;
  preferLocalBuild = true;
}
`), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}

	// Set up temp paths.
	paths := &Paths{
		GCRootDir:        mustAbsPath(filepath.Join(dir, "gc_roots")),
		DaemonSocketFile: mustAbsPath(filepath.Join(dir, "daemon.socket")),
		CASDir:           mustAbsPath(filepath.Join(dir, "cas")),
		SQLiteDB:         mustAbsPath(filepath.Join(dir, "lorri.sqlite")),
	}
	for _, d := range []string{
		string(paths.GCRootDir),
		string(paths.CASDir),
	} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatalf("mkdir %s: %v", d, err)
		}
	}

	// Start the daemon with a cancellable context so the test can shut it down.
	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)

	daemon := NewDaemon(EmptyNixOptions())
	daemonErr := make(chan error, 1)
	go func() {
		daemonErr <- daemon.ServeContext(ctx, paths, rtc)
	}()

	// Wait for the socket to appear.
	socketPath := NewSocketPath(paths.DaemonSocketFile)
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		if _, err := os.Stat(paths.DaemonSocketFile.String()); err == nil {
			break
		}
		time.Sleep(20 * time.Millisecond)
	}
	if _, err := os.Stat(paths.DaemonSocketFile.String()); err != nil {
		t.Fatalf("socket never appeared: %v", err)
	}

	// Open a stream-events connection before pinging so we don't miss events.
	streamFraming, err := connectClient(socketPath, CommStreamEvents, defaultReadTimeout)
	if err != nil {
		t.Fatalf("connectClient StreamEvents: %v", err)
	}
	defer streamFraming.Close()

	// Send the unit request body for StreamEvents.
	if err := streamFraming.WriteMsg(defaultReadTimeout, struct{}{}); err != nil {
		t.Fatalf("write StreamEvents request: %v", err)
	}

	// Give the stream-events handler goroutine time to call hub.Subscribe()
	// before we ping, so it doesn't miss the Started event.
	time.Sleep(100 * time.Millisecond)

	// Ping the daemon.
	projectFile := NewShellNixProjectFile(mustAbsPath(shellNix))
	if err := opPing(paths, projectFile); err != nil {
		t.Fatalf("opPing: %v", err)
	}

	// Collect events until we see both Started and Completed.
	gotStarted, gotCompleted := false, false
	timeout := time.NewTimer(120 * time.Second)
	defer timeout.Stop()

	type rawEvent map[string]any
	type readResult struct {
		ev  rawEvent
		err error
	}

	for !gotStarted || !gotCompleted {
		ch := make(chan readResult, 1)
		go func() {
			var e rawEvent
			err := streamFraming.ReadMsg(0, &e)
			ch <- readResult{e, err}
		}()

		select {
		case r := <-ch:
			if r.err != nil {
				t.Fatalf("read event: %v", r.err)
			}
			if _, ok := r.ev["Started"]; ok {
				t.Logf("got Started")
				gotStarted = true
			}
			if _, ok := r.ev["Completed"]; ok {
				t.Logf("got Completed")
				gotCompleted = true
			}
			if _, ok := r.ev["Failure"]; ok {
				t.Fatalf("got Failure: %v", r.ev)
			}
		case <-timeout.C:
			t.Fatalf("timed out waiting for events (started=%v completed=%v)", gotStarted, gotCompleted)
		}
	}

	// Verify snapshot contains the completed event.
	snapFraming, err := connectClient(socketPath, CommStreamSnapshot, defaultReadTimeout)
	if err != nil {
		t.Fatalf("connectClient StreamSnapshot: %v", err)
	}
	defer snapFraming.Close()

	// Decode snapshot as raw JSON.
	var rawSnap struct {
		Snapshot []map[string]any `json:"snapshot"`
	}
	if err := snapFraming.ReadMsg(defaultReadTimeout, &rawSnap); err != nil {
		t.Fatalf("read snapshot: %v", err)
	}
	if len(rawSnap.Snapshot) == 0 {
		t.Fatal("expected non-empty snapshot")
	}
	t.Logf("snapshot has %d event(s)", len(rawSnap.Snapshot))
}

// TestLorriDB exercises the LorriDB wrapper in isolation.
func TestLorriDB(t *testing.T) {
	dir := t.TempDir()
	dbPath := filepath.Join(dir, "test.sqlite")

	db, err := OpenLorriDB(dbPath)
	if err != nil {
		t.Fatalf("OpenLorriDB: %v", err)
	}
	defer db.Close()

	// Insert two projects.
	if err := db.UpsertProject("/home/user/proj/shell.nix", false, ""); err != nil {
		t.Fatalf("UpsertProject: %v", err)
	}
	if err := db.UpsertProject("/home/user/flake/flake.nix", true, ".#devShells.default"); err != nil {
		t.Fatalf("UpsertProject flake: %v", err)
	}

	// List and verify.
	projects, err := db.ListProjects()
	if err != nil {
		t.Fatalf("ListProjects: %v", err)
	}
	if len(projects) != 2 {
		t.Fatalf("expected 2 projects, got %d", len(projects))
	}

	// Upsert again — should be idempotent.
	if err := db.UpsertProject("/home/user/proj/shell.nix", false, ""); err != nil {
		t.Fatalf("UpsertProject duplicate: %v", err)
	}
	projects, _ = db.ListProjects()
	if len(projects) != 2 {
		t.Fatalf("expected 2 projects after duplicate upsert, got %d", len(projects))
	}

	// Delete one.
	if err := db.DeleteProject("/home/user/proj/shell.nix"); err != nil {
		t.Fatalf("DeleteProject: %v", err)
	}
	projects, _ = db.ListProjects()
	if len(projects) != 1 {
		t.Fatalf("expected 1 project after delete, got %d", len(projects))
	}

	// AutoGC — the flake path doesn't exist, so it should be deleted.
	if err := db.AutoGCRemovedProjects(); err != nil {
		t.Fatalf("AutoGCRemovedProjects: %v", err)
	}
	projects, _ = db.ListProjects()
	if len(projects) != 0 {
		t.Fatalf("expected 0 projects after AutoGC, got %d: %v", len(projects), projects)
	}
}
