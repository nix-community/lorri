package main

// BuildLoop: per-project build goroutine.
// Mirrors src/build_loop.rs BuildLoop + src/daemon.rs Daemon::build_loop().
//
// EventHub uses sync.Map for subscriber fan-out.

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"sync/atomic"
)

// ---------------------------------------------------------------------------
// BuildOutputPath — runtime GC-rooted output path
// ---------------------------------------------------------------------------

// BuildOutputPath is the realized, GC-rooted output of a build.
// Mirrors builder.rs OutputPath (runtime version, distinct from the wire type).
type BuildOutputPath struct {
	// ShellGCRoot is the absolute path to the shell_gc_root symlink in
	// ~/.cache/lorri/gc_roots/<hash>/gc_root/shell_gc_root.
	ShellGCRoot string
}

// Exists reports whether the GC root symlink target exists.
func (o BuildOutputPath) Exists() bool {
	_, err := os.Lstat(o.ShellGCRoot)
	return err == nil
}

// ToWireOutputPath converts to the wire-protocol OutputPath for IPC serialization.
func (o BuildOutputPath) ToWireOutputPath() OutputPath {
	return OutputPath{ShellGCRoot: o.ShellGCRoot}
}

// ---------------------------------------------------------------------------
// createGCRoot — nix-store --realise --add-root
// ---------------------------------------------------------------------------

// createGCRoot creates an indirect GC root at symlinkPath pointing to storePath.
// Mirrors project.rs Project::create_indirect_root.
//
// Uses nix-store --realise --add-root to keep the store path alive across
// nix-collect-garbage runs.
func createGCRoot(storePath, symlinkPath string) error {
	if err := os.MkdirAll(filepath.Dir(symlinkPath), 0o755); err != nil {
		return fmt.Errorf("createGCRoot: mkdir: %w", err)
	}
	cmd := exec.Command("nix-store",
		"--realise",
		"--add-root", symlinkPath,
		storePath,
	)
	if out, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("createGCRoot: nix-store --add-root: %w\n%s", err, out)
	}
	return nil
}

// ---------------------------------------------------------------------------
// LoopHandlerEvent — internal union type (mirrors daemon.rs LoopHandlerEvent)
// ---------------------------------------------------------------------------

// LoopHandlerEvent is sent from BuildLoops to the EventHub.
// Exactly one field is non-nil.
type LoopHandlerEvent struct {
	// A build event (Started / Completed / Failure).
	BuildEvent *Event
	// A new StreamEvents subscriber channel.
	StreamListener chan<- Event
	// A one-shot snapshot request; the sender will be sent the current snapshot.
	SnapshotListener chan<- EventSnapshot
}

// ---------------------------------------------------------------------------
// EventHub — fan-out router (sync.Map of subscribers)
// ---------------------------------------------------------------------------

// EventHub routes LoopHandlerEvents to registered subscribers.
// Mirrors daemon.rs Daemon::build_loop().
//
// Run() must be called in its own goroutine.
type EventHub struct {
	// projectStates holds the last known Event for each nix file.
	// Used to serve snapshot requests.
	projectStates map[string]Event // key: nix file path
	stateMu       sync.RWMutex

	// subscribers: sync.Map[uint64 → chan<- Event]
	subscribers sync.Map
	nextID      atomic.Uint64
}

// NewEventHub creates an EventHub ready to Run.
func NewEventHub() *EventHub {
	return &EventHub{
		projectStates: make(map[string]Event),
	}
}

// Subscribe registers a new event subscriber and returns its channel and ID.
// The caller must call Unsubscribe(id) when done to avoid leaking the channel.
// The channel is buffered (64 events) so slow consumers don't block the hub.
func (h *EventHub) Subscribe() (<-chan Event, uint64) {
	ch := make(chan Event, 64)
	id := h.nextID.Add(1)
	h.subscribers.Store(id, (chan<- Event)(ch))
	return ch, id
}

// Unsubscribe removes a subscriber registered with Subscribe.
func (h *EventHub) Unsubscribe(id uint64) {
	h.subscribers.Delete(id)
}

