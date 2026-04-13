package main

// Ports of tests/integration/{envrc,direnv,trivial,bug*.rs} and
// tests/integration/test_op_gc.rs.
//
// Two test harnesses mirror the Rust originals:
//
//   runEnvrcExport  — EnvrcTestCase: no Nix build; writes a synthetic
//                     EVALUATION_ROOT directory and runs direnv against
//                     envrc.bash directly.
//
//   runDirenvExport — DirenvTestCase: real Nix build of a fixture shell.nix,
//                     writes .envrc via opDirenv, runs direnv export json.
//
// All tests that require a real Nix build are gated on RUN_TIME_CLOSURE.
// Tests that only need direnv (envrc.rs ports) are gated on direnv being on PATH.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

// direnvCmd returns a *exec.Cmd for `direnv <args>` with the same clean-env
// setup that Rust's direnv_cmd() uses.
// configDir is used for DIRENV_CONFIG and XDG_CONFIG_HOME so direnv doesn't
// read the user's real config.
// projectDir is set as the working directory.
// ambientEnv is layered on top of the clean env.
func direnvCmd(configDir, projectDir string, ambientEnv map[string]string, args ...string) *exec.Cmd {
	cmd := exec.Command("direnv", args...)
	cmd.Dir = projectDir

	// Start from a clean slate (mirrors Rust's d.env_remove / env_clear calls).
	// All XDG dirs are pointed at configDir so direnv writes state there
	// regardless of what HOME the caller provides.
	env := []string{
		"DIRENV_CONFIG=" + configDir,
		"XDG_CONFIG_HOME=" + configDir,
		"XDG_CACHE_HOME=" + configDir,
		"XDG_DATA_HOME=" + configDir,
		// Keep PATH so direnv and bash are findable.
		"PATH=" + os.Getenv("PATH"),
	}
	// Merge caller-supplied ambient vars. HOME may be fake (e.g. /home/alice
	// in the envrc tests), but direnv won't touch it because all XDG dirs are
	// already redirected to configDir above.
	for k, v := range ambientEnv {
		env = append(env, k+"="+v)
	}
	// Ensure HOME is always set to something writable as a final fallback so
	// bash sub-processes don't complain, unless the caller already set it.
	if _, ok := ambientEnv["HOME"]; !ok {
		env = append(env, "HOME="+configDir)
	}
	cmd.Env = env
	return cmd
}

// direnvExportJSON runs `direnv allow` then `direnv export json` in projectDir
// and returns the parsed result.
// configDir is used for DIRENV_CONFIG / XDG_CONFIG_HOME.
// ambientEnv is merged into the clean env before running direnv.
func direnvExportJSON(t *testing.T, configDir, projectDir string, ambientEnv map[string]string) map[string]any {
	t.Helper()

	allow := direnvCmd(configDir, projectDir, ambientEnv, "allow")
	if out, err := allow.CombinedOutput(); err != nil {
		t.Fatalf("direnv allow: %v\n%s", err, out)
	}

	export := direnvCmd(configDir, projectDir, ambientEnv, "export", "json")
	out, err := export.Output()
	if err != nil {
		t.Fatalf("direnv export json: %v\nstderr: %s", err, out)
	}

	// direnv outputs nothing (not even "{}") when no vars change — treat as empty.
	if len(bytes.TrimSpace(out)) == 0 {
		return map[string]any{}
	}

	var result map[string]any
	if err := json.Unmarshal(out, &result); err != nil {
		t.Fatalf("parse direnv export json: %v\noutput: %s", err, out)
	}
	return result
}

// direnvGet returns the string value of key in env, or "" if absent/null.
// Mirrors DirenvEnv::get_env returning Value(s).
func direnvGet(env map[string]any, key string) (string, bool) {
	v, ok := env[key]
	if !ok {
		return "", false // NotSet
	}
	if v == nil {
		return "", true // Unset (direnv sets to null to signal removal)
	}
	s, ok := v.(string)
	return s, ok
}

// skipIfNoDirenv skips the test if direnv is not on PATH.
func skipIfNoDirenv(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("direnv"); err != nil {
		t.Skip("direnv not on PATH")
	}
}

// ---------------------------------------------------------------------------
// EnvrcTestCase harness
// ---------------------------------------------------------------------------
//
// Mirrors tests/integration/envrctestcase.rs.
//
// Writes a synthetic EVALUATION_ROOT directory containing:
//   v1: a single "bash-export" file (output of `bash -c export` with given vars)
//   v2: "bash-export" + "varmap-v1" (NUL-separated append instructions)
//
// Then writes envrcBash as .envrc in a "test-root" subdirectory and runs
// direnv export json against it.

