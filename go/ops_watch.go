package main

// op_watch: run a BuildLoop for a project, watching for input file changes.
// Mirrors src/ops.rs op_watch() / main_run_forever() / main_run_once().

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
)

// opWatch runs a BuildLoop for the given project.
// If once is true, runs exactly one build and exits.
// Otherwise, runs forever, printing events to stderr.
func opWatch(paths *Paths, projectFile ProjectFile, rtc string, once bool) error {
	nixFile := nixFilePathForProject(projectFile)
	cas, err := NewCAS(paths.CASDir)
	if err != nil {
		return fmt.Errorf("watch: init CAS: %w", err)
	}

	cfg := BuildLoopConfig{
		ProjectFile:    projectFile,
		NixFile:        nixFile,
		CAS:            cas,
		Opts:           EmptyNixOptions(),
		RunTimeClosure: rtc,
		GCRootDir:      paths.GCRootDir,
	}

	bl, err := NewBuildLoop(cfg)
	if err != nil {
		return fmt.Errorf("watch: create build loop: %w", err)
	}

	if once {
		return watchOnce(bl)
	}
	return watchForever(bl)
}

// watchOnce runs a single build and returns. Mirrors main_run_once().
func watchOnce(bl *BuildLoop) error {
	out, err := bl.Once()
	if err != nil {
		be, ok := err.(*BuildError)
		if ok && be.Kind != BuildErrorKindIo {
			return fmt.Errorf("build failed: %w", err)
		}
		return err
	}
	fmt.Fprintf(os.Stderr, "lorri: build succeeded. GC root: %s\n", out.ShellGCRoot)
	return nil
}

// watchForever runs the build loop indefinitely, printing events to stderr.
// Mirrors main_run_forever().
func watchForever(bl *BuildLoop) error {
	hub := NewEventHub()
	hubCh := make(chan LoopHandlerEvent, 64)
	go hub.Run(hubCh)

	sub, id := hub.Subscribe()
	defer hub.Unsubscribe(id)

	rxPing := make(chan struct{}, 1)
	go bl.Forever(hubCh, rxPing)
	rxPing <- struct{}{} // initial build

	// Handle Ctrl-C gracefully.
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	defer signal.Stop(sig)

	for {
		select {
		case ev := <-sub:
			printWatchEvent(ev)
		case <-sig:
			return nil
		}
	}
}

// printWatchEvent prints a build event to stderr in human-readable form.
func printWatchEvent(ev Event) {
	switch {
	case ev.Started != nil:
		reason := "ping"
		if ev.Started.Reason.IsPingReceived() {
			reason = "ping"
		} else if files := ev.Started.Reason.FilesChanged(); len(files) > 0 {
			reason = fmt.Sprintf("files changed: %v", files)
		}
		fmt.Fprintf(os.Stderr, "lorri: evaluating %s (%s)\n", ev.Started.NixFile, reason)
	case ev.Completed != nil:
		fmt.Fprintf(os.Stderr, "lorri: build of %s succeeded. GC root: %s\n",
			ev.Completed.NixFile, ev.Completed.RootedOutputPaths.ShellGCRoot)
	case ev.Failure != nil:
		fmt.Fprintf(os.Stderr, "lorri: build of %s failed: %s\n",
			ev.Failure.NixFile, ev.Failure.Message())
	}
}
