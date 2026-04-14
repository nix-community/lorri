package main

// lorri — Go rewrite entry point and all ops implementations.

import (
	"context"
	_ "embed"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"

	"zombiezen.com/go/sqlite"
	"zombiezen.com/go/sqlite/sqlitex"
)

// runtimeClosure is baked in at link time by go/default.nix via:
//
//	x_defs."main.runtimeClosure" = "${rtc}"
//
// This mirrors how build.rs bakes RUN_TIME_CLOSURE into the Rust binary.
// In development (lorri nix-shell), runtimeClosure is "" and the env var
// RUN_TIME_CLOSURE is used as a fallback instead.
var runtimeClosure = ""

// requireRTC returns the lorri runtime closure store path.
// Prefers the link-time constant; falls back to $RUN_TIME_CLOSURE.
func requireRTC() string {
	if runtimeClosure != "" {
		return runtimeClosure
	}
	return os.Getenv("RUN_TIME_CLOSURE")
}

func main() {
	// Ignore SIGPIPE so that writes to a closed socket/pipe return an error
	// rather than killing the daemon process (e.g. when a direnv client
	// disconnects mid-stream). Mirrors src/main.rs setup_sigpipe().
	signal.Ignore(syscall.SIGPIPE)

	os.Exit(withPanicHandler(func() int {
		if err := run(os.Args[1:]); err != nil {
			// BuildError → convert to the appropriate ExitError.
			var be *BuildError
			if errors.As(err, &be) {
				err = buildErrorToExitError(be)
			}
			var exitErr *ExitError
			if errors.As(err, &exitErr) {
				fmt.Fprintf(os.Stderr, "lorri: %v\n", exitErr)
				return exitErr.Code
			}
			fmt.Fprintf(os.Stderr, "lorri: %v\n", err)
			return 1
		}
		return 0
	}))
}

