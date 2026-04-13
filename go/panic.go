package main

// Crash reporting and semantic exit codes.
//
// Mirrors src/main.rs setup_panic() / render_backtrace() and
// src/ops/error.rs ExitError / ExitAs.

import (
	"fmt"
	"net/url"
	"os"
	"runtime"
	"runtime/debug"
	"strings"
)

// lorriVersion is set at build time via -ldflags "-X main.lorriVersion=<ver>".
// Falls back to the module pseudo-version from the build info.
var lorriVersion = ""

func lorriVersionString() string {
	if lorriVersion != "" {
		return lorriVersion
	}
	if info, ok := debug.ReadBuildInfo(); ok {
		return info.Main.Version
	}
	return "(unknown)"
}

// withPanicHandler runs f and, if f panics, prints a human-friendly crash
// report and exits with code 101 (programming-error exit code).
// Call as: os.Exit(withPanicHandler(func() int { ... }))
func withPanicHandler(f func() int) (exitCode int) {
	if os.Getenv("LORRI_NO_INSTALL_PANIC_HANDLER") != "" ||
		os.Getenv("LORRI_DEBUG_PANIC") != "" {
		return f()
	}

	defer func() {
		r := recover()
		if r == nil {
			return
		}

		// Collect stack trace.
		buf := make([]byte, 64*1024)
		n := runtime.Stack(buf, false)
		stack := string(buf[:n])

		msg := fmt.Sprintf("%v", r)
		osInfo := fmt.Sprintf("%s/%s", runtime.GOOS, runtime.GOARCH)
		ver := lorriVersionString()

		issueBody := fmt.Sprintf(`## Crash report

**OS**: %s
**lorri version**: %s

### Panic message

%s

### Stack trace

`+"```"+`
%s
`+"```"+`
`, osInfo, ver, msg, stack)

		title := fmt.Sprintf("lorri crashed: %s", truncate(msg, 80))

		issueURL := buildIssueURL(title, issueBody)

		fmt.Fprintf(os.Stderr, `
lorri crashed! This is a bug.

%s

Please file an issue at the URL above (or paste the body below if the URL is too long).

OS:      %s
version: %s
cause:   %s

Stack trace:
%s
`, issueURL, osInfo, ver, msg, stack)

		exitCode = ExitCodePanic
	}()

	return f()
}

// buildIssueURL constructs a GitHub new-issue URL pre-filled with title and body.
// If the resulting URL exceeds 2000 bytes, returns a title-only URL with a
// note to paste the body manually. Mirrors src/main.rs render_backtrace URL logic.
func buildIssueURL(title, body string) string {
	const base = "https://github.com/nix-community/lorri/issues/new"
	const maxURLLen = 2000

	full := base + "?title=" + url.QueryEscape(title) + "&body=" + url.QueryEscape(body)
	if len(full) <= maxURLLen {
		return full
	}
	// Too long: return title-only URL, user must paste body.
	short := base + "?title=" + url.QueryEscape(title)
	return short + "\n(The body is too long for a URL — please paste it manually.)"
}

func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	return s[:max-3] + "..."
}

// ---------------------------------------------------------------------------
// ExitError — semantic exit codes (mirrors src/ops/error.rs ExitError)
// ---------------------------------------------------------------------------

// Exit code constants matching the execline convention used by Rust lorri.
// See src/ops/error.rs.
const (
	ExitCodeExpected    = 1   // expected / transient error
	ExitCodeUserError   = 100 // permanent user-configuration error
	ExitCodePanic       = 101 // programming error / unexpected crash
	ExitCodeTemporary   = 111 // transient, retriable (disk full, OOM, I/O)
	ExitCodeEnvironment = 126 // environment not set up correctly
	ExitCodeMissing     = 127 // required executable not found on $PATH
)

// ExitError carries a semantic exit code and a message.
// Wrapping an ExitError in fmt.Errorf is fine; errors.As will unwrap it.
type ExitError struct {
	Code int
	Msg  string
	// Cause is the underlying error, if any.
	Cause error
}

func (e *ExitError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("%s: %v", e.Msg, e.Cause)
	}
	return e.Msg
}

func (e *ExitError) Unwrap() error { return e.Cause }

// exitExpected wraps err as an expected / recoverable error (exit 1).
func exitExpected(msg string, cause error) *ExitError {
	return &ExitError{Code: ExitCodeExpected, Msg: msg, Cause: cause}
}

// exitUserError wraps err as a permanent user-configuration error (exit 100).
// Use when the same invocation will always fail (wrong flags, missing file, etc.).
func exitUserError(msg string, cause error) *ExitError {
	return &ExitError{Code: ExitCodeUserError, Msg: msg, Cause: cause}
}

// exitTemporary wraps err as a transient error that may succeed on retry (exit 111).
func exitTemporary(msg string, cause error) *ExitError {
	return &ExitError{Code: ExitCodeTemporary, Msg: msg, Cause: cause}
}

// exitEnvironment wraps err as an environment-setup error (exit 126).
func exitEnvironment(msg string, cause error) *ExitError {
	return &ExitError{Code: ExitCodeEnvironment, Msg: msg, Cause: cause}
}

// exitMissing wraps err as a missing-executable error (exit 127).
func exitMissing(msg string, cause error) *ExitError {
	return &ExitError{Code: ExitCodeMissing, Msg: msg, Cause: cause}
}

// isMissingExec returns true if err represents an "executable not found" error
// (exec.ErrNotFound or a PATH-search failure), suitable for exitMissing.
func isMissingExec(err error) bool {
	if err == nil {
		return false
	}
	return strings.Contains(err.Error(), "executable file not found") ||
		strings.Contains(err.Error(), "no such file or directory")
}
