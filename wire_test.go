package main

// Pure unit tests — no Nix, no daemon, no filesystem required.
// Covers wire-format correctness, exit-code infrastructure, and opDirenv output.

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Reason round-trips
// ---------------------------------------------------------------------------

func TestReasonPingReceivedMarshal(t *testing.T) {
	r := reasonPingReceived()

	b, err := json.Marshal(r)
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}
	if string(b) != `"PingReceived"` {
		t.Errorf("want %q, got %q", `"PingReceived"`, string(b))
	}
}

func TestReasonPingReceivedUnmarshal(t *testing.T) {
	var r Reason
	if err := json.Unmarshal([]byte(`"PingReceived"`), &r); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}
	if !r.IsPingReceived() {
		t.Error("IsPingReceived() should be true")
	}
	if files := r.FilesChanged(); files != nil {
		t.Errorf("FilesChanged() should be nil, got %v", files)
	}
}

func TestReasonFilesChangedMarshal(t *testing.T) {
	paths := []string{"/a/b.nix", "/c/d.nix"}
	r := reasonFilesChanged(paths)

	b, err := json.Marshal(r)
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}

	// Must be {"FilesChanged": [...]}
	var wrapper struct {
		FilesChanged []string `json:"FilesChanged"`
	}
	if err := json.Unmarshal(b, &wrapper); err != nil {
		t.Fatalf("Unmarshal wrapper: %v", err)
	}
	if len(wrapper.FilesChanged) != 2 || wrapper.FilesChanged[0] != "/a/b.nix" {
		t.Errorf("unexpected FilesChanged: %v", wrapper.FilesChanged)
	}
}

func TestReasonFilesChangedUnmarshal(t *testing.T) {
	raw := `{"FilesChanged":["/x/y.nix"]}`
	var r Reason
	if err := json.Unmarshal([]byte(raw), &r); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}
	if r.IsPingReceived() {
		t.Error("IsPingReceived() should be false")
	}
	files := r.FilesChanged()
	if len(files) != 1 || files[0] != "/x/y.nix" {
		t.Errorf("FilesChanged() = %v, want [\"/x/y.nix\"]", files)
	}
}

// ---------------------------------------------------------------------------
// buildEventToJSON wire format
// ---------------------------------------------------------------------------

func TestBuildEventToJSONStarted(t *testing.T) {
	ev := Event{
		Started: &EventStarted{
			NixFile: "/project/shell.nix",
			Reason:  reasonPingReceived(),
		},
	}
	b, err := buildEventToJSON(ev)
	if err != nil {
		t.Fatalf("buildEventToJSON: %v", err)
	}

	var out map[string]any
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}

	// Top-level key must be "Started".
	started, ok := out["Started"].(map[string]any)
	if !ok {
		t.Fatalf("expected top-level \"Started\" key, got keys: %v", keys(out))
	}
	if started["nix_file"] != "/project/shell.nix" {
		t.Errorf("nix_file = %q, want %q", started["nix_file"], "/project/shell.nix")
	}
	if _, ok := started["reason"]; !ok {
		t.Error("missing \"reason\" field")
	}
	// Must not have "Completed" or "Failure" at top level.
	for _, bad := range []string{"Completed", "Failure"} {
		if _, ok := out[bad]; ok {
			t.Errorf("unexpected top-level key %q", bad)
		}
	}
}

func TestBuildEventToJSONCompleted(t *testing.T) {
	ev := Event{
		Completed: &EventCompleted{
			NixFile: "/project/shell.nix",
			RootedOutputPaths: OutputPath{
				ShellGCRoot: "/home/user/.cache/lorri/gc_roots/abc/gc_root/shell_gc_root",
			},
		},
	}
	b, err := buildEventToJSON(ev)
	if err != nil {
		t.Fatalf("buildEventToJSON: %v", err)
	}

	var out map[string]any
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}

	completed, ok := out["Completed"].(map[string]any)
	if !ok {
		t.Fatalf("expected top-level \"Completed\" key, got keys: %v", keys(out))
	}
	if completed["nix_file"] != "/project/shell.nix" {
		t.Errorf("nix_file = %q", completed["nix_file"])
	}
	rop, ok := completed["rooted_output_paths"].(map[string]any)
	if !ok {
		t.Fatalf("rooted_output_paths missing or wrong type")
	}
	if rop["shell_gc_root"] != "/home/user/.cache/lorri/gc_roots/abc/gc_root/shell_gc_root" {
		t.Errorf("shell_gc_root = %q", rop["shell_gc_root"])
	}
}

