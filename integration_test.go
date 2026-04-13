package main

// Integration tests against the real test fixtures in tests/integration/.
// All tests are gated on RUN_TIME_CLOSURE being set — they require a full
// Nix installation and are only run from within the lorri nix-shell.

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// lorriRoot returns the repository root.
func lorriRoot() string {
	// The test binary runs from the repo root (module root).
	abs, err := filepath.Abs(".")
	if err != nil {
		panic(err)
	}
	return abs
}

// rtcOrSkip returns RUN_TIME_CLOSURE or skips the test.
func rtcOrSkip(t *testing.T) string {
	t.Helper()
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}
	return rtc
}

// writeLoggedEvalForTest writes loggedEvaluationNix to dir/logged-evaluation.nix
// and returns the path. Fatals on error.
func writeLoggedEvalForTest(t *testing.T, dir string) AbsPath {
	t.Helper()
	p := mustAbsPath(filepath.Join(dir, "logged-evaluation.nix"))
	if err := writeFileIfChanged(string(p), loggedEvaluationNix, 0o644); err != nil {
		t.Fatalf("writeLoggedEvalForTest: %v", err)
	}
	return p
}

// TestIntegrationBasicFlake builds tests/integration/basic-flake/ via BuildFlake.
func TestIntegrationBasicFlake(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}
	_ = rtc // BuildFlake doesn't need RTC directly but gate on it for consistency

	flakeDir := filepath.Join(lorriRoot(), "tests/integration/basic-flake")
	if _, err := os.Stat(filepath.Join(flakeDir, "flake.nix")); err != nil {
		t.Fatalf("test fixture missing: %s/flake.nix", flakeDir)
	}

	fo := FlakeOutput{
		Context:     flakeDir,
		Installable: ".#",
	}

	result, err := BuildFlake(fo)
	if err != nil {
		t.Fatalf("BuildFlake: %v", err)
	}
	defer result.Result.Release()

	if result.Result.Path == "" {
		t.Fatal("expected non-empty store path")
	}
	t.Logf("store path: %s", result.Result.Path)

	if _, err := os.Stat(result.Result.Path); err != nil {
		t.Errorf("store path does not exist: %s: %v", result.Result.Path, err)
	}
}

// TestIntegrationDirenvMatchesRust verifies that the Go lorri direnv output
// is byte-for-byte identical to the Rust lorri direnv output for the same
// project file.
func TestIntegrationDirenvMatchesRust(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	// Check the Rust lorri binary is available.
	rustLorri, err := exec.LookPath("lorri")
	if err != nil {
		t.Skip("Rust lorri not on PATH — skipping diff test")
	}

	nixFile := filepath.Join(lorriRoot(), "tests/integration/basic/shell.nix")

	// Capture Rust lorri direnv output (stderr ignored — it's status messages).
	rustCmd := exec.Command(rustLorri, "direnv", "--shell-file", nixFile)
	rustOut, err := rustCmd.Output()
	if err != nil {
		t.Fatalf("rust lorri direnv: %v", err)
	}

	// Use the real XDG paths so the output matches the Rust binary's paths exactly.
	paths, err := InitPaths()
	if err != nil {
		t.Fatalf("InitPaths: %v", err)
	}

	// Capture Go lorri direnv output via an explicit io.Writer (no os.Stdout mutation).
	var goOutBuf bytes.Buffer
	projectFile := NewShellNixProjectFile(mustAbsPath(nixFile))
	opErr := opDirenv(&goOutBuf, paths, projectFile)
	goOut := goOutBuf.Bytes()

	if opErr != nil {
		t.Fatalf("go opDirenv: %v", opErr)
	}

	if !bytes.Equal(rustOut, goOut) {
		// Show a line-by-line diff for easy debugging.
		rustLines := strings.Split(string(rustOut), "\n")
		goLines := strings.Split(string(goOut), "\n")
		t.Errorf("direnv output mismatch (rust %d lines, go %d lines)",
			len(rustLines), len(goLines))
		maxLines := max(len(goLines), len(rustLines))
		for i := 0; i < maxLines; i++ {
			rl, gl := "", ""
			if i < len(rustLines) {
				rl = rustLines[i]
			}
			if i < len(goLines) {
				gl = goLines[i]
			}
			if rl != gl {
				t.Errorf("line %d differs:\n  rust: %q\n  go:   %q", i+1, rl, gl)
			}
		}
	} else {
		t.Logf("direnv output: identical (%d bytes)", len(rustOut))
	}
}
