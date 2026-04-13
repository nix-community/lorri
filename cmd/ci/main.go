// cmd/ci manages CI for lorri.
//
// Subcommands:
//
//	go run ./cmd/ci generate    # write .github/workflows/ci.yml
//	go run ./cmd/ci check       # exit non-zero if ci.yml is stale
//	go run ./cmd/ci test        # run the test suite in a clean environment
//
// The generate/check subcommands marshal the workflow config to JSON via
// encoding/json and convert it to YAML by piping through `yj -jy` (available
// in the lorri nix-shell, or via `nix run nixpkgs#yj`).
//
// The test subcommand resolves hermetic tool paths from the pinned nixpkgs
// (via NIX_PATH), clears the process environment, rebuilds it from those
// paths, then runs:
//
//	go test ./...
//	go run ./cmd/ci check
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

// repoRoot returns the repository root derived from this source file's
// location — works correctly with "go run ./cmd/ci".
func repoRoot() string {
	_, file, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(file), "..", "..")
}

// ── schema ───────────────────────────────────────────────────────────────────

type workflow struct {
	Name string            `json:"name"`
	On   workflowOn        `json:"on"`
	Env  map[string]string `json:"env"`
	Jobs map[string]job    `json:"jobs"`
}

type workflowOn struct {
	PullRequest      pushTrigger `json:"pull_request"`
	Push             pushTrigger `json:"push"`
	WorkflowDispatch struct{}    `json:"workflow_dispatch"`
}

type pushTrigger struct {
	Branches []string `json:"branches"`
}

type job struct {
	Name   string `json:"name"`
	RunsOn string `json:"runs-on"`
	Steps  []step `json:"steps"`
}

type step struct {
	Name string            `json:"name"`
	Uses string            `json:"uses,omitempty"`
	With map[string]string `json:"with,omitempty"`
	Run  string            `json:"run,omitempty"`
}

// ── reusable step fragments ──────────────────────────────────────────────────

var (
	stepCheckout = step{Name: "Checkout", Uses: "actions/checkout@v4"}
	stepNix      = step{Name: "Nix", Uses: "cachix/install-nix-action@v26"}
	stepCachix   = step{
		Name: "Cachix",
		Uses: "cachix/cachix-action@v14",
		With: map[string]string{
			"authToken": "${{ secrets.CACHIX_AUTH_TOKEN }}",
			"name":      "nix-community",
		},
	}
)

var commonSteps = []step{stepCheckout, stepNix, stepCachix}

func goTestSteps() []step {
	return append(commonSteps,
		step{
			Name: "Run CI tests",
			Run:  "nix develop --command go run ./cmd/ci test\n",
		},
	)
}

func stableSteps() []step {
	return append(commonSteps,
		step{Name: "Build", Run: "nix build\n"},
	)
}

// ── config ───────────────────────────────────────────────────────────────────

func config() workflow {
	return workflow{
		Name: "CI",
		On: workflowOn{
			PullRequest:      pushTrigger{Branches: []string{"**"}},
			Push:             pushTrigger{Branches: []string{"master"}},
			WorkflowDispatch: struct{}{},
		},
		Env: map[string]string{
			"LORRI_NO_INSTALL_PANIC_HANDLER": "absolutely",
		},
		Jobs: map[string]job{
			"j01-go-test-ubuntu-latest": {
				Name:   "Go tests (ubuntu-latest)",
				RunsOn: "ubuntu-latest",
				Steps:  goTestSteps(),
			},
			"j02-nix-build-ubuntu-latest": {
				Name:   "nix build (ubuntu-latest)",
				RunsOn: "ubuntu-latest",
				Steps:  stableSteps(),
			},
			"j11-go-test-macos-latest": {
				Name:   "Go tests (macos-latest)",
				RunsOn: "macos-latest",
				Steps:  goTestSteps(),
			},
			"j12-nix-build-macos-latest": {
				Name:   "nix build (macos-latest)",
				RunsOn: "macos-latest",
				Steps:  stableSteps(),
			},
		},
	}
}

// ── generate ─────────────────────────────────────────────────────────────────

func generate() ([]byte, error) {
	jsonBytes, err := json.Marshal(config())
	if err != nil {
		return nil, fmt.Errorf("marshal: %w", err)
	}
	cmd := exec.Command("yj", "-jy")
	cmd.Stdin = bytes.NewReader(jsonBytes)
	out, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("yj: %w", err)
	}
	return out, nil
}

// ── test subcommand ───────────────────────────────────────────────────────────