func TestBuildEventToJSONFailure(t *testing.T) {
	raw, _ := json.Marshal(map[string]string{"message": "build failed"})
	ev := Event{
		Failure: &EventFailure{
			NixFile:    "/project/shell.nix",
			FailureRaw: raw,
		},
	}
	b, err := buildEventToJSON(ev)
	if err != nil {
		t.Fatalf("buildEventToJSON: %v", err)
	}

	var out map[string]any
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}

	failure, ok := out["Failure"].(map[string]any)
	if !ok {
		t.Fatalf("expected top-level \"Failure\" key, got keys: %v", keys(out))
	}
	if failure["nix_file"] != "/project/shell.nix" {
		t.Errorf("nix_file = %q", failure["nix_file"])
	}
	// The "failure" sub-field carries the message.
	failureSub, ok := failure["failure"].(map[string]any)
	if !ok {
		t.Fatalf("failure.failure missing or wrong type: %T %v", failure["failure"], failure["failure"])
	}
	if failureSub["message"] != "build failed" {
		t.Errorf("failure.message = %q", failureSub["message"])
	}
}

func TestBuildEventToJSONEmptyReturnsError(t *testing.T) {
	_, err := buildEventToJSON(Event{})
	if err == nil {
		t.Error("expected error for empty event")
	}
}

// ---------------------------------------------------------------------------
// wireTimestamp — GC JSON wire format
// ---------------------------------------------------------------------------

func TestWireTimestampFormat(t *testing.T) {
	ts := time.Unix(1700000000, 123456789)
	wt := toWireTimestamp(&ts)
	if wt == nil {
		t.Fatal("toWireTimestamp returned nil")
	}

	b, err := json.Marshal(wt)
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}

	var obj map[string]int64
	if err := json.Unmarshal(b, &obj); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}

	if obj["secs_since_epoch"] != 1700000000 {
		t.Errorf("secs_since_epoch = %d, want 1700000000", obj["secs_since_epoch"])
	}
	if obj["nanos_since_epoch"] != 123456789 {
		t.Errorf("nanos_since_epoch = %d, want 123456789", obj["nanos_since_epoch"])
	}
}

func TestWireTimestampNilIsNull(t *testing.T) {
	wt := toWireTimestamp(nil)
	if wt != nil {
		t.Errorf("expected nil wireTimestamp for nil time, got %v", wt)
	}

	// When embedded in a struct with omitempty/pointer, it marshals as null.
	type wrapper struct {
		TS *wireTimestamp `json:"timestamp"`
	}
	b, err := json.Marshal(wrapper{TS: wt})
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}
	if !bytes.Contains(b, []byte("null")) {
		t.Errorf("expected null in JSON, got %s", b)
	}
}

// ---------------------------------------------------------------------------
// Exit code / panic handler
// ---------------------------------------------------------------------------

func TestWithPanicHandlerNoOp(t *testing.T) {
	code := withPanicHandler(func() int { return 42 })
	if code != 42 {
		t.Errorf("expected 42, got %d", code)
	}
}

func TestWithPanicHandlerCatchesPanic(t *testing.T) {
	os.Unsetenv("LORRI_NO_INSTALL_PANIC_HANDLER") //nolint:errcheck
	os.Unsetenv("LORRI_DEBUG_PANIC")              //nolint:errcheck

	// Redirect os.Stderr to /dev/null so the crash report doesn't pollute
	// the test output. Restore it afterwards.
	devNull, err := os.Open(os.DevNull)
	if err != nil {
		t.Fatalf("open /dev/null: %v", err)
	}
	defer devNull.Close()
	origStderr := os.Stderr
	os.Stderr = devNull
	defer func() { os.Stderr = origStderr }()

	code := withPanicHandler(func() int {
		panic("test panic")
	})
	if code != ExitCodePanic {
		t.Errorf("expected ExitCodePanic (%d), got %d", ExitCodePanic, code)
	}
}

func TestWithPanicHandlerBypassEnv(t *testing.T) {
	t.Setenv("LORRI_NO_INSTALL_PANIC_HANDLER", "1")
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic to propagate when handler is disabled")
		}
	}()
	withPanicHandler(func() int {
		panic("should propagate")
	})
}

func TestBuildIssueURLShort(t *testing.T) {
	url := buildIssueURL("short title", "short body")
	if !strings.HasPrefix(url, "https://github.com/nix-community/lorri/issues/new?") {
		t.Errorf("unexpected URL prefix: %s", url)
	}
	if !strings.Contains(url, "title=") {
		t.Error("missing title= in URL")
	}
	if !strings.Contains(url, "body=") {
		t.Error("missing body= in URL")
	}
	if len(url) > 2000 {
		t.Errorf("URL too long: %d bytes", len(url))
	}
}