// Run drains ch until it is closed, routing events to subscribers and
// answering snapshot requests.  Call in a dedicated goroutine.
func (h *EventHub) Run(ch <-chan LoopHandlerEvent) {
	for msg := range ch {
		switch {
		case msg.BuildEvent != nil:
			ev := *msg.BuildEvent
			nixFile := nixFileFromEvent(ev)

			// Update project state.
			h.stateMu.Lock()
			h.projectStates[nixFile] = ev
			h.stateMu.Unlock()

			// Broadcast to all live subscribers; remove any whose channels
			// are full/closed (non-blocking send, drop slow consumers).
			h.subscribers.Range(func(key, val any) bool {
				id := key.(uint64)
				ch := val.(chan<- Event)
				select {
				case ch <- ev:
				default:
					// Subscriber is full; drop it.
					h.subscribers.Delete(id)
				}
				return true
			})

		case msg.StreamListener != nil:
			h.subscribers.Store(h.nextID.Add(1), msg.StreamListener)

		case msg.SnapshotListener != nil:
			h.stateMu.RLock()
			snap := make([]Event, 0, len(h.projectStates))
			for _, ev := range h.projectStates {
				snap = append(snap, ev)
			}
			h.stateMu.RUnlock()
			// Non-blocking: if the receiver is gone, just drop.
			select {
			case msg.SnapshotListener <- EventSnapshot{Snapshot: snap}:
			default:
			}
		}
	}
}

// nixFileFromEvent extracts the nix file path from any Event variant.
func nixFileFromEvent(ev Event) string {
	switch {
	case ev.Started != nil:
		return ev.Started.NixFile
	case ev.Completed != nil:
		return ev.Completed.NixFile
	case ev.Failure != nil:
		return ev.Failure.NixFile
	}
	return ""
}

// ---------------------------------------------------------------------------
// BuildLoopConfig — immutable per-project config
// ---------------------------------------------------------------------------

// BuildLoopConfig holds all immutable configuration for a BuildLoop.
type BuildLoopConfig struct {
	// ProjectFile is the build source descriptor.
	ProjectFile ProjectFile
	// NixFile is the absolute path string used as the primary watch target
	// and as the key in EventHub.projectStates.
	// For ShellNix: the shell.nix path.
	// For FlakeNix: context/flake.nix.
	NixFile string
	// LoggedEvalFile is the path to logged-evaluation.nix on disk.
	LoggedEvalFile AbsPath
	// Opts are extra Nix CLI options.
	Opts NixOptions
	// RunTimeClosure is the Nix store path for the lorri runtime closure.
	RunTimeClosure string
	// GCRootDir is the directory under which per-project GC root dirs live.
	// (~/.cache/lorri/gc_roots/)
	GCRootDir AbsPath
}

// gcRootSymlinkPath returns the shell_gc_root symlink path for this config.
// Mirrors project.rs Project::shell_gc_root().
func (c BuildLoopConfig) gcRootSymlinkPath() string {
	return gcRootPathForProject(c.GCRootDir, c.ProjectFile).String()
}

// ---------------------------------------------------------------------------
// BuildLoop — per-project build goroutine
// ---------------------------------------------------------------------------

// BuildLoop watches a project's Nix file and rebuilds it on changes.
// Mirrors build_loop.rs BuildLoop.
type BuildLoop struct {
	cfg   BuildLoopConfig
	watch *Watch
}

// NewBuildLoop creates a BuildLoop and registers the initial watch on the nix file.
// Mirrors BuildLoop::new().
func NewBuildLoop(cfg BuildLoopConfig) (*BuildLoop, error) {
	w, err := NewWatch()
	if err != nil {
		return nil, fmt.Errorf("NewBuildLoop: watch: %w", err)
	}
	// Start watching only the nix file itself; new paths are added after each build.
	if err := w.AddPaths([]WatchPathBuf{{Recursive: false, Path: cfg.NixFile}}); err != nil {
		w.Close()
		return nil, fmt.Errorf("NewBuildLoop: add initial watch: %w", err)
	}
	return &BuildLoop{cfg: cfg, watch: w}, nil
}

// buildResult is the output of a single build goroutine.
type buildResult struct {
	result *RunResult
	err    error
}

