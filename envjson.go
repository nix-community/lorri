package main

// envjson implements `lorri internal generate-env_`.
//
// The Nix keep-env-hack builder calls this after running stdenv setup to
// produce $out/env.json — a structured, ordered description of what the
// lorri environment looks like.  Downstream tooling (e.g. `lorri export`)
// reads env.json to apply the environment without needing direnv or bash.
//
// The classification rules mirror envrc.bash exactly:
//
//   unset   — vars that should be taken from the user's ambient shell
//   prepend — vars whose Nix value should be prepended to the ambient value
//   append  — vars accumulated via addToSearchPathWithCustomDelimiter
//   set     — everything else

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

// EnvChange describes a single environment variable operation.
// The JSON encoding is a list element in LorriEnv.Env.
type EnvChange struct {
	// Op is one of "set", "prepend", "append", "unset".
	Op string `json:"op"`
	// Name is the environment variable name.
	Name string `json:"name"`
	// Value is the variable's value from the Nix build environment.
	// Omitted for op "unset".
	Value string `json:"value,omitempty"`
	// Sep is the path separator used for prepend/append (e.g. ":").
	// Omitted for op "set" and "unset".
	Sep string `json:"sep,omitempty"`
}

// LorriEnv is the top-level structure written to $out/env.json.
type LorriEnv struct {
	Version int         `json:"version"`
	Env     []EnvChange `json:"env"`
}

// ---------------------------------------------------------------------------
// Classification tables (mirrors envrc.bash)
// ---------------------------------------------------------------------------

// puntVars is the set of variables that should not be exported into the
// user's shell at all — they are either meaningless in a non-build context
// or should be inherited from the ambient shell.
//
// Sources (matching envrc.bash comments):
//   - https://github.com/NixOS/nix/blob/92d08c02c84be34ec0df56ed718526c382845d1a/src/nix-build/nix-build.cc#L100
//   - https://github.com/NixOS/nix/blob/92d08c02c84be34ec0df56ed718526c382845d1a/src/nix-build/nix-build.cc#L385
//   - https://github.com/NixOS/nix/blob/92d08c02c84be34ec0df56ed718526c382845d1a/src/nix-build/nix-build.cc#L421
//   - bash variables: https://www.gnu.org/software/bash/manual/html_node/Bash-Variables.html
var puntVars = map[string]bool{
	"HOME":            true,
	"USER":            true,
	"LOGNAME":         true,
	"DISPLAY":         true,
	"TERM":            true,
	"IN_NIX_SHELL":    true,
	"TZ":              true,
	"PAGER":           true,
	"NIX_BUILD_SHELL": true,
	"SHLVL":           true,
	"TEMPDIR":         true,
	"TMPDIR":          true,
	"TEMP":            true,
	"TMP":             true,
	// https://github.com/NixOS/nix/blob/92d08c02c84be34ec0df56ed718526c382845d1a/src/nix-build/nix-build.cc#L421
	"NIX_ENFORCE_PURITY": true,
	// bash variables reported in https://github.com/target/lorri/issues/153
	"OLDPWD": true,
	"PWD":    true,
	"SHELL":  true,
	// https://github.com/target/lorri/issues/97
	// preHook is a build-time hook, not useful in a dev shell
	"preHook": true,
	// lorri plumbing — not meaningful outside the build sandbox
	"lorriBin": true,
	// bash sets $_ to the last argument of the previous command
	"_": true,
}

// prependVars is the set of variables whose Nix value should be prepended
// to whatever the user already has in their ambient shell, using ":" as
// the separator.
var prependVars = map[string]bool{
	"PATH":            true,
	"XDG_DATA_DIRS":   true,
	"XDG_CONFIG_DIRS": true,
}

// ---------------------------------------------------------------------------
// Varmap parsing
// ---------------------------------------------------------------------------

// appendEntry records that a variable should be appended (not replaced) when
// applied to the ambient shell environment.
type appendEntry struct {
	name string
	sep  string
}