func TestBuildIssueURLTruncates(t *testing.T) {
	// Generate a body long enough to push the URL past 2000 bytes.
	body := strings.Repeat("x", 3000)
	url := buildIssueURL("title", body)
	if len(url) > 2000 {
		// The returned value may be title-only URL + paste note, which can exceed
		// 2000 if the note line is included — what matters is the URL part itself
		// doesn't include the full body.
		if strings.Contains(url, strings.Repeat("x", 100)) {
			t.Error("long body should have been dropped from the URL")
		}
	}
	if !strings.Contains(url, "paste it manually") {
		t.Error("expected paste-manually note in truncated URL output")
	}
}

func TestExitErrorError(t *testing.T) {
	e := &ExitError{Code: ExitCodeUserError, Msg: "bad flag"}
	if e.Error() != "bad flag" {
		t.Errorf("Error() = %q, want %q", e.Error(), "bad flag")
	}
}

func TestExitErrorWithCause(t *testing.T) {
	cause := fmt.Errorf("underlying")
	e := exitUserError("bad flag", cause)
	if !strings.Contains(e.Error(), "underlying") {
		t.Errorf("Error() should include cause: %q", e.Error())
	}
	if !errors.Is(e, cause) {
		t.Error("errors.Is should find cause through Unwrap")
	}
}

func TestExitErrorAsUnwrap(t *testing.T) {
	inner := exitTemporary("disk full", nil)
	wrapped := fmt.Errorf("context: %w", inner)

	var found *ExitError
	if !errors.As(wrapped, &found) {
		t.Fatal("errors.As did not find *ExitError through wrapping")
	}
	if found.Code != ExitCodeTemporary {
		t.Errorf("Code = %d, want %d", found.Code, ExitCodeTemporary)
	}
}

func TestExitCodeValues(t *testing.T) {
	// Verify the numeric values match the execline convention used by Rust.
	cases := []struct {
		name string
		got  int
		want int
	}{
		{"Expected", ExitCodeExpected, 1},
		{"UserError", ExitCodeUserError, 100},
		{"Panic", ExitCodePanic, 101},
		{"Temporary", ExitCodeTemporary, 111},
		{"Environment", ExitCodeEnvironment, 126},
		{"Missing", ExitCodeMissing, 127},
	}
	for _, tc := range cases {
		if tc.got != tc.want {
			t.Errorf("ExitCode%s = %d, want %d", tc.name, tc.got, tc.want)
		}
	}
}

// ---------------------------------------------------------------------------
// opDirenv output shape (no daemon required — ping will fail, which is fine)
// ---------------------------------------------------------------------------

func TestOpDirenvOutputShape(t *testing.T) {
	dir := t.TempDir()
	paths := &Paths{
		GCRootDir:        mustAbsPath(dir + "/gc_roots"),
		DaemonSocketFile: mustAbsPath(dir + "/daemon.socket"),
		SQLiteDB:         mustAbsPath(dir + "/lorri.sqlite"),
	}
	nixFile := mustAbsPath(dir + "/shell.nix")
	if err := os.WriteFile(string(nixFile), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}
	projectFile := NewShellNixProjectFile(nixFile)

	// We only check the stdout output, not stderr status messages.
	// The direnv version check will fail if direnv isn't installed; skip if so.
	var buf bytes.Buffer
	if err := opDirenv(&buf, paths, projectFile); err != nil {
		if strings.Contains(err.Error(), "direnv") {
			t.Skipf("direnv not available: %v", err)
		}
		t.Fatalf("opDirenv: %v", err)
	}

	out := buf.String()
	// Case 1 (no GC root yet): opDirenv emits only a watch_file line for the
	// daemon socket so direnv re-evaluates once a build completes.
	if !strings.Contains(out, "watch_file") {
		t.Errorf("output missing watch_file:\n%s", out)
	}
	// The daemon socket path must be watch_file'd.
	if !strings.Contains(out, dir+"/daemon.socket") {
		t.Errorf("output missing daemon socket path:\n%s", out)
	}
}

// callOpDirenv calls opDirenv and returns its stdout output and captured
// stderr. Skips the test if direnv is not available.
func callOpDirenv(t *testing.T, paths *Paths, projectFile ProjectFile) (out, stderr string) {
	t.Helper()
	oldStderr := os.Stderr
	r, w, err := os.Pipe()
	if err != nil {
		t.Fatalf("pipe: %v", err)
	}
	os.Stderr = w

	var buf bytes.Buffer
	opErr := opDirenv(&buf, paths, projectFile)

	w.Close()
	os.Stderr = oldStderr
	var stderrBuf bytes.Buffer
	stderrBuf.ReadFrom(r) //nolint:errcheck

	if opErr != nil {
		if strings.Contains(opErr.Error(), "direnv") {
			t.Skipf("direnv not available: %v", opErr)
		}
		t.Fatalf("opDirenv: %v", opErr)
	}
	return buf.String(), stderrBuf.String()
}

