// cmd/ci generates .github/workflows/ci.json.
//
// Usage:
//
//	go run ./cmd/ci                         # writes ci.json in place
//	go run ./cmd/ci --check                 # exits non-zero if ci.json is stale
//	go run ./cmd/ci --out /path/to/ci.json  # write to a specific path
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
)

// repoRoot returns the root of the repository by walking up from this
// source file's location. This works correctly with "go run ./cmd/ci".
func repoRoot() string {
	_, file, _, _ := runtime.Caller(0)
	// file is .../cmd/ci/main.go — go up two levels
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
			Name: "Build CI tests",
			Run: "nix-build \\\n" +
				"  --out-link ./ci-tests \\\n" +
				"  --arg isDevelopmentShell false \\\n" +
				"  -A ci.testsuite \\\n" +
				"  shell.nix\n",
		},
		step{Name: "Run CI tests", Run: "./ci-tests\n"},
	)
}

func simpleChecksSteps() []step {
	return append(commonSteps,
		step{
			Name: "Build simple checks",
			Run: "nix-build \\\n" +
				"  --out-link ./simple-tests \\\n" +
				"  --arg isDevelopmentShell false \\\n" +
				"  -A ci.testsuite-simple-checks \\\n" +
				"  shell.nix\n",
		},
		step{Name: "Run simple checks", Run: "./simple-tests\n"},
	)
}

func stableSteps() []step {
	return append(commonSteps,
		step{Name: "Build", Run: "nix-build\n"},
		step{Name: "Install", Run: "nix-env -i ./result\n"},
	)
}

func overlaySteps() []step {
	return append(commonSteps,
		step{
			Name: "Build w/ overlay (stable)",
			Run:  "nix-build ./nix/overlay.nix -A lorri --arg pkgs ./nix/nixpkgs-stable.json\n",
		},
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
			"j01-simple-checks": {
				Name:   "Simple Checks",
				RunsOn: "ubuntu-latest",
				Steps:  simpleChecksSteps(),
			},
			"j02-go-test-ubuntu-latest": {
				Name:   "Go tests (ubuntu-latest)",
				RunsOn: "ubuntu-latest",
				Steps:  goTestSteps(),
			},
			"j03-nix-build_stable-ubuntu-latest": {
				Name:   "nix-build [nixos stable] (ubuntu-latest)",
				RunsOn: "ubuntu-latest",
				Steps:  stableSteps(),
			},
			"j04-overlay-ubuntu-latest": {
				Name:   "Overlay builds (ubuntu-latest)",
				RunsOn: "ubuntu-latest",
				Steps:  overlaySteps(),
			},
			"j12-go-test-macos-latest": {
				Name:   "Go tests (macos-latest)",
				RunsOn: "macos-latest",
				Steps:  goTestSteps(),
			},
			"j13-nix-build_stable-macos-latest": {
				Name:   "nix-build [nixos stable] (macos-latest)",
				RunsOn: "macos-latest",
				Steps:  stableSteps(),
			},
			"j14-overlay-macos-latest": {
				Name:   "Overlay builds (macos-latest)",
				RunsOn: "macos-latest",
				Steps:  overlaySteps(),
			},
		},
	}
}

// ── generate ─────────────────────────────────────────────────────────────────

func generate() ([]byte, error) {
	out, err := json.MarshalIndent(config(), "", "  ")
	if err != nil {
		return nil, err
	}
	return append(out, '\n'), nil
}

// ── main ─────────────────────────────────────────────────────────────────────

func main() {
	var outPath string
	var check bool
	flag.StringVar(&outPath, "out", "", "path to write ci.json (default: .github/workflows/ci.json in repo root)")
	flag.BoolVar(&check, "check", false, "check that ci.json is up to date instead of writing it")
	flag.Parse()

	if outPath == "" {
		outPath = filepath.Join(repoRoot(), ".github", "workflows", "ci.json")
	}

	content, err := generate()
	if err != nil {
		fmt.Fprintf(os.Stderr, "ci: marshal error: %v\n", err)
		os.Exit(1)
	}

	if check {
		existing, err := os.ReadFile(outPath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "ci: could not read %s: %v\n", outPath, err)
			os.Exit(1)
		}
		if !bytes.Equal(existing, content) {
			fmt.Fprintf(os.Stderr, "ci: %s is stale — run `go run ./cmd/ci` to regenerate\n", outPath)
			os.Exit(1)
		}
		return
	}

	if err := os.WriteFile(outPath, content, 0644); err != nil {
		fmt.Fprintf(os.Stderr, "ci: could not write %s: %v\n", outPath, err)
		os.Exit(1)
	}
	fmt.Fprintf(os.Stderr, "ci: wrote %s\n", outPath)
}