type envrcV1 struct {
	vars map[string]string // variables to set
}

type envrcV2 struct {
	set    map[string]string // variables to set
	append map[string]string // varname -> separator (for append entries)
}

// buildEvalRootV1 writes a v1 EVALUATION_ROOT directory and returns its path.
// Generates "bash-export" by running `bash -c export` with the given env vars.
func buildEvalRootV1(t *testing.T, dir string, v envrcV1) string {
	t.Helper()
	evalRoot := filepath.Join(dir, "eval-root-v1")
	if err := os.MkdirAll(evalRoot, 0o755); err != nil {
		t.Fatalf("mkdir eval-root-v1: %v", err)
	}

	bashExport := bashExportOutput(t, v.vars)
	if err := os.WriteFile(filepath.Join(evalRoot, "bash-export"), bashExport, 0o644); err != nil {
		t.Fatalf("write bash-export: %v", err)
	}
	return evalRoot
}

// buildEvalRootV2 writes a v2 EVALUATION_ROOT directory and returns its path.
func buildEvalRootV2(t *testing.T, dir string, v envrcV2) string {
	t.Helper()
	evalRoot := filepath.Join(dir, "eval-root-v2")
	if err := os.MkdirAll(evalRoot, 0o755); err != nil {
		t.Fatalf("mkdir eval-root-v2: %v", err)
	}

	bashExport := bashExportOutput(t, v.set)
	if err := os.WriteFile(filepath.Join(evalRoot, "bash-export"), bashExport, 0o644); err != nil {
		t.Fatalf("write bash-export: %v", err)
	}

	// varmap-v1: NUL-separated triples: instruction\0variable\0separator\0
	// Only "append" entries are written (matching Rust ProjectEnvBuilderV2).
	var varmap []byte
	for varname, sep := range v.append {
		varmap = append(varmap, []byte("append\x00"+varname+"\x00"+sep+"\x00")...)
	}
	if err := os.WriteFile(filepath.Join(evalRoot, "varmap-v1"), varmap, 0o644); err != nil {
		t.Fatalf("write varmap-v1: %v", err)
	}
	return evalRoot
}

// bashExportOutput runs `bash -c export` with exactly the given env vars
// and returns stdout. Mirrors ProjectEnvBuilderV1/V2::write_to.
func bashExportOutput(t *testing.T, vars map[string]string) []byte {
	t.Helper()
	bash, err := exec.LookPath("bash")
	if err != nil {
		t.Fatalf("bash not found: %v", err)
	}
	cmd := exec.Command(bash, "-c", "export")
	cmd.Env = []string{}
	for k, v := range vars {
		cmd.Env = append(cmd.Env, k+"="+v)
	}
	out, err := cmd.Output()
	if err != nil {
		t.Fatalf("bash -c export: %v", err)
	}
	return out
}

// runEnvrcExport mirrors EnvrcTestCase::get_direnv_variables.
// evalRoot is the path to the pre-built EVALUATION_ROOT directory.
// ambientEnv is merged into the clean direnv environment (on top of PATH/HOME).
func runEnvrcExport(t *testing.T, evalRoot string, ambientEnv map[string]string) map[string]any {
	t.Helper()
	skipIfNoDirenv(t)

	dir := t.TempDir()
	projectDir := filepath.Join(dir, "test-root")
	if err := os.MkdirAll(projectDir, 0o755); err != nil {
		t.Fatalf("mkdir test-root: %v", err)
	}

	// Write envrc.bash verbatim as .envrc — mirrors Rust's write_all(include_bytes!(...)).
	if err := os.WriteFile(filepath.Join(projectDir, ".envrc"), []byte(envrcBash), 0o644); err != nil {
		t.Fatalf("write .envrc: %v", err)
	}

	// Merge EVALUATION_ROOT into the ambient env for direnv.
	merged := make(map[string]string, len(ambientEnv)+1)
	for k, v := range ambientEnv {
		merged[k] = v
	}
	merged["EVALUATION_ROOT"] = evalRoot

	return direnvExportJSON(t, dir, projectDir, merged)
}

// ---------------------------------------------------------------------------
// DirenvTestCase harness
// ---------------------------------------------------------------------------
//
// Mirrors tests/integration/direnvtestcase.rs:
//   1. Build the fixture via InstantiateAndBuild.
//   2. Create a GC root.
//   3. Write .envrc via opDirenv into a fresh project directory.
//   4. Run direnv export json.

