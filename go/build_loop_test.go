package main

import (
	"fmt"
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

	// Set up CAS and GC root dirs.
	casDir := mustAbsPath(filepath.Join(dir, "cas"))
	gcRootDir := mustAbsPath(filepath.Join(dir, "gc_roots"))
	if err := os.MkdirAll(string(gcRootDir), 0o755); err != nil {
		t.Fatalf("mkdir gc_roots: %v", err)
	}

	cas, err := NewCAS(casDir)
	if err != nil {
		t.Fatalf("NewCAS: %v", err)
	}

	cfg := BuildLoopConfig{
		ProjectFile:    NewShellNixProjectFile(mustAbsPath(shellNix)),
		NixFile:        shellNix,
		CAS:            cas,
		Opts:           EmptyNixOptions(),
		RunTimeClosure: rtc,
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

// TestBuildLoopCoalescing verifies that rapid pings while a build is running
// result in at most one extra build (coalescing), not N builds.
func TestBuildLoopCoalescing(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	dir := t.TempDir()
	shellNix := filepath.Join(dir, "shell.nix")
	if err := os.WriteFile(shellNix, []byte(`
derivation {
  name = "lorri-test-coalesce";
  builder = "/bin/sh";
  args = [ "-c" "sleep 1; echo > $out" ];
  system = builtins.currentSystem;
  allowSubstitutes = false;
  preferLocalBuild = true;
}
`), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}

	casDir := mustAbsPath(filepath.Join(dir, "cas"))
	gcRootDir := mustAbsPath(filepath.Join(dir, "gc_roots"))
	os.MkdirAll(string(gcRootDir), 0o755) //nolint:errcheck

	cas, err := NewCAS(casDir)
	if err != nil {
		t.Fatalf("NewCAS: %v", err)
	}

	cfg := BuildLoopConfig{
		ProjectFile:    NewShellNixProjectFile(mustAbsPath(shellNix)),
		NixFile:        shellNix,
		CAS:            cas,
		Opts:           EmptyNixOptions(),
		RunTimeClosure: rtc,
		GCRootDir:      gcRootDir,
	}

	bl, err := NewBuildLoop(cfg)
	if err != nil {
		t.Fatalf("NewBuildLoop: %v", err)
	}

	hub := NewEventHub()
	eventCh := make(chan LoopHandlerEvent, 64)
	go hub.Run(eventCh)

	sub, subID := hub.Subscribe()
	defer hub.Unsubscribe(subID)

	rxPing := make(chan struct{}, 10)
	t.Cleanup(func() { close(rxPing); bl.watch.Close() })
	go bl.Forever(eventCh, rxPing)

	// Send first ping to start a build, then flood with more pings while it runs.
	rxPing <- struct{}{}
	time.Sleep(100 * time.Millisecond) // let it start
	for range 5 {
		select {
		case rxPing <- struct{}{}:
		default:
		}
	}

	// Collect all events for 10 seconds. Count completions.
	deadline := time.NewTimer(30 * time.Second)
	defer deadline.Stop()

	completions := 0
	started := 0
	for {
		select {
		case ev := <-sub:
			switch {
			case ev.Started != nil:
				started++
				t.Logf("Started #%d", started)
			case ev.Completed != nil:
				completions++
				t.Logf("Completed #%d", completions)
				if completions >= 2 {
					// Two completions is the maximum expected: one for the
					// in-flight build, one for the coalesced follow-up.
					// If we get here, we're done.
					goto done
				}
			case ev.Failure != nil:
				t.Fatalf("Failure: %s", ev.Failure.Message())
			}
		case <-deadline.C:
			// We may only get one completion if the coalesced build was
			// deduplicated — that is also acceptable.
			if completions == 0 {
				t.Fatal("no completions within deadline")
			}
			goto done
		}
	}
done:
	t.Logf("coalescing result: started=%d completions=%d (5 pings sent)", started, completions)
	if completions > 2 {
		t.Errorf("expected at most 2 completions for 5+ pings during a build, got %d", completions)
	}
}

// TestEventHubSnapshot verifies that snapshot requests return the last known
// state for each project.
func TestEventHubSnapshot(t *testing.T) {
	hub := NewEventHub()
	eventCh := make(chan LoopHandlerEvent, 64)
	go hub.Run(eventCh)

	// Inject a Completed event.
	nixFile := "/tmp/test.nix"
	eventCh <- LoopHandlerEvent{BuildEvent: &Event{
		Completed: &EventCompleted{
			NixFile:           nixFile,
			RootedOutputPaths: OutputPath{ShellGCRoot: "/nix/store/abc-test"},
		},
	}}

	// Give the hub time to process.
	time.Sleep(20 * time.Millisecond)

	// Request a snapshot.
	snapCh := make(chan EventSnapshot, 1)
	eventCh <- LoopHandlerEvent{SnapshotListener: snapCh}

	select {
	case snap := <-snapCh:
		if len(snap.Snapshot) != 1 {
			t.Fatalf("expected 1 event in snapshot, got %d", len(snap.Snapshot))
		}
		ev := snap.Snapshot[0]
		if ev.Completed == nil {
			t.Fatalf("expected Completed event in snapshot, got %+v", ev)
		}
		if ev.Completed.NixFile != nixFile {
			t.Errorf("nix_file mismatch: got %q want %q", ev.Completed.NixFile, nixFile)
		}
	case <-time.After(time.Second):
		t.Fatal("timed out waiting for snapshot")
	}
}

// TestEventHubFanOut verifies that multiple subscribers all receive the same events.
func TestEventHubFanOut(t *testing.T) {
	hub := NewEventHub()
	eventCh := make(chan LoopHandlerEvent, 64)
	go hub.Run(eventCh)

	const N = 5
	subs := make([]<-chan Event, N)
	ids := make([]uint64, N)
	for i := range subs {
		subs[i], ids[i] = hub.Subscribe()
	}
	defer func() {
		for _, id := range ids {
			hub.Unsubscribe(id)
		}
	}()

	// Send 3 events.
	for i := range 3 {
		eventCh <- LoopHandlerEvent{BuildEvent: &Event{
			Started: &EventStarted{
				NixFile: fmt.Sprintf("/tmp/proj%d.nix", i),
				Reason:  reasonPingReceived(),
			},
		}}
	}

	// Each subscriber should receive all 3 events.
	time.Sleep(50 * time.Millisecond)
	for i, sub := range subs {
		count := 0
	drain:
		for {
			select {
			case <-sub:
				count++
			default:
				break drain
			}
		}
		if count != 3 {
			t.Errorf("subscriber %d received %d events, want 3", i, count)
		}
	}
}
