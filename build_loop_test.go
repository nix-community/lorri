package main

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

// TestBuildLoopPingStartsAndCompletes is an integration test that:
//  1. Creates a minimal shell.nix in a temp directory
//  2. Creates a BuildLoop for it
//  3. Wires up an EventHub
//  4. Sends a ping
//  5. Asserts that Started and Completed events arrive within a timeout
//
// Requires: nix-instantiate, nix-build, nix-store on $PATH and RUN_TIME_CLOSURE set.
func TestBuildLoopPingStartsAndCompletes(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	// Create a minimal shell.nix that builds quickly.
	dir := t.TempDir()
	shellNix := filepath.Join(dir, "shell.nix")
	if err := os.WriteFile(shellNix, []byte(`
derivation {
  name = "lorri-test-shell";
  builder = "/bin/sh";
  args = [ "-c" "echo > $out" ];
  system = builtins.currentSystem;
  allowSubstitutes = false;
  preferLocalBuild = true;
}
`), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}

	// Set up logged-eval file and GC root dir.
	gcRootDir := mustAbsPath(filepath.Join(dir, "gc_roots"))
	if err := os.MkdirAll(string(gcRootDir), 0o755); err != nil {
		t.Fatalf("mkdir gc_roots: %v", err)
	}
	loggedEvalFile := writeLoggedEvalForTest(t, dir)

	cfg := BuildLoopConfig{
		ProjectFile:    NewShellNixProjectFile(mustAbsPath(shellNix)),
		NixFile:        shellNix,
		LoggedEvalFile: loggedEvalFile,
		Opts:           NixOptions{},
		RunTimeClosure: rtc,
		LorriBin:       lorriBinForTest(t),
		GCRootDir:      gcRootDir,
	}

	bl, err := NewBuildLoop(cfg)
	if err != nil {
		t.Fatalf("NewBuildLoop: %v", err)
	}

	// Wire up the EventHub.
	hub := NewEventHub()
	eventCh := make(chan LoopHandlerEvent, 64)
	go hub.Run(eventCh)

	// Subscribe before starting the loop so we don't miss any events.
	sub, subID := hub.Subscribe()
	defer hub.Unsubscribe(subID)

	rxPing := make(chan struct{}, 1)
	t.Cleanup(func() { close(rxPing); bl.watch.Close() })
	go bl.Forever(eventCh, rxPing)

	// Send a ping.
	rxPing <- struct{}{}

	// Wait for Started then Completed (or Failure) within a generous timeout.
	timeout := 120 * time.Second
	deadline := time.NewTimer(timeout)
	defer deadline.Stop()

	var gotStarted, gotCompleted bool
	for !gotStarted || !gotCompleted {
		select {
		case ev := <-sub:
			switch {
			case ev.Started != nil:
				t.Logf("got Started: nix_file=%s reason=%s",
					ev.Started.NixFile, ev.Started.Reason.raw)
				gotStarted = true
			case ev.Completed != nil:
				t.Logf("got Completed: nix_file=%s shell_gc_root=%s",
					ev.Completed.NixFile,
					ev.Completed.RootedOutputPaths.ShellGCRoot)
				gotCompleted = true
			case ev.Failure != nil:
				t.Fatalf("got Failure: nix_file=%s message=%s",
					ev.Failure.NixFile, ev.Failure.Message())
			}
		case <-deadline.C:
			t.Fatalf("timed out after %v waiting for events (started=%v completed=%v)",
				timeout, gotStarted, gotCompleted)
		}
	}
}
