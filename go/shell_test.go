package main

// Tests for the lorri shell trampoline mechanism.
//
// Requires RUN_TIME_CLOSURE (must run from within the lorri nix-shell).

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// rtcOrSkip returns RUN_TIME_CLOSURE or skips the test.
func rtcOrSkip(t *testing.T) string {
	t.Helper()
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}
	return rtc
}

// TestEnvrcBashSetsPath builds a real Nix environment via the basic integration
// fixture, constructs the same init script that opShell writes, runs it under
// the RTC bash, and verifies that $PATH contains Nix store entries — meaning
// envrc.bash successfully loaded the closure into the environment.
//
// Requires: nix-instantiate, nix-build, nix-store on $PATH and RUN_TIME_CLOSURE set.
func TestEnvrcBashSetsPath(t *testing.T) {
	rtc := rtcOrSkip(t)

	nixFile := filepath.Join(lorriRoot(), "tests/integration/basic/shell.nix")
	if _, err := os.Stat(nixFile); err != nil {
		t.Fatalf("test fixture missing: %s", nixFile)
	}

	dir := t.TempDir()
	cas, err := NewCAS(mustAbsPath(filepath.Join(dir, "cas")))
	if err != nil {
		t.Fatalf("NewCAS: %v", err)
	}

	// Build the environment — this is the same call opShell makes internally.
	result, err := InstantiateAndBuild(nixFile, cas, EmptyNixOptions(), rtc)
	if err != nil {
		t.Fatalf("InstantiateAndBuild: %v", err)
	}
	defer result.Result.Release()

	gcRootDir := mustAbsPath(filepath.Join(dir, "gc_roots"))
	projectFile := NewShellNixProjectFile(mustAbsPath(nixFile))
	gcRootPath := gcRootPathForProject(gcRootDir, projectFile)

	if err := createGCRoot(result.Result.Path, gcRootPath.String()); err != nil {
		t.Fatalf("createGCRoot: %v", err)
	}
	t.Logf("gc root: %s", gcRootPath)

	// Construct the same init script that opShell writes to CAS:
	//   EVALUATION_ROOT="<gc_root_path>"
	//   <contents of envrc.bash>
	initContent := fmt.Sprintf("\nEVALUATION_ROOT=%q\n\n%s", gcRootPath.String(), envrcBash)
	initFile := filepath.Join(dir, "bash_env_init.sh")
	if err := os.WriteFile(initFile, []byte(initContent), 0o644); err != nil {
		t.Fatalf("write init script: %v", err)
	}

	// Run the RTC bash with BASH_ENV pointing at the init script.
	// After sourcing, $PATH should contain Nix store paths from the closure.
	bashPath, err := bashFromRTC(rtc)
	if err != nil {
		t.Fatalf("bashFromRTC: %v", err)
	}

	cmd := exec.Command(bashPath, "-c", "echo $PATH")
	cmd.Env = append(os.Environ(), "BASH_ENV="+initFile)
	out, err := cmd.Output()
	if err != nil {
		t.Fatalf("bash with BASH_ENV: %v", err)
	}

	path := strings.TrimSpace(string(out))
	t.Logf("PATH after sourcing init: %s", path)

	if !strings.Contains(path, "/nix/store/") {
		t.Errorf("expected /nix/store/ entries in PATH after sourcing envrc.bash, got: %s", path)
	}
}