func run(args []string) error {
	if len(args) == 0 {
		printUsage()
		return exitUserError("no subcommand given", nil)
	}

	switch args[0] {
	case "daemon":
		return runDaemon(args[1:])
	case "direnv":
		return runDirenv(args[1:])
	case "gc":
		return runGC(args[1:])
	case "info":
		return runInfo(args[1:])
	case "init":
		if err := opInit(); err != nil {
			return exitTemporary("lorri init failed", err)
		}
		return nil
	case "prompt":
		return runPrompt(args[1:])
	case "internal":
		return runInternal(args[1:])
	case "-h", "--help", "help":
		printUsage()
		return nil
	default:
		printUsage()
		return exitUserError(fmt.Sprintf("unknown subcommand %q", args[0]), nil)
	}
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

// buildErrorToExitError maps a BuildError to the appropriate ExitError.
// Mirrors the exit code semantics from src/ops/error.rs:
//
//	Spawn (executable not found) → ExitCodeMissing (127)
//	Io (disk/pipe)               → ExitCodeTemporary (111)
//	Exit / Output (build failed) → ExitCodeExpected (1)
func buildErrorToExitError(be *BuildError) *ExitError {
	switch be.Kind {
	case BuildErrorKindSpawn:
		return exitMissing(be.Error(), nil)
	case BuildErrorKindIo:
		return exitTemporary(be.Error(), nil)
	default: // Exit, Output
		return exitExpected(be.Error(), nil)
	}
}

// mustInitPaths calls InitPaths and wraps any error as a user error (exit 100).
// Failure to set up paths is a permanent configuration problem.
func mustInitPaths() (*Paths, error) {
	paths, err := InitPaths()
	if err != nil {
		return nil, exitUserError("could not initialise lorri state directories", err)
	}
	return paths, nil
}

// mustRTC returns the runtime closure path or an environment error (exit 126).
func mustRTC() (string, error) {
	rtc := requireRTC()
	if rtc == "" {
		return "", exitEnvironment(
			"RUN_TIME_CLOSURE not set; please run lorri from its nix-shell", nil)
	}
	return rtc, nil
}

// mustResolveProjectFile wraps resolveProjectFile as a user error (exit 100).
func mustResolveProjectFile(shellFile, contextDir, flake string) (ProjectFile, error) {
	pf, err := resolveProjectFile(shellFile, contextDir, flake)
	if err != nil {
		return ProjectFile{}, exitUserError("could not resolve project file", err)
	}
	return pf, nil
}

// ---------------------------------------------------------------------------
// daemon
// ---------------------------------------------------------------------------

func runDaemon(args []string) error {
	fs := flag.NewFlagSet("lorri daemon", flag.ContinueOnError)
	extraNixOptsJSON := fs.String("extra-nix-options", "",
		`JSON object with optional "builders" and "substituters" string arrays`)
	if err := fs.Parse(args); err != nil {
		return err
	}

	opts := NixOptions{}
	if *extraNixOptsJSON != "" {
		var parsed struct {
			Builders     []string `json:"builders"`
			Substituters []string `json:"substituters"`
		}
		if err := json.Unmarshal([]byte(*extraNixOptsJSON), &parsed); err != nil {
			return exitUserError("--extra-nix-options: invalid JSON", err)
		}
		opts.Builders = parsed.Builders
		opts.Substituters = parsed.Substituters
	}

	rtc, err := mustRTC()
	if err != nil {
		return err
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	return NewDaemon(opts).ServeContext(context.Background(), paths, rtc)
}

// ---------------------------------------------------------------------------
// direnv
// ---------------------------------------------------------------------------

func runDirenv(args []string) error {
	fs := flag.NewFlagSet("lorri direnv", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor (e.g. .#)")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	projectFile, err := mustResolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opDirenv(os.Stdout, paths, projectFile)
}

// ---------------------------------------------------------------------------
// gc
// ---------------------------------------------------------------------------

func runGC(args []string) error {
	// --json applies to both info and rm, matching Rust's GcOptions layout.
	fs := flag.NewFlagSet("lorri gc", flag.ContinueOnError)
	jsonOut := fs.Bool("json", false, "machine-readable JSON output")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	remaining := fs.Args()
	if len(remaining) == 0 {
		fmt.Fprint(os.Stderr, `usage: lorri gc [--json] <info|rm> [options]

subcommands:
  info  print the gc roots lorri created, and whether their project still exists
  rm    remove gc roots for projects whose nix file no longer exists

options:
  --json  machine-readable JSON output
`)
		return exitUserError("no gc subcommand given", nil)
	}
	switch remaining[0] {
	case "info":
		return runGCInfo(remaining[1:], *jsonOut)
	case "rm":
		return runGCRm(remaining[1:], *jsonOut)
	default:
		return exitUserError(fmt.Sprintf("unknown gc subcommand %q", remaining[0]), nil)
	}
}

func runGCInfo(args []string, jsonOut bool) error {
	fs := flag.NewFlagSet("lorri gc info", flag.ContinueOnError)
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	return opGCInfo(paths, jsonOut)
}

func runGCRm(args []string, jsonOut bool) error {
	fs := flag.NewFlagSet("lorri gc rm", flag.ContinueOnError)
	allFlag := fs.Bool("all", false, "delete roots of all projects")
	olderThan := fs.String("older-than", "", "delete roots older than this duration (e.g. 30d, 2m, 1y)")
	dryRun := fs.Bool("dry-run", false, "only print what would be deleted")
	// --shell-file can be given multiple times, matching Rust's Vec<PathBuf>.
	var shellFiles []string
	fs.Func("shell-file", "also delete root for this shell file (repeatable)", func(v string) error {
		shellFiles = append(shellFiles, v)
		return nil
	})
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}

	opts := GCRmOptions{
		ShellFiles: shellFiles,
		All:        *allFlag,
		DryRun:     *dryRun,
		JSON:       jsonOut,
	}
	if *olderThan != "" {
		d, err := parseDuration(*olderThan)
		if err != nil {
			return exitUserError("--older-than: invalid duration", err)
		}
		opts.OlderThan = &d
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	return opGCRm(paths, opts)
}

// ---------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------

func runInfo(args []string) error {
	fs := flag.NewFlagSet("lorri info", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (required if no flake)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	// info uses SourceOptions (no default — must be explicit).
	if *shellFile == "" && *flake == "" {
		return exitUserError("lorri info requires --shell-file or --flake", nil)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	projectFile, err := mustResolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opInfo(paths, projectFile)
}

// ---------------------------------------------------------------------------
// prompt
// ---------------------------------------------------------------------------

func runPrompt(args []string) error {
	if len(args) == 0 {
		fmt.Fprint(os.Stderr, `usage: lorri prompt <default> [options]

subcommands:
  default  print 'ℓ' if the current directory is inside a lorri-watched project
`)
		return exitUserError("no prompt subcommand given", nil)
	}
	switch args[0] {
	case "default":
		return runPromptDefault(args[1:])
	default:
		return exitUserError(fmt.Sprintf("unknown prompt subcommand %q", args[0]), nil)
	}
}

func runPromptDefault(args []string) error {
	fs := flag.NewFlagSet("lorri prompt default", flag.ContinueOnError)
	leadingSpace := fs.Bool("include-leading-space", false,
		"include a leading space before the prompt symbol")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	return opPrompt(*leadingSpace, paths)
}

// ---------------------------------------------------------------------------
// internal subcommands
// ---------------------------------------------------------------------------

func runInternal(args []string) error {
	if len(args) == 0 {
		fmt.Fprint(os.Stderr, `usage: lorri internal <subcommand>

These are plumbing commands. They are unstable and intended for scripts
and integrations, not direct use.

subcommands:
  ping_            tell the daemon to watch the current project;
                   starts watching if not already, keeps it alive if so
  stream-events_   stream build events from the daemon as JSON lines;
                   intended for scripts (no stability guarantee yet)
`)
		return exitUserError("no internal subcommand given", nil)
	}
	switch args[0] {
	case "ping_":
		return runPing(args[1:])
	case "stream-events_":
		return runStreamEvents(args[1:])
	default:
		return exitUserError(fmt.Sprintf("unknown internal subcommand %q", args[0]), nil)
	}
}

func runPing(args []string) error {
	fs := flag.NewFlagSet("lorri internal ping_", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor (e.g. .#)")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	projectFile, err := mustResolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opPing(paths, projectFile)
}

func runStreamEvents(args []string) error {
	fs := flag.NewFlagSet("lorri internal stream-events_", flag.ContinueOnError)
	kind := fs.String("kind", "all", "event kind: live, snapshot, or all")
	if err := fs.Parse(args); err != nil {
		return exitUserError("bad flags", err)
	}
	var ek EventKind
	switch *kind {
	case "live":
		ek = EventKindLive
	case "snapshot":
		ek = EventKindSnapshot
	case "all":
		ek = EventKindAll
	default:
		return exitUserError(fmt.Sprintf("--kind must be live, snapshot, or all (got %q)", *kind), nil)
	}
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}
	return opStreamEvents(paths, ek)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// resolveProjectFile builds a ProjectFile from the CLI flags.
func resolveProjectFile(shellFile, contextDir, flake string) (ProjectFile, error) {
	if shellFile != "" && flake != "" {
		return ProjectFile{}, fmt.Errorf("cannot use --shell-file and --flake together")
	}
	if shellFile != "" {
		abs, err := NewAbsPathFromCwd(shellFile)
		if err != nil {
			return ProjectFile{}, fmt.Errorf("--shell-file: %w", err)
		}
		return NewShellNixProjectFile(abs), nil
	}
	if flake != "" {
		ctx, err := NewAbsPathFromCwd(contextDir)
		if err != nil {
			return ProjectFile{}, fmt.Errorf("--context: %w", err)
		}
		return NewFlakeProjectFile(ctx, flake), nil
	}
	// Auto-detect: shell.nix → flake.nix → default.nix
	for _, candidate := range []string{"shell.nix", "flake.nix", "default.nix"} {
		if pf, ok := findNixFile(candidate); ok {
			return pf, nil
		}
	}
	return ProjectFile{}, fmt.Errorf(
		"no default build source found; create shell.nix or flake.nix, " +
			"or pass --shell-file / --flake",
	)
}

func findNixFile(name string) (ProjectFile, bool) {
	cwd, err := cwdFunc()
	if err != nil {
		return ProjectFile{}, false
	}
	full := filepath.Join(cwd, name)
	info, err := os.Stat(full)
	if err != nil || !info.Mode().IsRegular() {
		return ProjectFile{}, false
	}
	abs := mustAbsPath(full)
	if name == "flake.nix" {
		return NewFlakeProjectFile(abs.Dir(), ".#"), true
	}
	return NewShellNixProjectFile(abs), true
}

func printUsage() {
	fmt.Fprint(os.Stderr, `lorri — your project's nix-shell

usage: lorri <subcommand> [options]

subcommands:
  daemon    start the multi-project daemon
  direnv    emit shell script for direnv to eval via 'eval "$(lorri direnv)"'
  gc        remove lorri GC roots for projects whose nix file is gone
  info      show project and daemon status information
  init      write bootstrap files to the current directory to start a new project
  prompt    generate a lorri status marker for inclusion in your shell prompt
  internal  plumbing commands (unstable)
`)
}

// ---------------------------------------------------------------------------
// ping
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// direnv
// ---------------------------------------------------------------------------

// envrcBash is the bash helper sourced by direnv to load the Nix environment.
// Embedded at compile time from envrc.bash (identical to src/ops/direnv/envrc.bash).
//
//go:embed envrc.bash
var envrcBash string

// minDirenvVersion is the minimum direnv version lorri requires.
var minDirenvVersion = direnvVersion{2, 19, 2}

// direnvVersion is a parsed semantic version triple.
type direnvVersion struct {
	major, minor, patch int
}

// parseDirenvVersion parses "major.minor.patch", e.g. "2.19.2".
func parseDirenvVersion(s string) (direnvVersion, error) {
	var v direnvVersion
	if _, err := fmt.Sscanf(strings.TrimSpace(s), "%d.%d.%d", &v.major, &v.minor, &v.patch); err != nil {
		return direnvVersion{}, fmt.Errorf("expected major.minor.patch, got %q: %w", s, err)
	}
	return v, nil
}

func (v direnvVersion) String() string {
	return fmt.Sprintf("%d.%d.%d", v.major, v.minor, v.patch)
}

// lt returns true if v is strictly less than other.
func (v direnvVersion) lt(other direnvVersion) bool {
	if v.major != other.major {
		return v.major < other.major
	}
	if v.minor != other.minor {
		return v.minor < other.minor
	}
	return v.patch < other.patch
}

// checkDirenvVersion runs `direnv version` and fails if it is below the minimum.
func checkDirenvVersion() error {
	out, err := exec.Command("direnv", "version").Output()
	if err != nil {
		if _, notFound := err.(*exec.Error); notFound {
			return exitMissing("`direnv`: executable not found", nil)
		}
		return exitTemporary("could not run `direnv version`", err)
	}

	ver, err := parseDirenvVersion(string(out))
	if err != nil {
		return exitEnvironment("could not figure out the current `direnv` version (parse error)", err)
	}

	if ver.lt(minDirenvVersion) {
		return exitEnvironment(fmt.Sprintf(
			"`direnv` is version %s, but >= %s is required for lorri to function",
			ver, minDirenvVersion,
		), nil)
	}
	return nil
}

// opDirenv implements `lorri direnv`.
// Writes the direnv shell script to out; all status messages go to stderr.
// Accepting an explicit io.Writer for the script output (instead of writing
// directly to os.Stdout) makes the function testable without mutating the
// global os.Stdout.
func opDirenv(out io.Writer, paths *Paths, projectFile ProjectFile) error {
	if err := checkDirenvVersion(); err != nil {
		return err
	}

	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)
	cachedExists := fileExists(string(gcRootPath))

	// Ping the daemon — best-effort, failure is not fatal.
	// Uses OnlyIfNotYetWatching (unlike `internal ping_` which always rebuilds).
	pingErr := opPingWithRebuild(paths, projectFile, RebuildOnlyIfNotYetWatching)
	pingSent := pingErr == nil

	// Log status to stderr (stdout is reserved for the shell script).
	switch {
	case pingSent && !cachedExists:
		fmt.Fprintln(os.Stderr, "lorri: has not completed an evaluation for this project yet")
	case !pingSent && cachedExists:
		fmt.Fprintln(os.Stderr, "lorri: daemon is not running, loading a cached environment")
	case !pingSent && !cachedExists:
		fmt.Fprintln(os.Stderr, "lorri: daemon is not running and this project has not yet been evaluated, please run `lorri daemon`")
	}

	// Write the shell script to out.
	// Format mirrors the Rust writeln! with r#"..."# (note leading newline).
	fmt.Fprintf(out,
		"\nEVALUATION_ROOT=%q\n\nwatch_file %q\nwatch_file \"$EVALUATION_ROOT\"\n\n%s\n",
		string(gcRootPath),
		string(paths.DaemonSocketFile),
		envrcBash,
	)

	// Warn if not being called from within direnv's envrc evaluation.
	if os.Getenv("DIRENV_IN_ENVRC") != "1" {
		fmt.Fprintln(os.Stderr, "lorri: `lorri direnv` should be executed by direnv from within an `.envrc` file. Run `lorri init` to get started.")
	}

	return nil
}

// ---------------------------------------------------------------------------
// stream-events
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

//go:embed trivial-shell.nix
var trivialShellNix string

//go:embed default-envrc
var defaultEnvrc string

// opInit writes shell.nix and .envrc to the current directory,
// skipping each file if it already exists.
func opInit() error {
	if err := createIfMissing("./shell.nix", trivialShellNix,
		"Make sure shell.nix is of a form that works with nix-shell."); err != nil {
		return err
	}
	if err := createIfMissing("./.envrc", defaultEnvrc,
		`Please add 'eval "$(lorri direnv)"' to .envrc to set up lorri support.`); err != nil {
		return err
	}
	fmt.Fprintln(os.Stderr, "lorri: done")
	return nil
}

// createIfMissing writes contents to path only if the file does not already
// exist. Mirrors ops.rs create_if_missing().
func createIfMissing(path, contents, msg string) error {
	if _, err := os.Stat(path); err == nil {
		fmt.Fprintf(os.Stderr, "lorri: file already exists, skipping: %s — %s\n", path, msg)
		return nil
	}
	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		return fmt.Errorf("could not write %s: %w", path, err)
	}
	fmt.Fprintf(os.Stderr, "lorri: wrote file: %s\n", path)
	return nil
}

// ---------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------

// opInfo prints lorri project info to stdout.
// Exact output format mirrors ops.rs op_info().
func opInfo(paths *Paths, projectFile ProjectFile) error {
	nixFile := nixFilePathForProject(projectFile)
	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)

	// Try to connect to the daemon for DaemonInfo.
	daemonStatus := daemonInfoStatus(paths)

	// Check whether GC root exists.
	gcRootStr := ""
	if fileExists(gcRootPath.String()) {
		gcRootStr = gcRootPath.String()
	} else {
		gcRootStr = "GC roots do not exist. Has the project been built with lorri yet?"
	}

	fmt.Printf(`Project Shell File: %s
Project Garbage Collector Root: %s

General:
Lorri User GC Root Dir: %s
Lorri Daemon Socket: %s
Lorri Daemon Status: %s
`,
		nixFile,
		gcRootStr,
		paths.GCRootDir,
		paths.DaemonSocketFile,
		daemonStatus,
	)
	return nil
}

// daemonInfoStatus connects to the daemon and returns a human-readable status string.
// Mirrors the daemon_status computation in ops.rs op_info().
func daemonInfoStatus(paths *Paths) string {
	socketPath := NewSocketPath(paths.DaemonSocketFile)
	framing, err := connectClient(socketPath, CommDaemonInfo, defaultReadTimeout)
	if err != nil {
		return fmt.Sprintf("`lorri daemon` is not up: %v", err)
	}
	defer framing.Close()

	// Send empty DaemonInfo request, read empty reply.
	if err := framing.WriteMsg(defaultReadTimeout, struct{}{}); err != nil {
		return fmt.Sprintf("Problem connecting to the `lorri daemon`: %v", err)
	}
	var reply struct{}
	if err := framing.ReadMsg(defaultReadTimeout, &reply); err != nil {
		return fmt.Sprintf("Problem connecting to the `lorri daemon`: %v", err)
	}
	return "`lorri daemon` is running"
}

// ---------------------------------------------------------------------------
// gc
// ---------------------------------------------------------------------------

// GCRootInfo holds metadata about one GC root entry.
// Mirrors project.rs GcRootInfo.
type GCRootInfo struct {
	// GCDir is the per-project directory under ~/.cache/lorri/gc_roots/<hash>/
	GCDir string
	// NixFile is the absolute path to the nix file.
	NixFile string
	// Timestamp is the mtime of the shell_gc_root symlink, or nil if unknown.
	Timestamp *time.Time
	// ProjectExists reports whether NixFile still exists on disk.
	ProjectExists bool
}

// formatPrettyOneline formats a GCRootInfo for human display.
// Mirrors GcRootInfo::format_pretty_oneline().
func (g GCRootInfo) formatPrettyOneline() string {
	age := "sometime in the past"
	if g.Timestamp != nil {
		d := time.Since(*g.Timestamp)
		age = prettyTimeAgo(d)
	}
	alive := ""
	if !g.ProjectExists {
		alive = "[gone] "
	}
	return fmt.Sprintf("%s -> %s %s(%s)", g.GCDir, g.NixFile, alive, age)
}

// prettyTimeAgo formats a duration as a human-friendly string.
func prettyTimeAgo(d time.Duration) string {
	switch {
	case d < time.Minute:
		return fmt.Sprintf("%ds ago", int(d.Seconds()))
	case d < time.Hour:
		return fmt.Sprintf("%dm ago", int(d.Minutes()))
	case d < 24*time.Hour:
		return fmt.Sprintf("%dh ago", int(d.Hours()))
	default:
		return fmt.Sprintf("%dd ago", int(d.Hours()/24))
	}
}

// listGCRoots reads all GC roots from the database and enriches them with
// filesystem metadata.
func listGCRoots(paths *Paths) ([]GCRootInfo, error) {
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return nil, fmt.Errorf("gc: open db: %w", err)
	}
	defer db.Close()

	projects, err := db.ListProjects()
	if err != nil {
		return nil, fmt.Errorf("gc: list projects: %w", err)
	}

	var infos []GCRootInfo
	for _, p := range projects {
		var pf ProjectFile
		if p.IsFlake {
			dir := filepath.Dir(p.NixFile) // context dir from flake.nix path
			pf = NewFlakeProjectFile(mustAbsPath(dir), p.FlakeInstallable)
		} else {
			pf = NewShellNixProjectFile(mustAbsPath(p.NixFile))
		}

		gcRoot := gcRootPathForProject(paths.GCRootDir, pf)
		// GCDir is two levels up from shell_gc_root: <hash>/gc_root/shell_gc_root
		gcDir := gcRoot.Dir().Dir()

		// Read mtime of the shell_gc_root symlink.
		var ts *time.Time
		if fi, err := os.Lstat(gcRoot.String()); err == nil {
			t := fi.ModTime()
			ts = &t
		}

		infos = append(infos, GCRootInfo{
			GCDir:         gcDir.String(),
			NixFile:       p.NixFile,
			Timestamp:     ts,
			ProjectExists: fileExists(p.NixFile),
		})
	}

	// Sort most-recently-built last (mirrors ListRootsSort::MoreRecentLast).
	// Entries with no timestamp sort first.
	sort.Slice(infos, func(i, j int) bool {
		a, b := infos[i], infos[j]
		// nil timestamp sorts first (oldest).
		if a.Timestamp == nil && b.Timestamp == nil {
			return a.NixFile < b.NixFile
		}
		if a.Timestamp == nil {
			return true
		}
		if b.Timestamp == nil {
			return false
		}
		// More recent = larger time = sorts last.
		if !a.Timestamp.Equal(*b.Timestamp) {
			return a.Timestamp.Before(*b.Timestamp)
		}
		return a.NixFile < b.NixFile
	})
	return infos, nil
}

// opGCInfo lists GC roots, optionally as JSON.
func opGCInfo(paths *Paths, jsonOutput bool) error {
	infos, err := listGCRoots(paths)
	if err != nil {
		return err
	}
	if jsonOutput {
		return writeGCInfoJSON(infos)
	}
	for _, info := range infos {
		fmt.Println(info.formatPrettyOneline())
	}
	return nil
}

// wireTimestamp mirrors Rust's SystemTime serde serialization default:
// {"secs_since_epoch": N, "nanos_since_epoch": N}.
type wireTimestamp struct {
	Secs  int64 `json:"secs_since_epoch"`
	Nanos int64 `json:"nanos_since_epoch"`
}

func toWireTimestamp(t *time.Time) *wireTimestamp {
	if t == nil {
		return nil
	}
	return &wireTimestamp{
		Secs:  t.Unix(),
		Nanos: int64(t.Nanosecond()),
	}
}

func writeGCInfoJSON(infos []GCRootInfo) error {
	type jsonRoot struct {
		GCDir     string         `json:"gc_dir"`
		NixFile   string         `json:"nix_file"`
		Timestamp *wireTimestamp `json:"timestamp"`
		Alive     bool           `json:"alive"`
	}
	out := make([]jsonRoot, len(infos))
	for i, info := range infos {
		out[i] = jsonRoot{
			GCDir:     info.GCDir,
			NixFile:   info.NixFile,
			Timestamp: toWireTimestamp(info.Timestamp),
			Alive:     info.ProjectExists,
		}
	}
	return json.NewEncoder(os.Stdout).Encode(out)
}

// GCRmOptions holds options for `lorri gc rm`.
type GCRmOptions struct {
	ShellFiles []string
	All        bool
	OlderThan  *time.Duration
	DryRun     bool
	JSON       bool
}

// gcFilterRoots returns the subset of infos that should be removed given opts.
// Extracted from opGCRm so tests can call it directly.
func gcFilterRoots(infos []GCRootInfo, opts GCRmOptions) []GCRootInfo {
	shellFileSet := make(map[string]bool, len(opts.ShellFiles))
	for _, f := range opts.ShellFiles {
		if abs, err := NewAbsPathFromCwd(f); err == nil {
			shellFileSet[abs.String()] = true
		}
	}

	var toRemove []GCRootInfo
	for _, info := range infos {
		if opts.All {
			toRemove = append(toRemove, info)
			continue
		}
		if !info.ProjectExists {
			toRemove = append(toRemove, info)
			continue
		}
		if shellFileSet[info.NixFile] {
			toRemove = append(toRemove, info)
			continue
		}
		if opts.OlderThan != nil && info.Timestamp != nil {
			if time.Since(*info.Timestamp) > *opts.OlderThan {
				toRemove = append(toRemove, info)
				continue
			}
		}
		if opts.OlderThan != nil && info.Timestamp == nil {
			toRemove = append(toRemove, info)
		}
	}
	return toRemove
}

// opGCRm removes GC roots matching the given criteria.
func opGCRm(paths *Paths, opts GCRmOptions) error {
	infos, err := listGCRoots(paths)
	if err != nil {
		return err
	}

	// Resolve --shell-file paths before filtering (needs error handling).
	shellFileSet := make(map[string]bool, len(opts.ShellFiles))
	for _, f := range opts.ShellFiles {
		abs, err := NewAbsPathFromCwd(f)
		if err != nil {
			return fmt.Errorf("--shell-file: %w", err)
		}
		shellFileSet[abs.String()] = true
	}

	// opts.ShellFiles are already resolved into shellFileSet above;
	// pass them as absolute paths to gcFilterRoots.
	resolvedOpts := opts
	resolvedOpts.ShellFiles = make([]string, 0, len(shellFileSet))
	for k := range shellFileSet {
		resolvedOpts.ShellFiles = append(resolvedOpts.ShellFiles, k)
	}
	toRemove := gcFilterRoots(infos, resolvedOpts)

	if opts.DryRun {
		if len(toRemove) == 0 {
			fmt.Println("--dry-run: Would not delete any GC roots")
		} else {
			fmt.Println("--dry-run: Would delete the following GC roots:")
			for _, info := range toRemove {
				fmt.Println(info.formatPrettyOneline())
			}
		}
		return nil
	}

	// Actually remove.
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return fmt.Errorf("gc rm: open db: %w", err)
	}
	defer db.Close()

	type rmResult struct {
		info GCRootInfo
		err  error
	}
	var results []rmResult
	for _, info := range toRemove {
		var rerr error
		if err := os.RemoveAll(info.GCDir); err != nil {
			rerr = fmt.Errorf("remove %s: %w", info.GCDir, err)
		} else if err := db.DeleteProject(info.NixFile); err != nil {
			rerr = fmt.Errorf("db delete %s: %w", info.NixFile, err)
		}
		results = append(results, rmResult{info, rerr})
	}

	if opts.JSON {
		type jsonRmRoot struct {
			GCDir     string         `json:"gc_dir"`
			NixFile   string         `json:"nix_file"`
			Timestamp *wireTimestamp `json:"timestamp"`
			Alive     bool           `json:"alive"`
		}
		type jsonRmEntry struct {
			Root  jsonRmRoot `json:"root"`
			Error *string    `json:"error"`
		}
		out := make([]jsonRmEntry, len(results))
		for i, r := range results {
			var errStr *string
			if r.err != nil {
				s := r.err.Error()
				errStr = &s
			}
			out[i] = jsonRmEntry{
				Root: jsonRmRoot{
					GCDir:     r.info.GCDir,
					NixFile:   r.info.NixFile,
					Timestamp: toWireTimestamp(r.info.Timestamp),
					Alive:     r.info.ProjectExists,
				},
				Error: errStr,
			}
		}
		return json.NewEncoder(os.Stdout).Encode(out)
	}

	var ok, errCount int
	for _, r := range results {
		if r.err != nil {
			fmt.Fprintf(os.Stderr, "lorri: failed to remove gc root %s: %v\n", r.info.GCDir, r.err)
			errCount++
		} else {
			ok++
		}
	}
	fmt.Printf("Removed %d gc roots.\n", ok)
	if ok > 0 {
		fmt.Println("Remember to run nix-collect-garbage to actually free space.")
	}
	return nil
}

// parseDuration parses human-friendly durations: 30d, 2m, 1y.
// Mirrors cli.rs human_friendly_duration().
func parseDuration(s string) (time.Duration, error) {
	if len(s) < 2 {
		return 0, fmt.Errorf("invalid duration %q: should end with d, m or y", s)
	}
	suffix := s[len(s)-1]
	numStr := s[:len(s)-1]
	n, err := strconv.ParseUint(numStr, 10, 64)
	if err != nil {
		return 0, fmt.Errorf("invalid duration %q: %q is not an integer: %v", s, numStr, err)
	}
	var secs uint64
	switch suffix {
	case 'd':
		secs = n * 24 * 60 * 60
	case 'm':
		secs = n * 30 * 24 * 60 * 60
	case 'y':
		secs = n * 365 * 24 * 60 * 60
	default:
		return 0, fmt.Errorf("invalid duration %q: should end with d, m or y", s)
	}
	return time.Duration(secs) * time.Second, nil
}

// ---------------------------------------------------------------------------
// prompt
// ---------------------------------------------------------------------------

// opPrompt prints "ℓ" (or " ℓ") if the current directory is inside a
// lorri-watched project. Prints nothing otherwise.
// Mirrors op_prompt(PromptOptions::Default { include_leading_space }).
func opPrompt(includeLeadingSpace bool, paths *Paths) error {
	cwd, err := os.Getwd()
	if err != nil {
		// Can't determine cwd; print nothing.
		return nil
	}

	// Open DB read-only so we don't block the daemon.
	conn, err := sqlite.OpenConn(paths.SQLiteDB.String(), sqlite.OpenReadOnly)
	if err != nil {
		// DB doesn't exist yet (no projects watched); print nothing.
		return nil
	}
	defer conn.Close()

	_, found := isSubdirOfKnownProject(conn, cwd)
	if !found {
		return nil
	}

	if includeLeadingSpace {
		fmt.Print(" ℓ")
	} else {
		fmt.Print("ℓ")
	}
	return nil
}

// isSubdirOfKnownProject checks whether path (or any of its ancestors) is
// directly the parent directory of a registered nix_file.
// Returns (nixFile, true) if found, ("", false) otherwise.
// Mirrors is_subdir_of_known_project() using the same SQL logic.
func isSubdirOfKnownProject(conn *sqlite.Conn, path string) (string, bool) {
	// Walk upward through ancestors of path.
	for dir := filepath.Clean(path); ; dir = filepath.Dir(dir) {
		// The SQL check from Rust:
		//   WHERE :path || '/' = rtrim(nix_file, replace(nix_file, '/', ''))
		// This checks that the directory of nix_file equals :path.
		// We replicate this logic in Go: check if nix_file's Dir() == dir.
		var found string
		err := sqlitex.Execute(conn,
			`SELECT nix_file FROM gc_roots
			 WHERE :path || '/' = rtrim(nix_file, replace(nix_file, '/', ''))
			 LIMIT 1`,
			&sqlitex.ExecOptions{
				Named: map[string]any{":path": dir},
				ResultFunc: func(stmt *sqlite.Stmt) error {
					found = stmt.ColumnText(0)
					return nil
				},
			},
		)
		if err == nil && found != "" {
			return found, true
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			// Reached the filesystem root.
			break
		}
	}
	return "", false
}
