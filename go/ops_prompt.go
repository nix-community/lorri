package main

// op_prompt: print a lorri indicator for shell prompts.
// Mirrors src/ops.rs op_prompt() + is_subdir_of_known_project().

import (
	"fmt"
	"os"
	"path/filepath"

	"zombiezen.com/go/sqlite"
	"zombiezen.com/go/sqlite/sqlitex"
)

// opPrompt prints "ℓ" (or " ℓ") if the current directory is inside a
// lorri-watched project. Prints nothing otherwise.
// Mirrors op_prompt(PromptOptions::Default { include_leading_space }).
func opPrompt(includeLeadingSpace bool, paths *Paths) error {
	cwd, err := os.Getwd()
	if err != nil {
		// Can't determine cwd; print nothing.
		return nil
	}

	// Open DB read-only so we don't block the daemon.
	conn, err := sqlite.OpenConn(paths.SQLiteDB.String(), sqlite.OpenReadOnly)
	if err != nil {
		// DB doesn't exist yet (no projects watched); print nothing.
		return nil
	}
	defer conn.Close()

	_, found := isSubdirOfKnownProject(conn, cwd)
	if !found {
		return nil
	}

	if includeLeadingSpace {
		fmt.Print(" ℓ")
	} else {
		fmt.Print("ℓ")
	}
	return nil
}

// isSubdirOfKnownProject checks whether path (or any of its ancestors) is
// directly the parent directory of a registered nix_file.
// Returns (nixFile, true) if found, ("", false) otherwise.
// Mirrors is_subdir_of_known_project() using the same SQL logic.
func isSubdirOfKnownProject(conn *sqlite.Conn, path string) (string, bool) {
	// Walk upward through ancestors of path.
	for dir := filepath.Clean(path); ; dir = filepath.Dir(dir) {
		// The SQL check from Rust:
		//   WHERE :path || '/' = rtrim(nix_file, replace(nix_file, '/', ''))
		// This checks that the directory of nix_file equals :path.
		// We replicate this logic in Go: check if nix_file's Dir() == dir.
		var found string
		err := sqlitex.Execute(conn,
			`SELECT nix_file FROM gc_roots
			 WHERE :path || '/' = rtrim(nix_file, replace(nix_file, '/', ''))
			 LIMIT 1`,
			&sqlitex.ExecOptions{
				Named: map[string]any{":path": dir},
				ResultFunc: func(stmt *sqlite.Stmt) error {
					found = stmt.ColumnText(0)
					return nil
				},
			},
		)
		if err == nil && found != "" {
			return found, true
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			// Reached the filesystem root.
			break
		}
	}
	return "", false
}
