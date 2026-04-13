package main

// Builder: run nix-instantiate + nix-build (or nix develop for flakes),
// parse stderr to discover watched paths, return build results.
//
// Mirrors src/builder.rs.

import (
	"bufio"
	_ "embed"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"sync"
)

// loggedEvaluationNix is the Nix instrumentation file, embedded at compile time.
//
//go:embed logged-evaluation.nix
var loggedEvaluationNix string

// ---------------------------------------------------------------------------
// BuildError — mirrors builder.rs BuildError
// ---------------------------------------------------------------------------

// BuildErrorKind classifies a BuildError.
type BuildErrorKind string

const (
	BuildErrorKindIo     BuildErrorKind = "io"
	BuildErrorKindSpawn  BuildErrorKind = "spawn"
	BuildErrorKindExit   BuildErrorKind = "exit"
	BuildErrorKindOutput BuildErrorKind = "output"
)

// BuildError represents a failure during a Nix build.
type BuildError struct {
	Kind   BuildErrorKind
	Cmd    string   // for Spawn and Exit
	Status *int     // for Exit (nil = unknown)
	Logs   []string // for Exit
	Msg    string   // for Io and Output
}

func (e *BuildError) Error() string {
	switch e.Kind {
	case BuildErrorKindIo:
		return fmt.Sprintf("I/O error: %s", e.Msg)
	case BuildErrorKindSpawn:
		return fmt.Sprintf(
			"failed to spawn Nix process. Is Nix installed and on the $PATH?\n$ %s\n%s",
			e.Cmd, e.Msg,
		)
	case BuildErrorKindExit:
		status := "<unknown>"
		if e.Status != nil {
			status = fmt.Sprintf("%d", *e.Status)
		}
		logs := strings.Join(e.Logs, "\n")
		return fmt.Sprintf("Nix process returned exit code %s.\n$ %s\n%s", status, e.Cmd, logs)
	case BuildErrorKindOutput:
		return e.Msg
	default:
		return fmt.Sprintf("build error: %s", e.Msg)
	}
}

func buildErrorIo(msg string) *BuildError {
	return &BuildError{Kind: BuildErrorKindIo, Msg: msg}
}
func buildErrorSpawn(cmd string, msg string) *BuildError {
	return &BuildError{Kind: BuildErrorKindSpawn, Cmd: cmd, Msg: msg}
}
func buildErrorExit(cmd string, status *int, logs []string) *BuildError {
	return &BuildError{Kind: BuildErrorKindExit, Cmd: cmd, Status: status, Logs: logs}
}
func buildErrorOutput(msg string) *BuildError {
	return &BuildError{Kind: BuildErrorKindOutput, Msg: msg}
}

// ---------------------------------------------------------------------------
// RootedPath — a realized store path kept alive by a temp-dir GC handle
// ---------------------------------------------------------------------------

// RootedPath holds a build result store path plus a temporary directory
// whose existence keeps the GC root alive.
// Call Release() when done (or pass ownership to Project.CreateRoots).
type RootedPath struct {
	// Realized store path (e.g. /nix/store/…-lorri-keep-env-hack-shell)
	Path string
	// For flake builds: the profile store path that bash-export was added from.
	// Empty string for shell.nix builds.
	FlakeProfilePath string
	// Temp directory used as GC root; kept alive as long as this struct is held.
	gcHandle string
}

// Release removes the temp dir, allowing Nix to GC the store paths.
func (r *RootedPath) Release() {
	if r.gcHandle != "" {
		os.RemoveAll(r.gcHandle)
		r.gcHandle = ""
	}
}

// ---------------------------------------------------------------------------
// RunResult — output of a complete instantiate+build cycle
// ---------------------------------------------------------------------------

// RunResult is the output of a full build attempt.
type RunResult struct {
	// All paths discovered during evaluation that should be watched for changes.
	ReferencedPaths []WatchPathBuf
	// The realized build outputs.
	Result RootedPath
}

// ---------------------------------------------------------------------------
// LogDatum — structured line from nix-instantiate -vv stderr
// ---------------------------------------------------------------------------

type logDatumKind int

