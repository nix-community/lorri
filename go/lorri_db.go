package main

// LorriDB: thin wrapper around zombiezen sqlite for the gc_roots table.
// Mirrors src/sqlite.rs Sqlite + the relevant parts of src/project.rs.
//
// All methods must be called from a single goroutine (zombiezen Conn is
// not concurrency-safe).

import (
	"fmt"
	"os"

	"zombiezen.com/go/sqlite"
	"zombiezen.com/go/sqlite/sqlitex"
)

const gcRootsSchema = `
CREATE TABLE IF NOT EXISTS gc_roots (
    id INTEGER PRIMARY KEY,
    nix_file TEXT UNIQUE NOT NULL,
    last_updated EPOCH_TIME,
    is_flake BOOLEAN NOT NULL DEFAULT FALSE,
    flake_installable TEXT DEFAULT NULL
);
`

// LorriDB wraps a single zombiezen sqlite connection.
type LorriDB struct {
	conn *sqlite.Conn
}

// OpenLorriDB opens (or creates) the SQLite database at path and ensures
// the schema exists.
func OpenLorriDB(path string) (*LorriDB, error) {
	conn, err := sqlite.OpenConn(path)
	if err != nil {
		return nil, fmt.Errorf("OpenLorriDB: open %s: %w", path, err)
	}
	if err := sqlitex.ExecScript(conn, gcRootsSchema); err != nil {
		conn.Close()
		return nil, fmt.Errorf("OpenLorriDB: create schema: %w", err)
	}
	return &LorriDB{conn: conn}, nil
}

// Close closes the underlying SQLite connection.
func (db *LorriDB) Close() error {
	return db.conn.Close()
}

// UpsertProject inserts a project row if it does not already exist.
// On conflict (same nix_file) it does nothing — mirrors Rust's
// ON CONFLICT (nix_file) DO NOTHING.
func (db *LorriDB) UpsertProject(nixFile string, isFlake bool, flakeInstallable string) error {
	var installableVal any
	if isFlake && flakeInstallable != "" {
		installableVal = flakeInstallable
	}
	return sqlitex.Execute(db.conn,
		`INSERT INTO gc_roots (nix_file, is_flake, flake_installable)
		 VALUES (:nix_file, :is_flake, :flake_installable)
		 ON CONFLICT (nix_file) DO NOTHING`,
		&sqlitex.ExecOptions{
			Named: map[string]any{
				":nix_file":          nixFile,
				":is_flake":          isFlake,
				":flake_installable": installableVal,
			},
		},
	)
}

// DeleteProject removes a project row by nix_file path.
func (db *LorriDB) DeleteProject(nixFile string) error {
	return sqlitex.Execute(db.conn,
		`DELETE FROM gc_roots WHERE nix_file = :nix_file`,
		&sqlitex.ExecOptions{
			Named: map[string]any{":nix_file": nixFile},
		},
	)
}

// DBProject is a row from the gc_roots table.
type DBProject struct {
	NixFile          string
	IsFlake          bool
	FlakeInstallable string // empty string if NULL or not a flake
}

// ListProjects returns all rows from gc_roots.
func (db *LorriDB) ListProjects() ([]DBProject, error) {
	var projects []DBProject
	err := sqlitex.Execute(db.conn,
		`SELECT nix_file, is_flake, flake_installable FROM gc_roots`,
		&sqlitex.ExecOptions{
			ResultFunc: func(stmt *sqlite.Stmt) error {
				projects = append(projects, DBProject{
					NixFile:          stmt.ColumnText(0),
					IsFlake:          stmt.ColumnInt(1) != 0,
					FlakeInstallable: stmt.ColumnText(2),
				})
				return nil
			},
		},
	)
	return projects, err
}

// AutoGCRemovedProjects deletes rows whose nix_file no longer exists on disk.
// Mirrors project.rs Project::auto_gc_removed_project_files.
func (db *LorriDB) AutoGCRemovedProjects() error {
	projects, err := db.ListProjects()
	if err != nil {
		return fmt.Errorf("AutoGCRemovedProjects: list: %w", err)
	}
	for _, p := range projects {
		if _, err := os.Stat(p.NixFile); os.IsNotExist(err) {
			if err := db.DeleteProject(p.NixFile); err != nil {
				return fmt.Errorf("AutoGCRemovedProjects: delete %s: %w", p.NixFile, err)
			}
		}
	}
	return nil
}
