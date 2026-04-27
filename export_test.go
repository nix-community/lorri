package main

// export_test.go: tests for `lorri export <shell>`.
//
// Two levels:
//
//  1. TestExportOutputBash — in-process, no subprocess. Constructs a
//     synthetic LorriEnv, applies changes against a fake ambient env,
//     and asserts the bash export/unset syntax is correct.
//
//  2. TestExport<Shell> — end-to-end per shell. Builds a synthetic env.json,
//     registers the project in a temp SQLite DB, creates a fake GC root, then
//     runs a single shell script that:
//       a. Evals the output of `lorri export <shell>` (enter)
//       b. Asserts variables are set correctly
//       c. Changes PWD to a directory outside any registered project
//       d. Evals the output of `lorri export <shell>` again (leave)
//       e. Asserts variables are reverted to their original values
//
// The end-to-end tests use the test binary itself as the lorri binary
// (via lorriBinForTest). TestMain detects this and routes to run() instead
// of running the test suite, preventing recursive test execution.

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	direnv "github.com/nix-community/lorri/direnv_vendor"
)

// TestMain allows the test binary to double as a lorri binary for integration
// tests. When LORRI_TEST_SUBPROCESS=1 is set, the binary routes os.Args
// through lorri's run() instead of executing the test suite. This is the
// canonical Go pattern for helper subprocesses (used by the stdlib's
// os/exec tests).
func TestMain(m *testing.M) {
	// Route to lorri's run() in two cases:
	//   1. LORRI_TEST_SUBPROCESS=1 — set by export integration tests that
	//      invoke the test binary as a lorri subprocess from a shell script.
	//   2. First argument is "internal" — the Nix keep-env-hack builder calls
	//      "$lorriBin internal generate-env_ $varmap" directly from the build
	//      sandbox without any wrapper env var.
	isSubprocess := os.Getenv("LORRI_TEST_SUBPROCESS") == "1"
	isInternal := len(os.Args) > 1 && os.Args[1] == "internal"
	if isSubprocess || isInternal {
		if err := run(os.Args[1:]); err != nil {
			var exitErr *ExitError
			if errors.As(err, &exitErr) {
				fmt.Fprintf(os.Stderr, "lorri: %v\n", exitErr)
				os.Exit(exitErr.Code)
			}
			fmt.Fprintf(os.Stderr, "lorri: %v\n", err)
			os.Exit(1)
		}
		os.Exit(0)
	}
	os.Exit(m.Run())
}

// ---------------------------------------------------------------------------
// Level 2: in-process bash output test
// ---------------------------------------------------------------------------