// parseVarmap reads the NUL-delimited varmap file written by the monkey-patched
// addToSearchPathWithCustomDelimiter during stdenv setup.
//
// Format: repeated triples of NUL-terminated strings:
//
//	instruction \0 varname \0 separator \0
//
// Only "append" instructions are currently emitted; others are ignored.
// Duplicate (varname, sep) pairs are deduplicated (first wins, matching the
// original bash dedup logic).
func parseVarmap(path string) ([]appendEntry, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, fmt.Errorf("parseVarmap: read %s: %w", path, err)
	}

	parts := strings.Split(string(data), "\x00")
	// parts has a trailing empty string after the final \0; trim it.
	if len(parts) > 0 && parts[len(parts)-1] == "" {
		parts = parts[:len(parts)-1]
	}

	seen := make(map[string]bool)
	var entries []appendEntry
	for i := 0; i+2 < len(parts); i += 3 {
		instruction := parts[i]
		varname := parts[i+1]
		sep := parts[i+2]
		if instruction != "append" {
			continue
		}
		key := varname + "\x00" + sep
		if seen[key] {
			continue
		}
		seen[key] = true
		entries = append(entries, appendEntry{name: varname, sep: sep})
	}
	return entries, nil
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

// buildLorriEnv classifies the current process environment (os.Environ())
// against the varmap append entries and returns the ordered list of changes.
func buildLorriEnv(varmapPath string) (*LorriEnv, error) {
	appendEntries, err := parseVarmap(varmapPath)
	if err != nil {
		return nil, err
	}

	// Build a lookup: varname → separator for append vars.
	appendSep := make(map[string]string, len(appendEntries))
	for _, e := range appendEntries {
		appendSep[e.name] = e.sep
	}

	// origPreHook needs special handling: its value is emitted as "preHook".
	// We scan os.Environ() once and handle it inline.
	var changes []EnvChange

	for _, kv := range os.Environ() {
		idx := strings.IndexByte(kv, '=')
		if idx < 0 {
			// Malformed entry; skip.
			continue
		}
		name := kv[:idx]
		value := kv[idx+1:]

		// origPreHook → emit as preHook (set), skip origPreHook itself.
		// https://github.com/target/lorri/issues/97
		if name == "origPreHook" {
			changes = append(changes, EnvChange{
				Op:    "set",
				Name:  "preHook",
				Value: value,
			})
			continue
		}

		if puntVars[name] {
			// "pass" means lorri knows Nix set this variable but deliberately
			// leaves the ambient shell value untouched. The Nix build value is
			// recorded for display/diagnostic purposes.
			changes = append(changes, EnvChange{Op: "pass", Name: name, Value: value})
			continue
		}

		if prependVars[name] {
			changes = append(changes, EnvChange{
				Op:    "prepend",
				Name:  name,
				Value: value,
				Sep:   ":",
			})
			continue
		}

		if sep, ok := appendSep[name]; ok {
			changes = append(changes, EnvChange{
				Op:    "append",
				Name:  name,
				Value: value,
				Sep:   sep,
			})
			continue
		}

		changes = append(changes, EnvChange{
			Op:    "set",
			Name:  name,
			Value: value,
		})
	}

	return &LorriEnv{Version: 1, Env: changes}, nil
}

// opGenerateEnv is the implementation of `lorri internal generate-env_`.
// It reads the varmap from varmapPath, classifies os.Environ(), and writes
// $out/env.json.
func opGenerateEnv(varmapPath string) error {
	outDir := os.Getenv("out")
	if outDir == "" {
		return fmt.Errorf("generate-env_: $out is not set (must be run inside a Nix builder)")
	}

	lorriEnv, err := buildLorriEnv(varmapPath)
	if err != nil {
		return fmt.Errorf("generate-env_: %w", err)
	}

	outPath := outDir + "/env.json"
	f, err := os.Create(outPath)
	if err != nil {
		return fmt.Errorf("generate-env_: create %s: %w", outPath, err)
	}
	defer f.Close()

	enc := json.NewEncoder(f)
	enc.SetIndent("", "  ")
	if err := enc.Encode(lorriEnv); err != nil {
		return fmt.Errorf("generate-env_: write %s: %w", outPath, err)
	}

	return nil
}
