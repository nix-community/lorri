package main

// op_init: write bootstrap files to the current directory.
// Mirrors src/ops.rs op_init().

import (
	_ "embed"
	"fmt"
	"os"
)

//go:embed trivial-shell.nix
var trivialShellNix string

//go:embed default-envrc
var defaultEnvrc string

// opInit writes shell.nix and .envrc to the current directory,
// skipping each file if it already exists.
func opInit() error {
	if err := createIfMissing("./shell.nix", trivialShellNix,
		"Make sure shell.nix is of a form that works with nix-shell."); err != nil {
		return err
	}
	if err := createIfMissing("./.envrc", defaultEnvrc,
		`Please add 'eval "$(lorri direnv)"' to .envrc to set up lorri support.`); err != nil {
		return err
	}
	fmt.Fprintln(os.Stderr, "lorri: done")
	return nil
}

// createIfMissing writes contents to path only if the file does not already
// exist. Mirrors ops.rs create_if_missing().
func createIfMissing(path, contents, msg string) error {
	if _, err := os.Stat(path); err == nil {
		fmt.Fprintf(os.Stderr, "lorri: file already exists, skipping: %s — %s\n", path, msg)
		return nil
	}
	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		return fmt.Errorf("could not write %s: %w", path, err)
	}
	fmt.Fprintf(os.Stderr, "lorri: wrote file: %s\n", path)
	return nil
}
