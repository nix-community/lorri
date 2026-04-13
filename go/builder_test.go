package main

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

func TestInstantiateAndBuild(t *testing.T) {
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	if rtc == "" {
		t.Skip("RUN_TIME_CLOSURE not set — run from lorri nix-shell")
	}

	cas, err := NewCAS(mustAbsPath(os.TempDir() + "/lorri-test-cas"))
	if err != nil {
		t.Fatalf("cas: %v", err)
	}

	result, err := InstantiateAndBuild(
		filepath.Join(lorriRoot(), "shell.nix"),
		cas,
		NixOptions{},
		rtc,
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
}