const (
	logNixSourceFile   logDatumKind = iota // evaluating file '...'
	logCopiedSource                        // copied source '...' -> '...'
	logReadRecursively                     // trace: lorri read: '...'
	logReadDir                             // trace: lorri readdir: '...'
	logText                                // anything else (UTF-8)
	logNonUtf                              // non-UTF-8 bytes (passed through as string)
)

type logDatum struct {
	kind logDatumKind
	path string // for source/file kinds
	text string // for text/nonUtf kinds
}

// ---------------------------------------------------------------------------
// parseEvaluationLine — 4 static regexes, mirrors parse_evaluation_line()
// ---------------------------------------------------------------------------

var (
	reEvalFile     = regexp.MustCompile(`^evaluating file '(?P<source>.*)'$`)
	reCopiedSource = regexp.MustCompile(`^copied source '(?P<source>.*)' -> '(?:.*)'$`)
	reLorriRead    = regexp.MustCompile(`^trace: lorri read: '(?P<source>.*)'$`)
	reLorriReadDir = regexp.MustCompile(`^trace: lorri readdir: '(?P<source>.*)'$`)
)

func parseEvaluationLine(line string) logDatum {
	if m := reEvalFile.FindStringSubmatch(line); m != nil {
		return logDatum{kind: logNixSourceFile, path: m[reEvalFile.SubexpIndex("source")]}
	}
	if m := reCopiedSource.FindStringSubmatch(line); m != nil {
		return logDatum{kind: logCopiedSource, path: m[reCopiedSource.SubexpIndex("source")]}
	}
	if m := reLorriRead.FindStringSubmatch(line); m != nil {
		return logDatum{kind: logReadRecursively, path: m[reLorriRead.SubexpIndex("source")]}
	}
	if m := reLorriReadDir.FindStringSubmatch(line); m != nil {
		return logDatum{kind: logReadDir, path: m[reLorriReadDir.SubexpIndex("source")]}
	}
	return logDatum{kind: logText, text: line}
}

// logDatumToWatchPath converts a logDatum to a WatchPathBuf if it represents
// a path that should be watched. Returns (path, true) or (zero, false).
// Mirrors the match arms in instrumented_instantiation.
func logDatumToWatchPath(d logDatum) (WatchPathBuf, bool) {
	switch d.kind {
	case logCopiedSource, logReadRecursively:
		return WatchPathBuf{Recursive: true, Path: d.path}, true
	case logReadDir:
		return WatchPathBuf{Recursive: false, Path: d.path}, true
	case logNixSourceFile:
		p := d.path
		// Emulate Nix's default.nix mechanism: if we saw a directory, Nix
		// actually imported <dir>/default.nix — watch that file, not the dir.
		if info, err := os.Stat(p); err == nil && info.IsDir() {
			p = filepath.Join(p, "default.nix")
		}
		return WatchPathBuf{Recursive: false, Path: p}, true
	default:
		return WatchPathBuf{}, false
	}
}

// ---------------------------------------------------------------------------
// NixDevParser — stateful flake log parser, mirrors NixDevParser in Rust
// ---------------------------------------------------------------------------

// NixDevParser is a stateful parser for `nix develop --debug` stderr.
// It dynamically builds regexes as it encounters flake/tree references.
type NixDevParser struct {
	// flakeRees maps flake URL → (regex matching "got tree '...' from '<flakeURL>'", absPath)
	flakeRees map[string]regexEntry
	// treeRees maps tree store path → (regex matching "checking access to '<tree>/...'", absPath)
	treeRees map[string]regexEntry
}

// regexEntry pairs a compiled regexp with the absolute path it was built from.
type regexEntry struct {
	re      *regexp.Regexp
	absPath string
}

// reEvalDrv matches lines like:
//
//	evaluating derivation 'git+file:///home/user/proj#devShells.x86_64-linux.default'...
var reEvalDrv = regexp.MustCompile(
	`evaluating derivation '(?P<flake>git\+file://(?P<path>[^?]*)[^#]*)#\S*'\.\.\.`)

func newNixDevParser() *NixDevParser {
	return &NixDevParser{
		flakeRees: make(map[string]regexEntry),
		treeRees:  make(map[string]regexEntry),
	}
}