// resolveToolEnv calls nix-instantiate --eval to build a PATH string and
// RUN_TIME_CLOSURE from the pinned nixpkgs (via NIX_PATH).
// Returns (path, rtc, error).
func resolveToolEnv(root string) (string, string, error) {
	expr := `with import <nixpkgs> {};
let
  bins = lib.makeBinPath (map lib.getBin [ go nix direnv git bash yj ]);
  rtc  = "${callPackage ./nix/runtime.nix {}}";
in "${bins}:::${rtc}"`
	cmd := exec.Command("nix-instantiate", "--eval", "--json", "--read-write-mode", "-E", expr)
	cmd.Dir = root
	out, err := cmd.Output()
	if err != nil {
		return "", "", fmt.Errorf("nix-instantiate: %w\nstderr: %s", err, out)
	}
	// Output is a JSON string — unmarshal to strip quotes and escapes.
	var result string
	if err := json.Unmarshal(out, &result); err != nil {
		return "", "", fmt.Errorf("parse nix-instantiate output: %w\noutput: %s", err, out)
	}
	// Split on the sentinel ":::" we used to separate PATH from rtc.
	parts := strings.SplitN(result, ":::", 2)
	if len(parts) != 2 {
		return "", "", fmt.Errorf("unexpected nix-instantiate output: %q", result)
	}
	return parts[0], parts[1], nil
}

// runCheck runs cmd in root with the given environment, streaming output.
// Returns an error if the command exits non-zero.
func runCheck(root string, env []string, name string, argv ...string) error {
	fmt.Fprintf(os.Stderr, "ci: running %s\n", name)
	cmd := exec.Command(argv[0], argv[1:]...)
	cmd.Dir = root
	cmd.Env = env
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("%s: %w", name, err)
	}
	return nil
}

func runTests() error {
	root := repoRoot()

	// Save go binary path before we clear the environment.
	gobin, err := exec.LookPath("go")
	if err != nil {
		return fmt.Errorf("go not found on PATH: %w", err)
	}

	// Save the variables we want to preserve across the env clear.
	preserve := []string{"USER", "HOME", "TERM", "NIX_PATH", "TMPDIR", "TMP", "TEMP"}
	saved := make(map[string]string, len(preserve))
	for _, k := range preserve {
		saved[k] = os.Getenv(k)
	}

	fmt.Fprintln(os.Stderr, "ci: resolving tool paths from pinned nixpkgs...")
	toolPath, rtc, err := resolveToolEnv(root)
	if err != nil {
		return err
	}

	// Build a clean environment.
	os.Clearenv()

	env := []string{
		"LORRI_NO_INSTALL_PANIC_HANDLER=absolutely",
		"RUN_TIME_CLOSURE=" + rtc,
		"LORRI_ROOT=" + root,
		"PATH=" + toolPath,
	}
	// Restore preserved vars.
	for _, k := range preserve {
		if v := saved[k]; v != "" {
			env = append(env, k+"="+v)
		}
	}

	var failed []string

	if err := runCheck(root, env, "ci check", gobin, "run", "./cmd/ci", "check"); err != nil {
		fmt.Fprintf(os.Stderr, "ci: FAIL: %v\n", err)
		failed = append(failed, "ci check")
	}

	if err := runCheck(root, env, "go test ./...", gobin, "test", "./..."); err != nil {
		fmt.Fprintf(os.Stderr, "ci: FAIL: %v\n", err)
		failed = append(failed, "go test")
	}

	if len(failed) > 0 {
		return fmt.Errorf("checks failed: %s", strings.Join(failed, ", "))
	}
	fmt.Fprintln(os.Stderr, "ci: all checks passed")
	return nil
}

// ── main ─────────────────────────────────────────────────────────────────────

func usage() {
	fmt.Fprintf(os.Stderr, "usage: go run ./cmd/ci <generate|check|test>\n")
	os.Exit(2)
}

func main() {
	if len(os.Args) < 2 {
		usage()
	}

	switch os.Args[1] {
	case "test":
		if err := runTests(); err != nil {
			fmt.Fprintf(os.Stderr, "ci: %v\n", err)
			os.Exit(1)
		}

	case "generate":
		outPath := filepath.Join(repoRoot(), ".github", "workflows", "ci.yml")
		if len(os.Args) > 2 {
			outPath = os.Args[2]
		}
		content, err := generate()
		if err != nil {
			fmt.Fprintf(os.Stderr, "ci: %v\n", err)
			os.Exit(1)
		}
		if err := os.WriteFile(outPath, content, 0644); err != nil {
			fmt.Fprintf(os.Stderr, "ci: could not write %s: %v\n", outPath, err)
			os.Exit(1)
		}
		fmt.Fprintf(os.Stderr, "ci: wrote %s\n", outPath)

	case "check":
		outPath := filepath.Join(repoRoot(), ".github", "workflows", "ci.yml")
		content, err := generate()
		if err != nil {
			fmt.Fprintf(os.Stderr, "ci: %v\n", err)
			os.Exit(1)
		}
		existing, err := os.ReadFile(outPath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "ci: could not read %s: %v\n", outPath, err)
			os.Exit(1)
		}
		if !bytes.Equal(existing, content) {
			fmt.Fprintf(os.Stderr, "ci: %s is stale — run `go run ./cmd/ci generate` to regenerate\n", outPath)
			os.Exit(1)
		}

	default:
		usage()
	}
}