// TestOpDirenvMigration tests the full migration arc from an old-style Rust
// lorri direnv GC root to the new env.json-based one.
//
// Phase 1 — old-style GC root (bash-export, no env.json):
//   - stdout watches both socket and GC root (so direnv re-evals after rebuild)
//   - stdout has no export statements (stale bash-export must not be loaded)
//   - stderr mentions "not yet migrated"
//
// Phase 2 — rebuild completes, env.json appears in the same store path:
//   - stdout contains export statements derived from env.json
//   - stdout watches both socket and GC root
//   - MARKER is exported with the correct value
func TestOpDirenvMigration(t *testing.T) {
	dir := t.TempDir()
	paths := &Paths{
		GCRootDir:        mustAbsPath(dir + "/gc_roots"),
		DaemonSocketFile: mustAbsPath(dir + "/daemon.socket"),
		SQLiteDB:         mustAbsPath(dir + "/lorri.sqlite"),
	}
	nixFile := mustAbsPath(dir + "/shell.nix")
	if err := os.WriteFile(string(nixFile), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write shell.nix: %v", err)
	}
	projectFile := NewShellNixProjectFile(nixFile)

	// Synthesise an old-style GC root: a store-path directory containing
	// bash-export but not env.json — exactly what the Rust lorri produced.
	fakeStorePath := dir + "/fake-store-path"
	if err := os.MkdirAll(fakeStorePath, 0o755); err != nil {
		t.Fatalf("mkdir fake store path: %v", err)
	}
	if err := os.WriteFile(fakeStorePath+"/bash-export", []byte("export MARKER=present\n"), 0o644); err != nil {
		t.Fatalf("write bash-export: %v", err)
	}

	// Point the GC root symlink at the fake store path.
	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)
	if err := os.MkdirAll(string(gcRootPath.Dir()), 0o755); err != nil {
		t.Fatalf("mkdir gc root parent: %v", err)
	}
	if err := os.Symlink(fakeStorePath, gcRootPath.String()); err != nil {
		t.Fatalf("symlink gc root: %v", err)
	}

	// ── Phase 1: old-style root ───────────────────────────────────────────
	t.Run("old-style root", func(t *testing.T) {
		out, stderr := callOpDirenv(t, paths, projectFile)

		// Must watch both the socket and the GC root.
		if !strings.Contains(out, dir+"/daemon.socket") {
			t.Errorf("output missing daemon socket watch_file:\n%s", out)
		}
		if !strings.Contains(out, gcRootPath.String()) {
			t.Errorf("output missing gc root watch_file:\n%s", out)
		}
		// Must NOT emit any export statements.
		if strings.Contains(out, "export ") {
			t.Errorf("output should not contain export statements during migration:\n%s", out)
		}
		// Stderr must mention migration.
		if !strings.Contains(stderr, "not yet migrated") {
			t.Errorf("stderr missing migration message:\n%s", stderr)
		}
	})

	// ── Phase 2: rebuild completes, env.json lands ────────────────────────
	// Simulate the daemon completing a rebuild by writing env.json into the
	// same store path the GC root already points at.
	envJSON := `{"version":1,"env":[{"op":"set","name":"MARKER","value":"present"}]}`
	if err := os.WriteFile(fakeStorePath+"/env.json", []byte(envJSON), 0o644); err != nil {
		t.Fatalf("write env.json: %v", err)
	}

	t.Run("after rebuild", func(t *testing.T) {
		out, _ := callOpDirenv(t, paths, projectFile)

		// Must watch both socket and GC root.
		if !strings.Contains(out, dir+"/daemon.socket") {
			t.Errorf("output missing daemon socket watch_file:\n%s", out)
		}
		if !strings.Contains(out, gcRootPath.String()) {
			t.Errorf("output missing gc root watch_file:\n%s", out)
		}
		// Must export MARKER with the value from env.json.
		if !strings.Contains(out, "MARKER") {
			t.Errorf("output missing MARKER export:\n%s", out)
		}
		if !strings.Contains(out, "present") {
			t.Errorf("output missing MARKER value 'present':\n%s", out)
		}
	})
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

func keys(m map[string]any) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	return ks
}