// runDirenvExport builds fixtureName's shell.nix, calls opDirenv to write
// .envrc, runs `direnv export json`, and returns the parsed env map and the
// GC-rooted build output path (mirrors DirenvTestCase returning both testcase
// and the OutputPath so callers can check res.exists()).
// ambientEnv is merged into the clean direnv environment.
func runDirenvExport(t *testing.T, fixtureName string, ambientEnv map[string]string) (map[string]any, BuildOutputPath) {
	t.Helper()
	rtc := rtcOrSkip(t)
	skipIfNoDirenv(t)

	nixFile := filepath.Join(lorriRoot(), "tests/integration", fixtureName, "shell.nix")
	if _, err := os.Stat(nixFile); err != nil {
		t.Fatalf("fixture missing: %s", nixFile)
	}

	dir := t.TempDir()

	cas, err := NewCAS(mustAbsPath(filepath.Join(dir, "cas")))
	if err != nil {
		t.Fatalf("NewCAS: %v", err)
	}

	result, err := InstantiateAndBuild(nixFile, cas, EmptyNixOptions(), rtc)
	if err != nil {
		t.Fatalf("InstantiateAndBuild(%s): %v", fixtureName, err)
	}
	defer result.Result.Release()

	gcRootDir := mustAbsPath(filepath.Join(dir, "gc_roots"))
	projectFile := NewShellNixProjectFile(mustAbsPath(nixFile))
	gcRootPath := gcRootPathForProject(gcRootDir, projectFile)

	if err := createGCRoot(result.Result.Path, gcRootPath.String()); err != nil {
		t.Fatalf("createGCRoot: %v", err)
	}

	paths := &Paths{
		GCRootDir:        gcRootDir,
		DaemonSocketFile: mustAbsPath(filepath.Join(dir, "daemon.socket")),
		CASDir:           mustAbsPath(filepath.Join(dir, "cas")),
		SQLiteDB:         mustAbsPath(filepath.Join(dir, "lorri.sqlite")),
	}

	// Write .envrc via opDirenv into a fresh project directory.
	projectDir := filepath.Join(dir, "project")
	if err := os.MkdirAll(projectDir, 0o755); err != nil {
		t.Fatalf("mkdir project: %v", err)
	}

	envrcFile, err := os.Create(filepath.Join(projectDir, ".envrc"))
	if err != nil {
		t.Fatalf("create .envrc: %v", err)
	}
	if err := opDirenv(envrcFile, paths, projectFile); err != nil {
		envrcFile.Close()
		// opDirenv may fail if direnv version check fails — surface clearly.
		t.Fatalf("opDirenv: %v", err)
	}
	envrcFile.Close()

	buildOut := BuildOutputPath{ShellGCRoot: gcRootPath.String()}
	return direnvExportJSON(t, dir, projectDir, ambientEnv), buildOut
}

// ---------------------------------------------------------------------------
// envrc.rs ports — no Nix build required
// ---------------------------------------------------------------------------

func TestEnvrcTrivialV1(t *testing.T) {
	dir := t.TempDir()
	evalRoot := buildEvalRootV1(t, dir, envrcV1{vars: map[string]string{
		"HOME":   "/homeless-shelter",
		"USER":   "nixbld1",
		"PATH":   "/foo/bar/path",
		"FOOBAR": "TUX",
		"GOPATH": "BAR",
	}})

	env := runEnvrcExport(t, evalRoot, map[string]string{
		"HOME":   "/home/alice",
		"USER":   "alice",
		"FOOBAR": "BAZ",
		"GOPATH": "FOO",
	})

	// HOME and USER are punt'd — not set.
	if _, ok := env["HOME"]; ok {
		t.Errorf("HOME should not be set by direnv, got %v", env["HOME"])
	}
	if _, ok := env["USER"]; ok {
		t.Errorf("USER should not be set by direnv, got %v", env["USER"])
	}

	// PATH: new value prepended to ambient PATH.
	path, _ := direnvGet(env, "PATH")
	ambientPath := os.Getenv("PATH")
	want := "/foo/bar/path:" + ambientPath
	if path != want {
		t.Errorf("PATH = %q, want %q", path, want)
	}

	// FOOBAR: set to nix value.
	if v, _ := direnvGet(env, "FOOBAR"); v != "TUX" {
		t.Errorf("FOOBAR = %q, want TUX", v)
	}

	// GOPATH: set to nix value (v1 has no append semantics).
	if v, _ := direnvGet(env, "GOPATH"); v != "BAR" {
		t.Errorf("GOPATH = %q, want BAR", v)
	}
}

