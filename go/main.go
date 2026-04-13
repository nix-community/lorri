package main

// lorri — Go rewrite entry point.

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// stringSliceFlag implements flag.Value for a repeatable flag that accumulates
// into a []string. Mirrors Rust's Vec<PathBuf> for --shell-file.
type stringSliceFlag []string

func (s *stringSliceFlag) String() string     { return strings.Join(*s, ", ") }
func (s *stringSliceFlag) Set(v string) error { *s = append(*s, v); return nil }

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "lorri: %v\n", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		printUsage()
		return fmt.Errorf("no subcommand given")
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
		return opInit()
	case "prompt":
		return runPrompt(args[1:])
	case "shell":
		return runShell(args[1:])
	case "watch":
		return runWatch(args[1:])
	case "internal":
		return runInternal(args[1:])
	case "-h", "--help", "help":
		printUsage()
		return nil
	default:
		printUsage()
		return fmt.Errorf("unknown subcommand %q", args[0])
	}
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

	opts := EmptyNixOptions()
	if *extraNixOptsJSON != "" {
		var parsed struct {
			Builders     []string `json:"builders"`
			Substituters []string `json:"substituters"`
		}
		if err := json.Unmarshal([]byte(*extraNixOptsJSON), &parsed); err != nil {
			return fmt.Errorf("--extra-nix-options: invalid JSON: %w", err)
		}
		opts.Builders = parsed.Builders
		opts.Substituters = parsed.Substituters
	}

	rtc := requireRTC()
	if rtc == "" {
		return fmt.Errorf("RUN_TIME_CLOSURE environment variable not set; please run lorri from its nix-shell")
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	return NewDaemon(opts).Serve(paths, rtc)
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
		return err
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	projectFile, err := resolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opDirenv(paths, projectFile)
}

// ---------------------------------------------------------------------------
// gc
// ---------------------------------------------------------------------------

func runGC(args []string) error {
	// --json applies to both info and rm, matching Rust's GcOptions layout.
	fs := flag.NewFlagSet("lorri gc", flag.ContinueOnError)
	jsonOut := fs.Bool("json", false, "machine-readable JSON output")
	if err := fs.Parse(args); err != nil {
		return err
	}
	remaining := fs.Args()
	if len(remaining) == 0 {
		fmt.Fprintln(os.Stderr, "usage: lorri gc [--json] <info|rm> [options]")
		return fmt.Errorf("no gc subcommand given")
	}
	switch remaining[0] {
	case "info":
		return runGCInfo(remaining[1:], *jsonOut)
	case "rm":
		return runGCRm(remaining[1:], *jsonOut)
	default:
		return fmt.Errorf("unknown gc subcommand %q", remaining[0])
	}
}

func runGCInfo(args []string, jsonOut bool) error {
	fs := flag.NewFlagSet("lorri gc info", flag.ContinueOnError)
	if err := fs.Parse(args); err != nil {
		return err
	}
	paths, err := InitPaths()
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
	var shellFiles stringSliceFlag
	fs.Var(&shellFiles, "shell-file", "also delete root for this shell file (repeatable)")
	if err := fs.Parse(args); err != nil {
		return err
	}

	opts := GCRmOptions{
		ShellFiles: []string(shellFiles),
		All:        *allFlag,
		DryRun:     *dryRun,
		JSON:       jsonOut,
	}
	if *olderThan != "" {
		d, err := parseDuration(*olderThan)
		if err != nil {
			return fmt.Errorf("--older-than: %w", err)
		}
		opts.OlderThan = &d
	}
	paths, err := InitPaths()
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
		return err
	}
	// info uses SourceOptions (no default — must be explicit).
	if *shellFile == "" && *flake == "" {
		return fmt.Errorf("lorri info requires --shell-file or --flake")
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	projectFile, err := resolveProjectFile(*shellFile, *contextDir, *flake)
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
		return fmt.Errorf("no prompt subcommand given")
	}
	switch args[0] {
	case "default":
		return runPromptDefault(args[1:])
	default:
		return fmt.Errorf("unknown prompt subcommand %q", args[0])
	}
}

