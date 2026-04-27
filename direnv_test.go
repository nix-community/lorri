package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	direnv "github.com/nix-community/lorri/direnv_vendor"
)

// ---------------------------------------------------------------------------
// Unit tests for env.json generation (buildLorriEnv / parseVarmap / applyChange)
// ---------------------------------------------------------------------------

// TestInLorriShellVar verifies that IN_LORRI_SHELL is set to the nix file path
// in the generated env.json. The keep-env-hack builder exports IN_LORRI_SHELL
// before sourcing stdenv setup, so generate-env_ must pass it through as a
// "set" change.
func TestInLorriShellVar(t *testing.T) {
	nixFile := "/home/alice/project/shell.nix"
	t.Setenv("IN_LORRI_SHELL", nixFile)

	env, err := buildLorriEnv(os.DevNull) // no varmap
	if err != nil {
		t.Fatalf("buildLorriEnv: %v", err)
	}

	for _, c := range env.Env {
		if c.Name == "IN_LORRI_SHELL" {
			if c.Op != "set" {
				t.Errorf("IN_LORRI_SHELL op = %q, want set", c.Op)
			}
			if c.Value != nixFile {
				t.Errorf("IN_LORRI_SHELL value = %q, want %q", c.Value, nixFile)
			}
			return
		}
	}
	t.Errorf("IN_LORRI_SHELL not found in env changes")
}

// TestOrigPreHookRemapping verifies that origPreHook is remapped to preHook
// and that no extra variables leak into the generated env.json.
// This is a regression test for github.com/target/lorri/issues/97.
func TestOrigPreHookRemapping(t *testing.T) {
	t.Setenv("origPreHook", "echo 'foo bar'")
	// Ensure preHook itself is absent so we can check only the remapped value.
	os.Unsetenv("preHook") //nolint:errcheck

	env, err := buildLorriEnv(os.DevNull)
	if err != nil {
		t.Fatalf("buildLorriEnv: %v", err)
	}

	var foundPreHook bool
	var foundOrigPreHook bool
	for _, c := range env.Env {
		if c.Name == "preHook" {
			foundPreHook = true
			if c.Op != "set" {
				t.Errorf("preHook op = %q, want set", c.Op)
			}
			if c.Value != "echo 'foo bar'" {
				t.Errorf("preHook value = %q, want \"echo 'foo bar'\"", c.Value)
			}
		}
		if c.Name == "origPreHook" {
			foundOrigPreHook = true
		}
	}
	if !foundPreHook {
		t.Error("preHook not found in env changes")
	}
	if foundOrigPreHook {
		t.Error("origPreHook should not appear in env changes (must be remapped)")
	}
}

// TestVarmapDedupAppends verifies that duplicate (varname, separator) pairs in
// the varmap are deduplicated — only the first occurrence is kept. This is a
// regression test for github.com/target/lorri/issues/110.
func TestVarmapDedupAppends(t *testing.T) {
	// Write a varmap with the same (ITWORKED, :) pair repeated many times,
	// as a slow shell setup hook might produce.
	var varmap []byte
	for range 1000 {
		varmap = append(varmap, []byte("append\x00ITWORKED\x00:\x00")...)
	}

	dir := t.TempDir()
	varmapPath := filepath.Join(dir, "lorri-varmap")
	if err := os.WriteFile(varmapPath, varmap, 0o644); err != nil {
		t.Fatalf("write varmap: %v", err)
	}

	entries, err := parseVarmap(varmapPath)
	if err != nil {
		t.Fatalf("parseVarmap: %v", err)
	}

	if len(entries) != 1 {
		t.Errorf("expected 1 deduped entry, got %d", len(entries))
	}
	if len(entries) > 0 {
		if entries[0].name != "ITWORKED" || entries[0].sep != ":" {
			t.Errorf("unexpected entry: %+v", entries[0])
		}
	}
}

// TestPathWithSpacesInPrepend verifies that PATH values containing spaces are
// handled correctly by applyChange when prepending. This is a regression test
// for github.com/target/lorri/issues/18.
func TestPathWithSpacesInPrepend(t *testing.T) {
	exemplar := "/mnt/c/Program Files (x86)/QuickTime/QTSystem"
	ambient := map[string]string{
		"PATH": "/usr/bin:" + exemplar,
	}

	export := make(direnv.ShellExport)
	applyChange(EnvChange{Op: "prepend", Name: "PATH", Value: "/nix/store/abc/bin", Sep: ":"}, ambient, export)

	val, ok := export["PATH"]
	if !ok || val == nil {
		t.Fatal("PATH not set in export")
	}
	want := "/nix/store/abc/bin:/usr/bin:" + exemplar
	if *val != want {
		t.Errorf("PATH = %q, want %q", *val, want)
	}
}