// Parse processes one stderr line and returns a logDatum.
func (p *NixDevParser) Parse(line string) logDatum {
	// Phase 1: check if this line matches any already-known tree access pattern.
	for _, entry := range p.treeRees {
		if m := entry.re.FindStringSubmatch(line); m != nil {
			file := m[entry.re.SubexpIndex("file")]
			fullPath := filepath.Join(entry.absPath, file)
			return logDatum{kind: logReadRecursively, path: fullPath}
		}
	}

	// Phase 2: check if this line matches a "got tree" line for a known flake.
	for flakeName, entry := range p.flakeRees {
		if m := entry.re.FindStringSubmatch(line); m != nil {
			tree := m[entry.re.SubexpIndex("tree")]
			// Add a new treeRees entry for this tree.
			treeRe := regexp.MustCompile(fmt.Sprintf(
				"checking access to '%s/(?P<file>[^']*)'",
				regexp.QuoteMeta(tree),
			))
			p.treeRees[tree] = regexEntry{re: treeRe, absPath: entry.absPath}
			// Remove this flake entry — we've matched it.
			delete(p.flakeRees, flakeName)
			return logDatum{kind: logText, text: line}
		}
	}

	// Phase 3: check for a new "evaluating derivation" line.
	if m := reEvalDrv.FindStringSubmatch(line); m != nil {
		flakeURL := m[reEvalDrv.SubexpIndex("flake")]
		sourcePath := m[reEvalDrv.SubexpIndex("path")]
		gotTreeRe := regexp.MustCompile(fmt.Sprintf(
			"got tree '(?P<tree>[^']*)' from '%s'",
			regexp.QuoteMeta(flakeURL),
		))
		p.flakeRees[flakeURL] = regexEntry{re: gotTreeRe, absPath: sourcePath}
	}

	return logDatum{kind: logText, text: line}
}

// ---------------------------------------------------------------------------
// InstantiateAndBuild — shell.nix path
// ---------------------------------------------------------------------------

// InstantiateAndBuild runs nix-instantiate followed by nix-build for a shell.nix.
// Mirrors builder.rs instantiate_and_build().
func InstantiateAndBuild(
	nixFile string,
	cas *CAS,
	opts NixOptions,
	runTimeClosure string,
) (*RunResult, error) {
	// Write the instrumentation Nix file to the CAS so nix-instantiate can read it.
	loggedEvalPath, err := cas.FileFromString(loggedEvaluationNix)
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("write logged-evaluation.nix to CAS: %v", err))
	}

	// Temp dir used as an indirect GC root for the .drv output.
	gcRootDir, err := os.MkdirTemp("", "lorri-gc-root-*")
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("create gc root temp dir: %v", err))
	}
	// cleanup tracks whether we should remove gcRootDir on exit.
	// Set to false when ownership is transferred to the returned RootedPath.
	cleanup := true
	defer func() {
		if cleanup {
			os.RemoveAll(gcRootDir)
		}
	}()

	// Build the nix-instantiate argument list.
	args := []string{"-vv"}
	args = append(args, opts.ToNixArglist()...)
	args = append(args,
		"--add-root", filepath.Join(gcRootDir, "result"),
		"--indirect",
		"--argstr", "runTimeClosure", runTimeClosure,
		"--argstr", "src", nixFile,
		"--", loggedEvalPath.String(),
	)

	cmd := exec.Command("nix-instantiate", args...)
	cmd.Stdin = nil

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("stdout pipe: %v", err))
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("stderr pipe: %v", err))
	}

	if err := cmd.Start(); err != nil {
		if isNotFound(err) {
			return nil, buildErrorSpawn(cmd.String(), err.Error())
		}
		return nil, buildErrorIo(err.Error())
	}

	// Concurrently read stdout (.drv paths) and stderr (log datums).
	var (
		drvPaths  []string
		logDatums []logDatum
		stdoutErr error
		stderrErr error
		wg        sync.WaitGroup
	)

	wg.Add(2)
	go func() {
		defer wg.Done()
		scanner := bufio.NewScanner(stdoutPipe)
		for scanner.Scan() {
			line := scanner.Text()
			if line != "" {
				drvPaths = append(drvPaths, line)
			}
		}
		stdoutErr = scanner.Err()
	}()
	go func() {
		defer wg.Done()
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logDatums = append(logDatums, parseEvaluationLine(scanner.Text()))
		}
		stderrErr = scanner.Err()
	}()

	wg.Wait()
	exitErr := cmd.Wait()

	if stdoutErr != nil {
		return nil, buildErrorIo(fmt.Sprintf("read stdout: %v", stdoutErr))
	}
	if stderrErr != nil {
		return nil, buildErrorIo(fmt.Sprintf("read stderr: %v", stderrErr))
	}

	// Collect watched paths and log lines from stderr datums.
	var referencedPaths []WatchPathBuf
	var logLines []string
	for _, d := range logDatums {
		if wp, ok := logDatumToWatchPath(d); ok {
			referencedPaths = append(referencedPaths, wp)
		} else {
			logLines = append(logLines, d.text)
		}
	}

	if exitErr != nil {
		status := exitStatus(exitErr)
		return nil, buildErrorExit(cmd.String(), status, logLines)
	}

	// Expect exactly one .drv path on stdout.
	if len(drvPaths) == 0 {
		return nil, buildErrorOutput("logged_evaluation.nix did not return a build product")
	}
	if len(drvPaths) > 1 {
		return nil, buildErrorOutput(fmt.Sprintf(
			"got more than one build product (%d) from logged_evaluation.nix: %v",
			len(drvPaths), drvPaths,
		))
	}
	drvPath := drvPaths[0]

	// Build the .drv.
	rootedPath, err := buildDrv(drvPath, gcRootDir)
	if err != nil {
		return nil, err
	}

	cleanup = false // ownership transferred to rootedPath.gcHandle
	return &RunResult{
		ReferencedPaths: referencedPaths,
		Result:          *rootedPath,
	}, nil
}

