package main

// lorri — Go rewrite entry point.

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
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
		fmt.Fprintln(os.Stderr, "usage: lorri gc [--json] <info|rm> [options]")
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
		fmt.Fprintln(os.Stderr, "usage: lorri prompt <default> [options]")
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
		fmt.Fprintln(os.Stderr, "usage: lorri internal <subcommand>")
		fmt.Fprintln(os.Stderr, "  ping_              tell the daemon to watch a project")
		fmt.Fprintln(os.Stderr, "  stream-events_     stream build events from the daemon")
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
	fmt.Fprintln(os.Stderr, "usage: lorri <subcommand> [options]")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "subcommands:")
	fmt.Fprintln(os.Stderr, "  daemon    start the lorri daemon")
	fmt.Fprintln(os.Stderr, "  direnv    emit shell script for direnv to eval")
	fmt.Fprintln(os.Stderr, "  gc        manage lorri garbage collection roots")
	fmt.Fprintln(os.Stderr, "  info      show information about a lorri project")
	fmt.Fprintln(os.Stderr, "  init      write bootstrap files to current directory")
	fmt.Fprintln(os.Stderr, "  prompt    generate lorri status markers for shell prompts")
	fmt.Fprintln(os.Stderr, "  internal  plumbing commands")
}
