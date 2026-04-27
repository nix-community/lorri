package main

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// fixtureNix returns the absolute path to a test fixture under tests/.
func fixtureNix(name string) string {
	return filepath.Join(lorriRoot(), "tests", name)
}

func TestInstantiateAndBuild(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	nixFile := fixtureNix("basic/shell.nix")
	loggedEvalFile := writeLoggedEvalForTest(t, t.TempDir())

	result, err := InstantiateAndBuild(
		nixFile,
		loggedEvalFile,
		NixOptions{},
		rtc,
		lorriBinForTest(t),
	)
	if err != nil {
		t.Fatalf("build error: %v", err)
	}
	defer result.Result.Release()

	if result.Result.Path == "" {
		t.Fatal("expected a store path, got empty string")
	}
	t.Logf("store path: %s", result.Result.Path)
	t.Logf("referenced paths (raw): %d", len(result.ReferencedPaths))

	reduced := ReducePaths(result.ReferencedPaths)
	t.Logf("referenced paths (reduced): %d", len(reduced))
	for _, p := range reduced {
		rec := " "
		if p.Recursive {
			rec = "R"
		}
		fmt.Printf("  [%s] %s\n", rec, p.Path)
	}

	if len(reduced) == 0 {
		t.Fatal("expected at least one referenced path after reduction")
	}

	// ── Verify env.json contents ─────────────────────────────────────────────
	// The keep-env-hack builder must have run generate-env_ and produced a
	// valid env.json with the expected variables from tests/basic/shell.nix.
	envJSON, err := readEnvJSON(filepath.Join(result.Result.Path, "env.json"))
	if err != nil {
		t.Fatalf("read env.json: %v", err)
	}

	byName := make(map[string]EnvChange, len(envJSON.Env))
	for _, c := range envJSON.Env {
		byName[c.Name] = c
	}

	// MARKER=present is set directly in tests/basic/shell.nix.
	if c, ok := byName["MARKER"]; !ok {
		t.Error("env.json missing MARKER")
	} else if c.Op != "set" || c.Value != "present" {
		t.Errorf("MARKER = {op:%q value:%q}, want {set present}", c.Op, c.Value)
	}

	// IN_LORRI_SHELL must be set to the absolute path of the nix file.
	absNixFile, _ := filepath.Abs(nixFile)
	if c, ok := byName["IN_LORRI_SHELL"]; !ok {
		t.Error("env.json missing IN_LORRI_SHELL")
	} else if c.Op != "set" || c.Value != absNixFile {
		t.Errorf("IN_LORRI_SHELL = {op:%q value:%q}, want {set %q}", c.Op, c.Value, absNixFile)
	}
}