func TestEnvrcTrivialV2(t *testing.T) {
	dir := t.TempDir()
	evalRoot := buildEvalRootV2(t, dir, envrcV2{
		set: map[string]string{
			"HOME":   "/homeless-shelter",
			"USER":   "nixbld1",
			"PATH":   "/foo/bar/path",
			"FOOBAR": "TUX",
			"GOPATH": "BAR",
		},
		append: map[string]string{
			"GOPATH": ":",
		},
	})

	env := runEnvrcExport(t, evalRoot, map[string]string{
		"HOME":   "/home/alice",
		"USER":   "alice",
		"FOOBAR": "BAZ",
		"GOPATH": "FOO",
	})

	if _, ok := env["HOME"]; ok {
		t.Errorf("HOME should not be set")
	}
	if _, ok := env["USER"]; ok {
		t.Errorf("USER should not be set")
	}

	path, _ := direnvGet(env, "PATH")
	ambientPath := os.Getenv("PATH")
	if want := "/foo/bar/path:" + ambientPath; path != want {
		t.Errorf("PATH = %q, want %q", path, want)
	}

	if v, _ := direnvGet(env, "FOOBAR"); v != "TUX" {
		t.Errorf("FOOBAR = %q, want TUX", v)
	}

	// v2: GOPATH appended to ambient FOO → FOO:BAR
	if v, _ := direnvGet(env, "GOPATH"); v != "FOO:BAR" {
		t.Errorf("GOPATH = %q, want FOO:BAR", v)
	}
}

func TestEnvrcV2GopathPreviouslyUnset(t *testing.T) {
	dir := t.TempDir()
	evalRoot := buildEvalRootV2(t, dir, envrcV2{
		set:    map[string]string{"GOPATH": "BAR"},
		append: map[string]string{"GOPATH": ":"},
	})

	// No GOPATH in ambient env.
	env := runEnvrcExport(t, evalRoot, map[string]string{
		"HOME": "/home/alice",
	})

	// When ambient is unset, result should be just the new value.
	if v, _ := direnvGet(env, "GOPATH"); v != "BAR" {
		t.Errorf("GOPATH = %q, want BAR", v)
	}
}

func TestEnvrcBug18PathWithSpaces(t *testing.T) {
	exemplar := "/mnt/c/Program Files (x86)/QuickTime/QTSystem"
	ambientPath := os.Getenv("PATH") + ":" + exemplar

	dir := t.TempDir()
	evalRoot := buildEvalRootV1(t, dir, envrcV1{vars: map[string]string{
		"PATH": "/foo/bar",
	}})

	env := runEnvrcExport(t, evalRoot, map[string]string{
		"HOME": "/home/alice",
		"PATH": ambientPath,
	})

	path, _ := direnvGet(env, "PATH")
	want := "/foo/bar:" + ambientPath
	if path != want {
		t.Errorf("PATH = %q, want %q", path, want)
	}
}

// ---------------------------------------------------------------------------
// trivial.rs + direnv.rs ports — real Nix build
// ---------------------------------------------------------------------------

func TestTrivialOldStyle(t *testing.T) {
	env, res := runDirenvExport(t, "basic", nil)

	// Mirrors Rust trivial_old_style: assert!(res.exists(), "no build output …")
	if !res.Exists() {
		t.Errorf("build output (shell_gc_root) does not exist: %s", res.ShellGCRoot)
	}

	if v, _ := direnvGet(env, "MARKER"); v != "present" {
		t.Errorf("MARKER = %q, want present", v)
	}
}

func TestInLorriShell(t *testing.T) {
	env, _ := runDirenvExport(t, "basic", nil)

	want := filepath.Join(lorriRoot(), "tests/integration/basic/shell.nix")
	if v, _ := direnvGet(env, "IN_LORRI_SHELL"); v != want {
		t.Errorf("IN_LORRI_SHELL = %q, want %q", v, want)
	}
}

// ---------------------------------------------------------------------------
// bug23_gopath.rs + bug23_setuphook.rs ports
// ---------------------------------------------------------------------------