// buildDrv runs nix-build on a .drv file and returns the resulting store path.
// Mirrors the nix.rs CallOpts::path() + nix-build invocation in builder.rs build().
func buildDrv(drvPath string, gcRootDir string) (*RootedPath, error) {
	outLink := filepath.Join(gcRootDir, "result")
	cmd := exec.Command("nix-build", "--out-link", outLink, drvPath)
	cmd.Stdin = nil

	out, err := cmd.Output()
	if err != nil {
		if isNotFound(err) {
			return nil, buildErrorSpawn(cmd.String(), err.Error())
		}
		// Extract stderr from ExitError for log lines.
		logs := extractStderrLines(err)
		status := exitStatus(err)
		return nil, buildErrorExit(cmd.String(), status, logs)
	}

	storePath := strings.TrimSpace(string(out))
	if storePath == "" {
		return nil, buildErrorOutput("nix-build produced no output path")
	}

	return &RootedPath{
		Path:     storePath,
		gcHandle: gcRootDir,
	}, nil
}

// ---------------------------------------------------------------------------
// BuildFlake — flake path
// ---------------------------------------------------------------------------

// BuildFlake runs `nix develop` for a flake and returns the build result.
// Mirrors builder.rs flake().
func BuildFlake(fo FlakeOutput) (*RunResult, error) {
	gcRootDir, err := os.MkdirTemp("", "lorri-gc-root-flake-*")
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("create gc root temp dir: %v", err))
	}
	cleanup := true
	defer func() {
		if cleanup {
			os.RemoveAll(gcRootDir)
		}
	}()

	envPath := filepath.Join(gcRootDir, "bash-export")
	profilePath := filepath.Join(gcRootDir, "profile")

	// Run: nix develop --debug --profile <profilePath> <installable> -c bash -c export
	cmd := exec.Command("nix",
		"develop",
		"--debug",
		"--profile", profilePath,
		fo.Installable,
		"-c", "bash",
		"-c", "export",
	)
	cmd.Dir = fo.Context
	cmd.Stdin = nil

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("stdout pipe: %v", err))
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		return nil, buildErrorIo(fmt.Sprintf("stderr pipe: %v", err))
	}

	if err := cmd.Start(); err != nil {
		if isNotFound(err) {
			return nil, buildErrorSpawn(cmd.String(), err.Error())
		}
		return nil, buildErrorIo(err.Error())
	}

	var (
		logDatums []logDatum
		stdoutErr error
		stderrErr error
		wg        sync.WaitGroup
	)
	parser := newNixDevParser()

	// Stdout → write to bash-export file.
	wg.Go(func() {
		f, err := os.Create(envPath)
		if err != nil {
			stdoutErr = err
			io.Copy(io.Discard, stdoutPipe) //nolint:errcheck
			return
		}
		defer f.Close()
		_, stdoutErr = io.Copy(f, stdoutPipe)
	})

	// Stderr → NixDevParser.
	wg.Go(func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logDatums = append(logDatums, parser.Parse(scanner.Text()))
		}
		stderrErr = scanner.Err()
	})

	wg.Wait()
	exitErr := cmd.Wait()

	if stdoutErr != nil {
		return nil, buildErrorIo(fmt.Sprintf("write bash-export: %v", stdoutErr))
	}
	if stderrErr != nil {
		return nil, buildErrorIo(fmt.Sprintf("read stderr: %v", stderrErr))
	}
	if exitErr != nil {
		logs := extractStderrLines(exitErr)
		status := exitStatus(exitErr)
		return nil, buildErrorExit(cmd.String(), status, logs)
	}

	// Collect watched paths.
	var referencedPaths []WatchPathBuf
	for _, d := range logDatums {
		if wp, ok := logDatumToWatchPath(d); ok {
			referencedPaths = append(referencedPaths, wp)
		}
	}

	// Resolve the profile symlink chain (up to 10 hops) to find the real store path.
	profileRoot := profilePath
	for range 10 {
		fi, err := os.Lstat(profileRoot)
		if err != nil {
			break
		}
		if fi.Mode()&os.ModeSymlink == 0 {
			break
		}
		target, err := os.Readlink(profileRoot)
		if err != nil {
			break
		}
		if !filepath.IsAbs(target) {
			target = filepath.Join(filepath.Dir(profileRoot), target)
		}
		profileRoot = target
	}

	// Add the bash-export file to the Nix store.
	addCmd := exec.Command("nix", "store", "add-file", envPath)
	addCmd.Dir = fo.Context
	addOut, err := addCmd.Output()
	if err != nil {
		if isNotFound(err) {
			return nil, buildErrorSpawn(addCmd.String(), err.Error())
		}
		logs := extractStderrLines(err)
		status := exitStatus(err)
		return nil, buildErrorExit(addCmd.String(), status, logs)
	}

	storePath := strings.TrimSpace(string(addOut))
	if storePath == "" {
		return nil, buildErrorOutput("nix store add-file: no store path reported")
	}

	cleanup = false // ownership transferred to RootedPath.gcHandle
	return &RunResult{
		ReferencedPaths: referencedPaths,
		Result: RootedPath{
			Path:             storePath,
			FlakeProfilePath: profileRoot,
			gcHandle:         gcRootDir,
		},
	}, nil
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// isNotFound returns true if err is an "executable not found" error.
// cmd.Start() wraps the underlying os.PathError in an *exec.Error when the
// binary cannot be found on $PATH.
func isNotFound(err error) bool {
	if err == nil {
		return false
	}
	// exec.Command("nonexistent").Start() returns *exec.Error{Err: exec.ErrNotFound}
	if e, ok := err.(*exec.Error); ok && e.Err == exec.ErrNotFound {
		return true
	}
	// Fallback: string match for wrapped errors
	return strings.Contains(err.Error(), "executable file not found")
}

// exitStatus extracts the integer exit code from an exec error, or returns nil.
func exitStatus(err error) *int {
	if err == nil {
		return nil
	}
	if ee, ok := err.(*exec.ExitError); ok {
		code := ee.ExitCode()
		if code != -1 {
			return &code
		}
	}
	return nil
}

// extractStderrLines returns stderr log lines from an ExitError, or nil.
func extractStderrLines(err error) []string {
	if ee, ok := err.(*exec.ExitError); ok && len(ee.Stderr) > 0 {
		var lines []string
		scanner := bufio.NewScanner(strings.NewReader(string(ee.Stderr)))
		for scanner.Scan() {
			lines = append(lines, scanner.Text())
		}
		return lines
	}
	return nil
}
