package main

// Integration tests against the real test fixtures in tests/.
// All tests are gated on RUN_TIME_CLOSURE being set — they require a full
// Nix installation and are only run from within the lorri nix-shell.

import (
	"os"
	"path/filepath"
	"testing"
)

// lorriRoot returns the repository root from $LORRI_ROOT, falling back to
// filepath.Abs(".") when running outside the nix-shell.
func lorriRoot() string {
	if root := os.Getenv("LORRI_ROOT"); root != "" {
		return root
	}
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

// TestIntegrationBasicFlake builds tests/basic-flake/ via BuildFlake.
func TestIntegrationBasicFlake(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}
	_ = rtc // BuildFlake doesn't need RTC directly but gate on it for consistency

	flakeDir := filepath.Join(lorriRoot(), "tests/basic-flake")
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