// Forever runs the build loop indefinitely.
// Mirrors BuildLoop::forever().
//
// txEvents: send build events to the EventHub.
// rxPing:   receive ping signals (struct{}); channel close causes return.
func (bl *BuildLoop) Forever(txEvents chan<- LoopHandlerEvent, rxPing <-chan struct{}) {
	// buildResultCh is nil (never fires in select) when no build is running,
	// and set to a real channel when a build goroutine is active.
	// A nil channel blocks forever in select, so it acts as the "not building"
	// sentinel — no need for a separate isBuilding bool.
	// Mirrors BuildLoop::forever() in src/build_loop.rs.
	var buildResultCh <-chan buildResult
	scheduled := false

	sendEvent := func(ev Event) {
		txEvents <- LoopHandlerEvent{BuildEvent: &ev}
	}

	startBuild := func() {
		ch := make(chan buildResult, 1)
		buildResultCh = ch
		go runBuild(bl.cfg, ch)
	}

	for {
		select {
		case res, ok := <-buildResultCh:
			if !ok {
				// Should not happen since we use buffered channels, but be safe.
				buildResultCh = nil
				continue
			}
			buildResultCh = nil

			// If another build was scheduled while this one ran, start it now.
			if scheduled {
				scheduled = false
				startBuild()
			}

			// Process the build result.
			outPath, err := bl.handleRunResult(res)
			if err != nil {
				var be *BuildError
				if errors.As(err, &be) && be.Kind == BuildErrorKindIo {
					// Unrecoverable I/O error — panic like Rust does.
					panic(fmt.Sprintf("unrecoverable build error: %v", err))
				}
				log.Printf("build failed: %s: %v", bl.cfg.NixFile, err)
				sendEvent(Event{
					Failure: &EventFailure{
						NixFile:    bl.cfg.NixFile,
						FailureRaw: marshalFailureRaw(err),
					},
				})
			} else {
				log.Printf("build completed: %s → %s", bl.cfg.NixFile, outPath.ShellGCRoot)
				sendEvent(Event{
					Completed: &EventCompleted{
						NixFile: bl.cfg.NixFile,
						RootedOutputPaths: OutputPath{
							ShellGCRoot: outPath.ShellGCRoot,
						},
					},
				})
			}

		case changed, ok := <-bl.watch.Events:
			if !ok {
				return
			}
			log.Printf("files changed: %s (%d paths)", bl.cfg.NixFile, len(changed))
			sendEvent(Event{
				Started: &EventStarted{
					NixFile: bl.cfg.NixFile,
					Reason:  reasonFilesChanged(changed),
				},
			})
			if buildResultCh != nil {
				scheduled = true
			} else {
				startBuild()
			}

		case _, ok := <-rxPing:
			if !ok {
				return
			}
			log.Printf("ping received: %s", bl.cfg.NixFile)
			sendEvent(Event{
				Started: &EventStarted{
					NixFile: bl.cfg.NixFile,
					Reason:  reasonPingReceived(),
				},
			})
			if buildResultCh != nil {
				scheduled = true
			} else {
				startBuild()
			}
		}
	}
}

// handleRunResult processes a build result: updates the watch set and creates
// a GC root.  Mirrors BuildLoop::handle_run_result().
func (bl *BuildLoop) handleRunResult(res buildResult) (BuildOutputPath, error) {
	if res.err != nil {
		return BuildOutputPath{}, res.err
	}

	// Update the watch set with paths discovered during this build.
	reduced := ReducePaths(res.result.ReferencedPaths)
	if err := bl.watch.AddPaths(reduced); err != nil {
		return BuildOutputPath{}, &BuildError{Kind: BuildErrorKindIo, Msg: err.Error()}
	}

	// Create the GC root symlink so nix-collect-garbage won't remove the output.
	symlinkPath := bl.cfg.gcRootSymlinkPath()
	storePath := res.result.Result.Path
	if err := createGCRoot(storePath, symlinkPath); err != nil {
		// Non-fatal: the build succeeded, we just might lose the output to GC.
		// Log and continue (mirrors Rust's soft error handling here).
		log.Printf("createGCRoot: non-fatal, build output may be GC'd: %v", err)
	}

	// Transfer ownership: the RootedPath temp dir can now be released because
	// nix-store --add-root has taken over GC responsibility.
	res.result.Result.Release()

	return BuildOutputPath{ShellGCRoot: symlinkPath}, nil
}

// ---------------------------------------------------------------------------
// runBuild — called in a goroutine, sends one buildResult to ch
// ---------------------------------------------------------------------------

func runBuild(cfg BuildLoopConfig, ch chan<- buildResult) {
	var res buildResult

	if cfg.ProjectFile.ShellNix != nil {
		r, err := InstantiateAndBuild(cfg.NixFile, cfg.LoggedEvalFile, cfg.Opts, cfg.RunTimeClosure)
		res = buildResult{result: r, err: err}
	} else if cfg.ProjectFile.FlakeNix != nil {
		r, err := BuildFlake(*cfg.ProjectFile.FlakeNix)
		res = buildResult{result: r, err: err}
	} else {
		res = buildResult{err: buildErrorOutput("ProjectFile has no ShellNix or FlakeNix set")}
	}

	ch <- res
}

// ---------------------------------------------------------------------------
// Reason helpers — build the Reason wire type
// ---------------------------------------------------------------------------

func reasonPingReceived() Reason {
	return Reason{raw: []byte(`"PingReceived"`)}
}

func reasonFilesChanged(paths []string) Reason {
	// Encode as {"FilesChanged": ["path1", ...]}
	type w struct {
		FilesChanged []string `json:"FilesChanged"`
	}
	b, _ := json.Marshal(w{FilesChanged: paths})
	return Reason{raw: b}
}

// marshalFailureRaw encodes a build error as a raw JSON message for EventFailure.
func marshalFailureRaw(err error) []byte {
	type failureMsg struct {
		Message string `json:"message"`
	}
	b, _ := json.Marshal(failureMsg{Message: err.Error()})
	return b
}