func TestExportOutputBash(t *testing.T) {
	changes := []EnvChange{
		{Op: "set", Name: "MARKER", Value: "hello"},
		{Op: "unset", Name: "UNSET_MARKER"},
		{Op: "pass", Name: "PASS_MARKER", Value: "nix-set-value"},
		{Op: "prepend", Name: "PATH", Value: "/nix/store/abc/bin", Sep: ":"},
		{Op: "append", Name: "GOPATH", Value: "/nix-gopath", Sep: ":"},
	}

	ambient := map[string]string{
		"UNSET_MARKER": "original-value",
		"PASS_MARKER":  "ambient-value",
		"PATH":         "/usr/bin",
		"GOPATH":       "ambient-gopath",
	}

	export := make(direnv.ShellExport)
	for _, change := range changes {
		applyChange(change, ambient, export)
	}

	out, err := direnv.Shells["bash"].Export(export)
	if err != nil {
		t.Fatalf("Export: %v", err)
	}

	cases := []struct {
		desc    string
		present bool
		substr  string
	}{
		{"MARKER set", true, "MARKER="},
		{"UNSET_MARKER unset", true, "unset UNSET_MARKER"},
		{"PASS_MARKER not touched", false, "PASS_MARKER="},
		{"PATH prepended", true, "/nix/store/abc/bin:/usr/bin"},
		{"GOPATH appended", true, "ambient-gopath:/nix-gopath"},
		// Should not export lorri session vars yet (no session in this test)
		{"no LORRI_PROJECT leak", false, "LORRI_PROJECT="},
	}

	for _, tc := range cases {
		t.Run(tc.desc, func(t *testing.T) {
			contains := strings.Contains(out, tc.substr)
			if tc.present && !contains {
				t.Errorf("expected %q in output\ngot: %s", tc.substr, out)
			}
			if !tc.present && contains {
				t.Errorf("did not expect %q in output\ngot: %s", tc.substr, out)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// Level 3: end-to-end per-shell tests
// ---------------------------------------------------------------------------

// shellTestCase describes a shell-specific test configuration.
type shellTestCase struct {
	// name is the lorri shell name (passed to `lorri export <name>`).
	name string
	// bin is the shell binary name on PATH.
	bin string
	// flags are extra flags to pass to the shell when running the script.
	flags []string
	// script returns the full test script given the project dir, lorri bin,
	// and the pre-lorri HOME value (which should be restored on leave).
	script func(projectDir, lorriBin, origHome string) string
}

var shellTestCases = []shellTestCase{
	{
		name:  "bash",
		bin:   "bash",
		flags: []string{"-e"},
		script: func(projectDir, lorriBin, origHome string) string {
			return fmt.Sprintf(`
set -euo pipefail

LORRI=%q
PROJECT_DIR=%q

# ── Enter ──────────────────────────────────────────────────────────────────
cd "$PROJECT_DIR"
eval "$("$LORRI" export bash)"

[[ "$MARKER" == "hello" ]]                              || { echo "ENTER: MARKER wrong: $MARKER"; exit 1; }
[[ -z "${UNSET_MARKER+x}" ]]                            || { echo "ENTER: UNSET_MARKER should be unset, got: $UNSET_MARKER"; exit 1; }
[[ "$PASS_MARKER" == "ambient-value" ]]                 || { echo "ENTER: PASS_MARKER wrong: $PASS_MARKER"; exit 1; }
[[ "$PKG_CONFIG_PATH" == "/nix/store/abc/lib/pkgconfig:ambient-pkgconfig" ]] \
                                                        || { echo "ENTER: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; }
[[ "$GOPATH" == "ambient-gopath:/nix-gopath" ]]         || { echo "ENTER: GOPATH wrong: $GOPATH"; exit 1; }
[[ -n "$LORRI_SESSION_ID" ]]                            || { echo "ENTER: LORRI_SESSION_ID not set"; exit 1; }
[[ "$LORRI_PROJECT" == "$PROJECT_DIR/shell.nix" ]]      || { echo "ENTER: LORRI_PROJECT wrong: $LORRI_PROJECT"; exit 1; }

# ── Leave ──────────────────────────────────────────────────────────────────
cd /tmp
eval "$("$LORRI" export bash)"

[[ -z "${MARKER+x}" ]]                                  || { echo "LEAVE: MARKER should be unset, got: $MARKER"; exit 1; }
[[ "$UNSET_MARKER" == "original-value" ]]               || { echo "LEAVE: UNSET_MARKER wrong: $UNSET_MARKER"; exit 1; }
[[ "$PASS_MARKER" == "ambient-value" ]]                 || { echo "LEAVE: PASS_MARKER wrong: $PASS_MARKER"; exit 1; }
[[ "$PKG_CONFIG_PATH" == "ambient-pkgconfig" ]]         || { echo "LEAVE: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; }
[[ "$GOPATH" == "ambient-gopath" ]]                     || { echo "LEAVE: GOPATH wrong: $GOPATH"; exit 1; }
[[ -z "${LORRI_PROJECT+x}" ]]                           || { echo "LEAVE: LORRI_PROJECT should be unset"; exit 1; }
`, lorriBin, projectDir)
		},
	},
	{
		name:  "zsh",
		bin:   "zsh",
		flags: []string{"-e"},
		script: func(projectDir, lorriBin, origHome string) string {
			return fmt.Sprintf(`
set -euo pipefail

LORRI=%q
PROJECT_DIR=%q

# ── Enter ──────────────────────────────────────────────────────────────────
cd "$PROJECT_DIR"
eval "$("$LORRI" export zsh)"

[[ "$MARKER" == "hello" ]]                              || { echo "ENTER: MARKER wrong: $MARKER"; exit 1; }
[[ -z "${UNSET_MARKER+x}" ]]                            || { echo "ENTER: UNSET_MARKER should be unset, got: $UNSET_MARKER"; exit 1; }
[[ "$PASS_MARKER" == "ambient-value" ]]                 || { echo "ENTER: PASS_MARKER wrong: $PASS_MARKER"; exit 1; }
[[ "$PKG_CONFIG_PATH" == "/nix/store/abc/lib/pkgconfig:ambient-pkgconfig" ]] \
                                                        || { echo "ENTER: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; }
[[ "$GOPATH" == "ambient-gopath:/nix-gopath" ]]         || { echo "ENTER: GOPATH wrong: $GOPATH"; exit 1; }
[[ -n "$LORRI_SESSION_ID" ]]                            || { echo "ENTER: LORRI_SESSION_ID not set"; exit 1; }

# ── Leave ──────────────────────────────────────────────────────────────────
cd /tmp
eval "$("$LORRI" export zsh)"

[[ -z "${MARKER+x}" ]]                                  || { echo "LEAVE: MARKER should be unset, got: $MARKER"; exit 1; }
[[ "$UNSET_MARKER" == "original-value" ]]               || { echo "LEAVE: UNSET_MARKER wrong: $UNSET_MARKER"; exit 1; }
[[ "$PASS_MARKER" == "ambient-value" ]]                 || { echo "LEAVE: PASS_MARKER wrong: $PASS_MARKER"; exit 1; }
[[ "$PKG_CONFIG_PATH" == "ambient-pkgconfig" ]]         || { echo "LEAVE: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; }
[[ "$GOPATH" == "ambient-gopath" ]]                     || { echo "LEAVE: GOPATH wrong: $GOPATH"; exit 1; }
`, lorriBin, projectDir)
		},
	},
	{
		name:  "fish",
		bin:   "fish",
		flags: []string{},
		script: func(projectDir, lorriBin, origHome string) string {
			return fmt.Sprintf(`
set LORRI %q
set PROJECT_DIR %q

# ── Enter ──────────────────────────────────────────────────────────────────
cd $PROJECT_DIR
eval ($LORRI export fish)

test "$MARKER" = "hello"
    or begin; echo "ENTER: MARKER wrong: $MARKER"; exit 1; end
not set -q UNSET_MARKER
    or begin; echo "ENTER: UNSET_MARKER should be unset, got: $UNSET_MARKER"; exit 1; end
test "$PASS_MARKER" = "ambient-value"
    or begin; echo "ENTER: PASS_MARKER wrong: $PASS_MARKER"; exit 1; end
test "$PKG_CONFIG_PATH" = "/nix/store/abc/lib/pkgconfig:ambient-pkgconfig"
    or begin; echo "ENTER: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; end
test "$GOPATH" = "ambient-gopath:/nix-gopath"
    or begin; echo "ENTER: GOPATH wrong: $GOPATH"; exit 1; end
set -q LORRI_SESSION_ID
    or begin; echo "ENTER: LORRI_SESSION_ID not set"; exit 1; end

# ── Leave ──────────────────────────────────────────────────────────────────
cd /tmp
eval ($LORRI export fish)

not set -q MARKER
    or begin; echo "LEAVE: MARKER should be unset, got: $MARKER"; exit 1; end
test "$UNSET_MARKER" = "original-value"
    or begin; echo "LEAVE: UNSET_MARKER wrong: $UNSET_MARKER"; exit 1; end
test "$PASS_MARKER" = "ambient-value"
    or begin; echo "LEAVE: PASS_MARKER wrong: $PASS_MARKER"; exit 1; end
test "$PKG_CONFIG_PATH" = "ambient-pkgconfig"
    or begin; echo "LEAVE: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1; end
test "$GOPATH" = "ambient-gopath"
    or begin; echo "LEAVE: GOPATH wrong: $GOPATH"; exit 1; end
`, lorriBin, projectDir)
		},
	},
	{
		name:  "elvish",
		bin:   "elvish",
		flags: []string{},
		script: func(projectDir, lorriBin, origHome string) string {
			// elvish's Export outputs JSON (not elvish commands); apply it the
			// same way the hook does: parse with from-json, call set-env/unset-env.
			return fmt.Sprintf(`
var LORRI = %q
var PROJECT_DIR = %q

fn lorri-apply {
  var m = [($LORRI export elvish | from-json)]
  if (> (count $m) 0) {
    set m = (all $m)
    keys $m | each { |k|
      if $m[$k] {
        set-env $k $m[$k]
      } else {
        unset-env $k
      }
    }
  }
}

# ── Enter ──────────────────────────────────────────────────────────────────
cd $PROJECT_DIR
lorri-apply

if (not-eq $E:MARKER hello) {
  echo "ENTER: MARKER wrong: "$E:MARKER; exit 1
}
if (has-env UNSET_MARKER) {
  echo "ENTER: UNSET_MARKER should be unset, got: "$E:UNSET_MARKER; exit 1
}
if (not-eq $E:PASS_MARKER ambient-value) {
  echo "ENTER: PASS_MARKER wrong: "$E:PASS_MARKER; exit 1
}
if (not-eq $E:PKG_CONFIG_PATH /nix/store/abc/lib/pkgconfig:ambient-pkgconfig) {
  echo "ENTER: PKG_CONFIG_PATH wrong: "$E:PKG_CONFIG_PATH; exit 1
}
if (not-eq $E:GOPATH ambient-gopath:/nix-gopath) {
  echo "ENTER: GOPATH wrong: "$E:GOPATH; exit 1
}
if (not (has-env LORRI_SESSION_ID)) {
  echo "ENTER: LORRI_SESSION_ID not set"; exit 1
}

# ── Leave ──────────────────────────────────────────────────────────────────
cd /tmp
lorri-apply

if (has-env MARKER) {
  echo "LEAVE: MARKER should be unset, got: "$E:MARKER; exit 1
}
if (not-eq $E:UNSET_MARKER original-value) {
  echo "LEAVE: UNSET_MARKER wrong: "$E:UNSET_MARKER; exit 1
}
if (not-eq $E:PASS_MARKER ambient-value) {
  echo "LEAVE: PASS_MARKER wrong: "$E:PASS_MARKER; exit 1
}
if (not-eq $E:PKG_CONFIG_PATH ambient-pkgconfig) {
  echo "LEAVE: PKG_CONFIG_PATH wrong: "$E:PKG_CONFIG_PATH; exit 1
}
if (not-eq $E:GOPATH ambient-gopath) {
  echo "LEAVE: GOPATH wrong: "$E:GOPATH; exit 1
}
`, lorriBin, projectDir)
		},
	},
	{
		name:  "tcsh",
		bin:   "tcsh",
		flags: []string{},
		script: func(projectDir, lorriBin, origHome string) string {
			return fmt.Sprintf(`
set LORRI = %q
set PROJECT_DIR = %q

cd "$PROJECT_DIR"
eval `+"`"+`$LORRI export tcsh`+"`"+`

if ("$MARKER" != "hello") then
  echo "ENTER: MARKER wrong: $MARKER"; exit 1
endif
if ($?UNSET_MARKER) then
  echo "ENTER: UNSET_MARKER should be unset, got: $UNSET_MARKER"; exit 1
endif
if ("$PASS_MARKER" != "ambient-value") then
  echo "ENTER: PASS_MARKER wrong: $PASS_MARKER"; exit 1
endif
if ("$PKG_CONFIG_PATH" != "/nix/store/abc/lib/pkgconfig:ambient-pkgconfig") then
  echo "ENTER: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1
endif
if ("$GOPATH" != "ambient-gopath:/nix-gopath") then
  echo "ENTER: GOPATH wrong: $GOPATH"; exit 1
endif
if (! $?LORRI_SESSION_ID) then
  echo "ENTER: LORRI_SESSION_ID not set"; exit 1
endif

cd /tmp
eval `+"`"+`$LORRI export tcsh`+"`"+`

if ($?MARKER) then
  echo "LEAVE: MARKER should be unset, got: $MARKER"; exit 1
endif
if ("$UNSET_MARKER" != "original-value") then
  echo "LEAVE: UNSET_MARKER wrong: $UNSET_MARKER"; exit 1
endif
if ("$PASS_MARKER" != "ambient-value") then
  echo "LEAVE: PASS_MARKER wrong: $PASS_MARKER"; exit 1
endif
if ("$PKG_CONFIG_PATH" != "ambient-pkgconfig") then
  echo "LEAVE: PKG_CONFIG_PATH wrong: $PKG_CONFIG_PATH"; exit 1
endif
if ("$GOPATH" != "ambient-gopath") then
  echo "LEAVE: GOPATH wrong: $GOPATH"; exit 1
endif
`, lorriBin, projectDir)
		},
	},
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

// buildEnvJSON writes a LorriEnv as env.json into dir and returns dir.
func buildEnvJSON(t *testing.T, dir string, changes []EnvChange) string {
	t.Helper()
	env := LorriEnv{Version: 1, Env: changes}
	data, err := json.MarshalIndent(env, "", "  ")
	if err != nil {
		t.Fatalf("buildEnvJSON: marshal: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dir, "env.json"), data, 0o644); err != nil {
		t.Fatalf("buildEnvJSON: write: %v", err)
	}
	return dir
}

// skipIfNoShell skips the test if the given shell binary is not on PATH.
func skipIfNoShell(t *testing.T, bin string) {
	t.Helper()
	if _, err := exec.LookPath(bin); err != nil {
		t.Skipf("%s not on PATH", bin)
	}
}

// runExportIntegration runs the full enter/leave integration test for one shell.
func runExportIntegration(t *testing.T, tc shellTestCase) {
	t.Helper()
	skipIfNoShell(t, tc.bin)

	lorriBin := lorriBinForTest(t)
	tmpDir, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatalf("EvalSymlinks tmpDir: %v", err)
	}

	// ── Build env.json ────────────────────────────────────────────────────
	// The env.json lives directly in tmpDir; the GC root symlink will point here.
	buildEnvJSON(t, tmpDir, []EnvChange{
		{Op: "set", Name: "MARKER", Value: "hello"},
		{Op: "unset", Name: "UNSET_MARKER"},
		{Op: "pass", Name: "PASS_MARKER", Value: "nix-set-value"},
		{Op: "prepend", Name: "PKG_CONFIG_PATH", Value: "/nix/store/abc/lib/pkgconfig", Sep: ":"},
		{Op: "append", Name: "GOPATH", Value: "/nix-gopath", Sep: ":"},
	})

	// ── Register project in DB ────────────────────────────────────────────
	// The project dir contains shell.nix so findProjectForDir matches it.
	projectDir := filepath.Join(tmpDir, "project")
	if err := os.MkdirAll(projectDir, 0o755); err != nil {
		t.Fatalf("mkdir project: %v", err)
	}
	nixFile := filepath.Join(projectDir, "shell.nix")
	// Write a minimal shell.nix (content doesn't matter; only path matters).
	if err := os.WriteFile(nixFile, []byte("{}"), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}

	// The lorri export subprocess will use XDG_CACHE_HOME=tmpDir, so its DB
	// lives at tmpDir/lorri/lorri.sqlite. Pre-populate it.
	cacheDir := filepath.Join(tmpDir, "lorri")
	if err := os.MkdirAll(cacheDir, 0o755); err != nil {
		t.Fatalf("mkdir cache: %v", err)
	}
	db, err := OpenLorriDB(filepath.Join(cacheDir, "lorri.sqlite"))
	if err != nil {
		t.Fatalf("OpenLorriDB: %v", err)
	}
	if err := db.UpsertProject(nixFile, false, ""); err != nil {
		db.Close()
		t.Fatalf("UpsertProject: %v", err)
	}
	db.Close()

	// ── Create GC root symlink ─────────────────────────────────────────────
	// gcRootPathForProject needs an AbsPath gcRootDir and the project file.
	gcRootDir := mustAbsPath(filepath.Join(cacheDir, "gc_roots"))
	if err := os.MkdirAll(string(gcRootDir), 0o755); err != nil {
		t.Fatalf("mkdir gc_roots: %v", err)
	}
	projectFile := NewShellNixProjectFile(mustAbsPath(nixFile))
	gcRootLink := gcRootPathForProject(gcRootDir, projectFile).String()
	if err := os.MkdirAll(filepath.Dir(gcRootLink), 0o755); err != nil {
		t.Fatalf("mkdir gc root parent: %v", err)
	}
	// Use a plain symlink instead of nix-store --add-root: the target is a
	// temp dir (not a real Nix store path) and we only need lorri export to
	// be able to read env.json from it.
	if err := os.Symlink(tmpDir, gcRootLink); err != nil {
		t.Fatalf("symlink gc root: %v", err)
	}

	// HOME is set to a real writable subdir so shells that try to write
	// history or config files (e.g. fish) don't emit spurious warnings.
	// This is the value lorri export should restore on leave.
	fakeHome := filepath.Join(tmpDir, "home")
	if err := os.MkdirAll(fakeHome, 0o755); err != nil {
		t.Fatalf("mkdir fake home: %v", err)
	}

	// ── Write script to a temp file ───────────────────────────────────────
	script := tc.script(projectDir, lorriBin, fakeHome)
	scriptFile := filepath.Join(tmpDir, "test-script")
	if err := os.WriteFile(scriptFile, []byte(script), 0o755); err != nil {
		t.Fatalf("write script: %v", err)
	}

	// ── Run the script ─────────────────────────────────────────────────────
	shellBin, err := exec.LookPath(tc.bin)
	if err != nil {
		t.Fatalf("LookPath %s: %v", tc.bin, err)
	}
	args := append(tc.flags, scriptFile)
	cmd := exec.Command(shellBin, args...)

	cmd.Env = []string{
		"HOME=" + fakeHome,
		"GOPATH=ambient-gopath",
		"PKG_CONFIG_PATH=ambient-pkgconfig",
		"UNSET_MARKER=original-value",
		"PASS_MARKER=ambient-value",
		"PATH=" + os.Getenv("PATH"),
		"XDG_CACHE_HOME=" + tmpDir,
		// Causes the test binary to route through lorri's run() instead of
		// executing the test suite when invoked as a subprocess.
		"LORRI_TEST_SUBPROCESS=1",
		// No LORRI_SESSION_ID — new session.
	}

	cmd.Stdin = strings.NewReader("") // prevent shell from reading test process stdin
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("%s script failed: %v\noutput:\n%s", tc.name, err, out)
	}
	if len(out) > 0 {
		t.Logf("%s output:\n%s", tc.name, out)
	}
}

// ---------------------------------------------------------------------------
// Per-shell test functions
// ---------------------------------------------------------------------------

func TestExportBash(t *testing.T) {
	runExportIntegration(t, shellTestCases[0])
}

func TestExportZsh(t *testing.T) {
	runExportIntegration(t, shellTestCases[1])
}

func TestExportFish(t *testing.T) {
	runExportIntegration(t, shellTestCases[2])
}

func TestExportElvish(t *testing.T) {
	runExportIntegration(t, shellTestCases[3])
}

func TestExportTcsh(t *testing.T) {
	runExportIntegration(t, shellTestCases[4])
}