// ---------------------------------------------------------------------------
// Helpers shared by other test files
// ---------------------------------------------------------------------------

// captureStdout temporarily redirects os.Stdout to a pipe, runs f, and
// returns what was written.
func captureStdout(t *testing.T, f func()) string {
	t.Helper()
	old := os.Stdout
	r, w, err := os.Pipe()
	if err != nil {
		t.Fatalf("os.Pipe: %v", err)
	}
	os.Stdout = w

	f()

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer
	if _, err := buf.ReadFrom(r); err != nil {
		t.Fatalf("read from pipe: %v", err)
	}
	return buf.String()
}

// ---------------------------------------------------------------------------
// test_op_gc.rs port — full GC lifecycle
// ---------------------------------------------------------------------------

func TestOpGC(t *testing.T) {
	rtcOrSkip(t)

	dir := t.TempDir()

	// Write a minimal shell.nix that builds successfully.
	projectDir := filepath.Join(dir, "project")
	if err := os.MkdirAll(projectDir, 0o755); err != nil {
		t.Fatalf("mkdir project: %v", err)
	}
	nixFile := filepath.Join(projectDir, "shell.nix")
	if err := os.WriteFile(nixFile, []byte(`
derivation {
  name = "lorri-gc-test";
  builder = "/bin/sh";
  args = [ "-c" "echo > $out" ];
  system = builtins.currentSystem;
  allowSubstitutes = false;
  preferLocalBuild = true;
}
`), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}

	paths := &Paths{
		GCRootDir:        mustAbsPath(filepath.Join(dir, "gc_roots")),
		DaemonSocketFile: mustAbsPath(filepath.Join(dir, "daemon.socket")),
		SQLiteDB:         mustAbsPath(filepath.Join(dir, "lorri.sqlite")),
		LoggedEvalFile:   writeLoggedEvalForTest(t, dir),
	}
	if err := os.MkdirAll(string(paths.GCRootDir), 0o755); err != nil {
		t.Fatalf("mkdir gc_roots: %v", err)
	}

	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		t.Fatalf("OpenLorriDB: %v", err)
	}
	defer db.Close()

	projectFile := NewShellNixProjectFile(mustAbsPath(nixFile))
	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)

	// ── Build the project ────────────────────────────────────────────────────
	rtc := os.Getenv("RUN_TIME_CLOSURE")
	result, err := InstantiateAndBuild(nixFile, paths.LoggedEvalFile, NixOptions{}, rtc, lorriBinForTest(t))
	if err != nil {
		t.Fatalf("InstantiateAndBuild: %v", err)
	}
	defer result.Result.Release()

	if err := createGCRoot(result.Result.Path, gcRootPath.String()); err != nil {
		t.Fatalf("createGCRoot: %v", err)
	}

	if err := db.UpsertProject(nixFile, false, ""); err != nil {
		t.Fatalf("UpsertProject: %v", err)
	}

	// ── Verify exactly one GC root was created ───────────────────────────────
	entries, err := os.ReadDir(string(paths.GCRootDir))
	if err != nil {
		t.Fatalf("readdir gc_roots: %v", err)
	}
	if len(entries) != 1 {
		t.Fatalf("expected 1 GC root dir, got %d", len(entries))
	}
	gcDir := filepath.Join(string(paths.GCRootDir), entries[0].Name())

	// The shell_gc_root symlink must point into /nix/store.
	shellGCRoot := filepath.Join(gcDir, "gc_root", "shell_gc_root")
	link, err := os.Readlink(shellGCRoot)
	if err != nil {
		t.Fatalf("readlink shell_gc_root: %v", err)
	}
	if !strings.HasPrefix(link, "/nix/store") {
		t.Errorf("shell_gc_root -> %q, want /nix/store prefix", link)
	}

	// ── Default gc rm should have nothing to remove (nix file still exists) ──
	infos, err := listGCRoots(paths)
	if err != nil {
		t.Fatalf("listGCRoots: %v", err)
	}
	toRemove := gcFilterRoots(infos, GCRmOptions{})
	if len(toRemove) != 0 {
		t.Errorf("expected 0 roots to remove when project exists, got %d", len(toRemove))
	}

	// ── gc info --json should contain the gc dir path ─────────────────────────
	gcInfoJSON := captureStdout(t, func() {
		if err := opGCInfo(paths, true); err != nil {
			t.Fatalf("opGCInfo: %v", err)
		}
	})
	if !strings.Contains(gcInfoJSON, gcDir) {
		t.Errorf("gc info json should contain gc dir %q\ngot: %s", gcDir, gcInfoJSON)
	}

	// ── Remove the nix file ───────────────────────────────────────────────────
	backupFile := nixFile + ".bak"
	if err := os.Rename(nixFile, backupFile); err != nil {
		t.Fatalf("rename: %v", err)
	}

	// ── gc info (human) should show [gone] ────────────────────────────────────
	infos, err = listGCRoots(paths)
	if err != nil {
		t.Fatalf("listGCRoots after rename: %v", err)
	}
	humanOut := captureStdout(t, func() {
		if err := opGCInfo(paths, false); err != nil {
			t.Fatalf("opGCInfo human: %v", err)
		}
	})
	if !strings.Contains(humanOut, gcDir) {
		t.Errorf("human gc info should contain gc dir %q\ngot: %s", gcDir, humanOut)
	}
	if !strings.Contains(humanOut, "[gone]") {
		t.Errorf("human gc info should contain [gone]\ngot: %s", humanOut)
	}

	// ── gc rm should remove exactly 1 root ────────────────────────────────────
	toRemove = gcFilterRoots(infos, GCRmOptions{})
	if len(toRemove) != 1 {
		t.Fatalf("expected 1 root to remove after nix file gone, got %d", len(toRemove))
	}
	gcRmJSON := captureStdout(t, func() {
		if err := opGCRm(paths, GCRmOptions{JSON: true}); err != nil {
			t.Fatalf("opGCRm: %v", err)
		}
	})
	if !strings.Contains(gcRmJSON, gcDir) {
		t.Errorf("gc rm json should contain gc dir %q\ngot: %s", gcDir, gcRmJSON)
	}

	// ── Running gc rm again should find nothing to remove ────────────────────
	infos, err = listGCRoots(paths)
	if err != nil {
		t.Fatalf("listGCRoots after rm: %v", err)
	}
	toRemove = gcFilterRoots(infos, GCRmOptions{})
	if len(toRemove) != 0 {
		t.Errorf("expected 0 roots after gc rm, got %d", len(toRemove))
	}

	// ── Restore the nix file and rebuild ─────────────────────────────────────
	if err := os.Rename(backupFile, nixFile); err != nil {
		t.Fatalf("rename back: %v", err)
	}
	result2, err := InstantiateAndBuild(nixFile, paths.LoggedEvalFile, NixOptions{}, rtc, lorriBinForTest(t))
	if err != nil {
		t.Fatalf("rebuild: %v", err)
	}
	defer result2.Result.Release()
	if err := createGCRoot(result2.Result.Path, gcRootPath.String()); err != nil {
		t.Fatalf("createGCRoot after rebuild: %v", err)
	}
	if err := db.UpsertProject(nixFile, false, ""); err != nil {
		t.Fatalf("UpsertProject after rebuild: %v", err)
	}

	// ── gc info --json should again contain the gc dir and nix file ───────────
	gcInfoJSON2 := captureStdout(t, func() {
		if err := opGCInfo(paths, true); err != nil {
			t.Fatalf("opGCInfo after rebuild: %v", err)
		}
	})
	if !strings.Contains(gcInfoJSON2, gcDir) {
		t.Errorf("gc info after rebuild should contain gc dir %q\ngot: %s", gcDir, gcInfoJSON2)
	}
	if !strings.Contains(gcInfoJSON2, nixFile) {
		t.Errorf("gc info after rebuild should contain nix file %q\ngot: %s", nixFile, gcInfoJSON2)
	}
}

// lorriBinForTest returns the path to the test binary itself, which serves as
// the lorri binary for tests that need to invoke lorri internal generate-env_.
// The test binary is a static CGO_ENABLED=0 build so it can be copied into
// the Nix store by the sandbox.
func lorriBinForTest(t *testing.T) string {
	t.Helper()
	bin, err := os.Executable()
	if err != nil {
		t.Fatalf("lorriBinForTest: %v", err)
	}
	return bin
}
