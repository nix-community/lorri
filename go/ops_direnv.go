package main

// op_direnv: emit a shell script for direnv to eval.
// Mirrors src/ops.rs op_direnv() + src/ops/direnv.rs DirenvVersion.
//
// Usage (from .envrc):
//   eval "$(lorri direnv)"
//
// Stdout is the shell script; all logging goes to stderr.

import (
	"crypto/md5" //nolint:gosec // MD5 used for path-keying, not cryptographic security
	_ "embed"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
	"strings"
)

// envrcBash is the bash helper sourced by direnv to load the Nix environment.
// Embedded at compile time from envrc.bash (identical to src/ops/direnv/envrc.bash).
//
//go:embed envrc.bash
var envrcBash string

// minDirenvVersion is the minimum direnv version lorri requires.
const minDirenvMajor, minDirenvMinor, minDirenvPatch = 2, 19, 2

// direnvVersion is a parsed semantic version triple.
type direnvVersion struct {
	major, minor, patch int
}

// parseDirenvVersion parses "major.minor.patch", e.g. "2.19.2".
func parseDirenvVersion(s string) (direnvVersion, error) {
	parts := strings.Split(strings.TrimSpace(s), ".")
	if len(parts) != 3 {
		return direnvVersion{}, fmt.Errorf("expected major.minor.patch, got %q", s)
	}
	parse := func(p string) (int, error) {
		n, err := strconv.Atoi(p)
		if err != nil {
			return 0, fmt.Errorf("not an integer: %q", p)
		}
		return n, nil
	}
	major, err := parse(parts[0])
	if err != nil {
		return direnvVersion{}, err
	}
	minor, err := parse(parts[1])
	if err != nil {
		return direnvVersion{}, err
	}
	patch, err := parse(parts[2])
	if err != nil {
		return direnvVersion{}, err
	}
	return direnvVersion{major, minor, patch}, nil
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
			return fmt.Errorf("`direnv`: executable not found")
		}
		return fmt.Errorf("could not run `direnv version`: %w", err)
	}

	ver, err := parseDirenvVersion(string(out))
	if err != nil {
		return fmt.Errorf("could not figure out the current `direnv` version (parse error): %w", err)
	}

	min := direnvVersion{minDirenvMajor, minDirenvMinor, minDirenvPatch}
	if ver.lt(min) {
		return fmt.Errorf(
			"`direnv` is version %s, but >= %s is required for lorri to function",
			ver, min,
		)
	}
	return nil
}

// gcRootPathForProject computes the shell GC root path for a project.
// Mirrors project.rs Project::new_internal + Project::shell_gc_root:
//
//	hash = md5(nixFilePath bytes)
//	path = <gcRootDir>/<hexhash>/gc_root/shell_gc_root
func gcRootPathForProject(gcRootDir AbsPath, projectFile ProjectFile) AbsPath {
	nixFilePath := nixFilePathForProject(projectFile)
	//nolint:gosec // MD5 for path-keying only
	hash := md5.Sum([]byte(nixFilePath))
	hexHash := fmt.Sprintf("%x", hash)
	return gcRootDir.Join(hexHash, "gc_root", "shell_gc_root")
}

// nixFilePathForProject returns the nix file path string used for hashing.
// Mirrors ProjectFile::as_absolute_path() → as_nix_file() in Rust:
//
//	ShellNix(path)         → path
//	FlakeNix{context, ...} → context/flake.nix
func nixFilePathForProject(projectFile ProjectFile) string {
	if projectFile.ShellNix != nil {
		return projectFile.ShellNix.path
	}
	if projectFile.FlakeNix != nil {
		return AbsPath(projectFile.FlakeNix.Context).Join("flake.nix").String()
	}
	panic("ProjectFile has neither ShellNix nor FlakeNix set")
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

// fileExists reports whether path exists (as any filesystem object).
func fileExists(path string) bool {
	_, err := os.Lstat(path)
	return err == nil
}