func TestBug23Gopath(t *testing.T) {
	env, _ := runDirenvExport(t, "bug23_gopath", map[string]string{
		"GOPATH": "my-neat-go-path",
	})

	// The fixture appends /tmp/foo/bar to the ambient GOPATH.
	// Result: ambient comes first, appended value second.
	// Mirrors Rust: assert_eq!(env.get_env("GOPATH"), Value("my-neat-go-path:/tmp/foo/bar"))
	if v, _ := direnvGet(env, "GOPATH"); v != "my-neat-go-path:/tmp/foo/bar" {
		t.Errorf("GOPATH = %q, want my-neat-go-path:/tmp/foo/bar", v)
	}
}

func TestBug23SetupHook(t *testing.T) {
	env, _ := runDirenvExport(t, "bug23_setuphook", map[string]string{
		"EXAMPLE": "my-neat-path",
	})

	// Same pattern: ambient first, appended second.
	// Mirrors Rust: assert_eq!(env.get_env("EXAMPLE"), Value("my-neat-path:/tmp/foo/bar"))
	if v, _ := direnvGet(env, "EXAMPLE"); v != "my-neat-path:/tmp/foo/bar" {
		t.Errorf("EXAMPLE = %q, want my-neat-path:/tmp/foo/bar", v)
	}
}

// ---------------------------------------------------------------------------
// bug97_varmap_leak.rs port — strict "no extra vars" check
// ---------------------------------------------------------------------------

func TestBug97VarmapLeak(t *testing.T) {
	env, _ := runDirenvExport(t, "bug97_varmap_leak", nil)

	// preHook must be the literal string from the fixture.
	if v, _ := direnvGet(env, "preHook"); v != "echo 'foo bar'" {
		t.Errorf("preHook = %q, want \"echo 'foo bar'\"", v)
	}

	// These are the only variables allowed to appear.
	allowed := map[string]bool{
		// Scenario-specific
		"preHook": true,
		// Nix derivation variables
		"name": true, "builder": true, "out": true, "outputs": true,
		"stdenv": true, "system": true, "PATH": true,
		"extraClosure": true,
		// Lorri dependency capture
		"origBuilder": true, "origArgs": true, "origOutputs": true,
		"origSystem": true, "origPATH": true, "origExtraClosure": true,
		// Nix-set variables
		"IN_NIX_SHELL": true, "NIX_BUILD_CORES": true, "NIX_BUILD_TOP": true,
		"NIX_LOG_FD": true, "NIX_STORE": true,
		"allowSubstitutes": true, "preferLocalBuild": true,
		// Direnv state vars
		"DIRENV_DIFF": true, "DIRENV_DIR": true, "DIRENV_FILE": true,
		"DIRENV_WATCHES": true,
		// Lorri-set
		"IN_LORRI_SHELL": true,
		// See comment in Rust test: "unsure where it comes from but it fails on CI"
		"XDG_CONFIG_HOME": true,
	}

	var extra []string
	for k := range env {
		if !allowed[k] {
			extra = append(extra, fmt.Sprintf("%s=%v", k, env[k]))
		}
	}
	if len(extra) > 0 {
		t.Errorf("unexpected variables leaked into the environment:\n  %s", strings.Join(extra, "\n  "))
	}
}

// ---------------------------------------------------------------------------
// bug110_duplicate_appends.rs port — performance regression test
// ---------------------------------------------------------------------------

func TestBug110DuplicateAppends(t *testing.T) {
	start := time.Now()
	env, _ := runDirenvExport(t, "bug110_duplicate_appends", nil)
	elapsed := time.Since(start)

	if elapsed > 2*time.Second {
		t.Errorf("direnv export took %v, want < 2s (duplicate-appends perf regression)", elapsed)
	}

	v, _ := direnvGet(env, "ITWORKED")
	if !strings.HasSuffix(v, "foo/bar") {
		t.Errorf("ITWORKED = %q, want suffix foo/bar", v)
	}
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
		CASDir:           mustAbsPath(filepath.Join(dir, "cas")),
		SQLiteDB:         mustAbsPath(filepath.Join(dir, "lorri.sqlite")),
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
	cas, err := NewCAS(paths.CASDir)
	if err != nil {
		t.Fatalf("NewCAS: %v", err)
	}
	result, err := InstantiateAndBuild(nixFile, cas, EmptyNixOptions(), rtc)
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
	opts := GCRmOptions{}
	toRemove := gcFilterRoots(infos, opts)
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
	result2, err := InstantiateAndBuild(nixFile, cas, EmptyNixOptions(), rtc)
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