func runPromptDefault(args []string) error {
	fs := flag.NewFlagSet("lorri prompt default", flag.ContinueOnError)
	leadingSpace := fs.Bool("include-leading-space", false,
		"include a leading space before the prompt symbol")
	if err := fs.Parse(args); err != nil {
		return err
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	return opPrompt(*leadingSpace, paths)
}

// ---------------------------------------------------------------------------
// shell
// ---------------------------------------------------------------------------

func runShell(args []string) error {
	fs := flag.NewFlagSet("lorri shell", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor")
	cached := fs.Bool("cached", false, "use the most recently built environment")
	if err := fs.Parse(args); err != nil {
		return err
	}
	rtc := requireRTC()
	if rtc == "" {
		return fmt.Errorf("RUN_TIME_CLOSURE not set")
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	projectFile, err := resolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opShell(paths, projectFile, rtc, *cached)
}

// ---------------------------------------------------------------------------
// watch
// ---------------------------------------------------------------------------

func runWatch(args []string) error {
	fs := flag.NewFlagSet("lorri watch", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor")
	once := fs.Bool("once", false, "exit after the first build")
	if err := fs.Parse(args); err != nil {
		return err
	}
	rtc := requireRTC()
	if rtc == "" {
		return fmt.Errorf("RUN_TIME_CLOSURE not set")
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	projectFile, err := resolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opWatch(paths, projectFile, rtc, *once)
}

// ---------------------------------------------------------------------------
// internal subcommands
// ---------------------------------------------------------------------------

func runInternal(args []string) error {
	if len(args) == 0 {
		fmt.Fprintln(os.Stderr, "usage: lorri internal <subcommand>")
		fmt.Fprintln(os.Stderr, "  ping_              tell the daemon to watch a project")
		fmt.Fprintln(os.Stderr, "  stream-events_     stream build events from the daemon")
		fmt.Fprintln(os.Stderr, "  start-user-shell   exec into the user shell (used by lorri shell)")
		return fmt.Errorf("no internal subcommand given")
	}
	switch args[0] {
	case "ping_":
		return runPing(args[1:])
	case "stream-events_":
		return runStreamEvents(args[1:])
	case "start-user-shell":
		return runStartUserShell(args[1:])
	default:
		return fmt.Errorf("unknown internal subcommand %q", args[0])
	}
}

func runPing(args []string) error {
	fs := flag.NewFlagSet("lorri internal ping_", flag.ContinueOnError)
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor (e.g. .#)")
	if err := fs.Parse(args); err != nil {
		return err
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	projectFile, err := resolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opPing(paths, projectFile)
}

func runStreamEvents(args []string) error {
	fs := flag.NewFlagSet("lorri internal stream-events_", flag.ContinueOnError)
	kind := fs.String("kind", "all", "event kind: live, snapshot, or all")
	if err := fs.Parse(args); err != nil {
		return err
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
		return fmt.Errorf("--kind must be live, snapshot, or all (got %q)", *kind)
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	return opStreamEvents(paths, ek)
}

func runStartUserShell(args []string) error {
	fs := flag.NewFlagSet("lorri internal start-user-shell", flag.ContinueOnError)
	shellPath := fs.String("shell-path", "", "path to the user's shell binary (required)")
	shellFile := fs.String("shell-file", "", "path to shell.nix (or similar)")
	contextDir := fs.String("context", ".", "directory to resolve a flake from")
	flake := fs.String("flake", "", "flake installable descriptor")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *shellPath == "" {
		return fmt.Errorf("--shell-path is required")
	}
	paths, err := InitPaths()
	if err != nil {
		return err
	}
	cas, err := NewCAS(paths.CASDir)
	if err != nil {
		return err
	}
	// nixFile not used directly by opStartUserShell but we keep the parse for consistency.
	_, err = resolveProjectFile(*shellFile, *contextDir, *flake)
	if err != nil {
		return err
	}
	return opStartUserShell(*shellPath, "", cas)
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
	fmt.Fprintln(os.Stderr, "  shell     open a project shell")
	fmt.Fprintln(os.Stderr, "  watch     build project whenever an input file changes")
	fmt.Fprintln(os.Stderr, "  internal  plumbing commands")
}
