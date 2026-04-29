package main

// LorriDB: thin wrapper around zombiezen sqlite for the gc_roots table.
// Mirrors src/sqlite.rs Sqlite + the relevant parts of src/project.rs.
//
// All methods must be called from a single goroutine (zombiezen Conn is
// not concurrency-safe).

import (
	"fmt"
	"os"
	"path/filepath"
	"time"

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

// shellSessionSchema defines the tables used by lorri hook/export to track
// per-shell-session environment state.
//
// lorri_shell_sessions: one row per active shell session. session_id is a
// random 64-bit value encoded as base64, generated once per shell and stored
// in $LORRI_SESSION_ID. shell_pid is the PID of the shell process, used to
// detect stale sessions (kill(pid, 0) == ESRCH). project is the shell.nix
// path that is currently loaded. env_hash is the hash of the env.json that
// was applied, used to detect when a new build has landed.
//
// lorri_shell_env_prev: one row per env var that lorri touched when applying
// an env.json. prev_value is the value the var had before lorri set it, or
// NULL if the var did not exist. These rows are used to revert the environment
// when the user leaves the project directory. Rows are deleted immediately on
// leave, and stale sessions (dead shell_pid) are swept when a new session is
// created.
const shellSessionSchema = `
CREATE TABLE IF NOT EXISTS lorri_shell_sessions (
    session_id  TEXT    NOT NULL PRIMARY KEY,
    shell_pid   INTEGER NOT NULL,
    project     TEXT    NOT NULL,
    env_hash    TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS lorri_shell_env_prev (
    session_id  TEXT    NOT NULL REFERENCES lorri_shell_sessions(session_id),
    var_name    TEXT    NOT NULL,
    prev_value  TEXT,
    PRIMARY KEY (session_id, var_name)
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
	if err := sqlitex.ExecScript(conn, shellSessionSchema); err != nil {
		conn.Close()
		return nil, fmt.Errorf("OpenLorriDB: create shell session schema: %w", err)
	}
	return &LorriDB{conn: conn}, nil
}

// OpenLorriDBReadOnly opens the SQLite database at path in read-only mode.
// The schema is not created — use this only when the DB is known to exist.
// Read-only mode avoids blocking writers (e.g. the daemon) and is safe to
// call from prompt hooks and other high-frequency codepaths.
func OpenLorriDBReadOnly(path string) (*LorriDB, error) {
	conn, err := sqlite.OpenConn(path, sqlite.OpenReadOnly)
	if err != nil {
		return nil, fmt.Errorf("OpenLorriDBReadOnly: open %s: %w", path, err)
	}
	return &LorriDB{conn: conn}, nil
}

// FindRegisteredProjectForDir walks up from dir through its ancestors and
// returns the first DBProject registered in the lorri database whose nix_file
// lives directly in that directory. Returns nil, nil if nothing is found.
//
// Unlike findNixFile / resolveProjectFile (which look at the filesystem),
// this function only considers projects lorri already knows about. It handles
// both shell.nix and flake projects.
//
// Each ancestor level issues a single targeted SQL query rather than scanning
// all rows, so the cost is O(depth) queries of O(1) each.
func (db *LorriDB) FindRegisteredProjectForDir(dir string) (*DBProject, error) {
	for d := filepath.Clean(dir); ; d = filepath.Dir(d) {
		var found *DBProject
		err := sqlitex.Execute(db.conn,
			`SELECT nix_file, is_flake, flake_installable FROM gc_roots
			 WHERE :path || '/' = rtrim(nix_file, replace(nix_file, '/', ''))
			 LIMIT 1`,
			&sqlitex.ExecOptions{
				Named: map[string]any{":path": d},
				ResultFunc: func(stmt *sqlite.Stmt) error {
					found = &DBProject{
						NixFile:          stmt.ColumnText(0),
						IsFlake:          stmt.ColumnInt(1) != 0,
						FlakeInstallable: stmt.ColumnText(2),
					}
					return nil
				},
			},
		)
		if err != nil {
			return nil, err
		}
		if found != nil {
			return found, nil
		}
		parent := filepath.Dir(d)
		if parent == d {
			break
		}
	}
	return nil, nil
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

// ---------------------------------------------------------------------------
// Shell session methods
// ---------------------------------------------------------------------------

// ShellSession is a row from lorri_shell_sessions.
type ShellSession struct {
	SessionID string
	ShellPID  int
	Project   string
	EnvHash   string
	CreatedAt int64
}

// CreateShellSession inserts a new session row and sweeps stale sessions
// (those whose shell_pid is no longer alive). Must be called before inserting
// env_prev rows.
func (db *LorriDB) CreateShellSession(s ShellSession) error {
	// Sweep stale sessions whose shell PID is no longer alive.
	if err := db.sweepStaleSessions(); err != nil {
		// Non-fatal — log and continue.
		_ = err
	}
	return sqlitex.Execute(db.conn,
		`INSERT INTO lorri_shell_sessions (session_id, shell_pid, project, env_hash, created_at)
		 VALUES (:session_id, :shell_pid, :project, :env_hash, :created_at)`,
		&sqlitex.ExecOptions{
			Named: map[string]any{
				":session_id": s.SessionID,
				":shell_pid":  s.ShellPID,
				":project":    s.Project,
				":env_hash":   s.EnvHash,
				":created_at": s.CreatedAt,
			},
		},
	)
}

// UpdateShellSession updates the project and env_hash for an existing session.
func (db *LorriDB) UpdateShellSession(sessionID, project, envHash string) error {
	return sqlitex.Execute(db.conn,
		`UPDATE lorri_shell_sessions SET project = :project, env_hash = :env_hash
		 WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{
			Named: map[string]any{
				":session_id": sessionID,
				":project":    project,
				":env_hash":   envHash,
			},
		},
	)
}

// GetShellSession returns the session row for sessionID, or nil if not found.
func (db *LorriDB) GetShellSession(sessionID string) (*ShellSession, error) {
	var s *ShellSession
	err := sqlitex.Execute(db.conn,
		`SELECT session_id, shell_pid, project, env_hash, created_at
		 FROM lorri_shell_sessions WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{
			Named: map[string]any{":session_id": sessionID},
			ResultFunc: func(stmt *sqlite.Stmt) error {
				s = &ShellSession{
					SessionID: stmt.ColumnText(0),
					ShellPID:  int(stmt.ColumnInt64(1)),
					Project:   stmt.ColumnText(2),
					EnvHash:   stmt.ColumnText(3),
					CreatedAt: stmt.ColumnInt64(4),
				}
				return nil
			},
		},
	)
	return s, err
}

// SaveSessionAndEnvPrev creates (or updates) the shell session row and stores
// the pre-lorri env var values
func (db *LorriDB) SaveSessionAndEnvPrev(sessionID, project, envHash string, prev map[string]*string) (retErr error) {
	endTx := sqlitex.Save(db.conn)
	defer endTx(&retErr)

	shellPID := os.Getppid()
	if err := db.CreateShellSession(ShellSession{
		SessionID: sessionID,
		ShellPID:  shellPID,
		Project:   project,
		EnvHash:   envHash,
		CreatedAt: time.Now().Unix(),
	}); err != nil {
		// Session may already exist (e.g. re-entering same project). Update instead.
		if err := db.UpdateShellSession(sessionID, project, envHash); err != nil {
			return err
		}
	}
	return db.SetEnvPrev(sessionID, prev)
}

// SetEnvPrev stores the pre-lorri values for all vars touched in this session.
// Replaces any existing rows for this session.
func (db *LorriDB) SetEnvPrev(sessionID string, prev map[string]*string) (retErr error) {
	// Wrap all inserts in a single transaction — without this, each INSERT is
	// its own fsync which makes writing 60+ env vars take ~350ms.
	endTx := sqlitex.Save(db.conn)
	defer endTx(&retErr)

	// Delete existing rows for this session first.
	if err := sqlitex.Execute(db.conn,
		`DELETE FROM lorri_shell_env_prev WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{
			Named: map[string]any{":session_id": sessionID},
		},
	); err != nil {
		return err
	}
	for varName, prevValue := range prev {
		var val any
		if prevValue != nil {
			val = *prevValue
		}
		if err := sqlitex.Execute(db.conn,
			`INSERT INTO lorri_shell_env_prev (session_id, var_name, prev_value)
			 VALUES (:session_id, :var_name, :prev_value)`,
			&sqlitex.ExecOptions{
				Named: map[string]any{
					":session_id": sessionID,
					":var_name":   varName,
					":prev_value": val,
				},
			},
		); err != nil {
			return err
		}
	}
	return nil
}

// GetEnvPrev returns the prev-value map for a session.
// Map value is nil if the var did not exist before lorri set it.
func (db *LorriDB) GetEnvPrev(sessionID string) (map[string]*string, error) {
	prev := make(map[string]*string)
	err := sqlitex.Execute(db.conn,
		`SELECT var_name, prev_value FROM lorri_shell_env_prev WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{
			Named: map[string]any{":session_id": sessionID},
			ResultFunc: func(stmt *sqlite.Stmt) error {
				varName := stmt.ColumnText(0)
				if stmt.ColumnType(1) == sqlite.TypeNull {
					prev[varName] = nil
				} else {
					v := stmt.ColumnText(1)
					prev[varName] = &v
				}
				return nil
			},
		},
	)
	return prev, err
}

// DeleteShellSession removes the session and all its env_prev rows atomically.
func (db *LorriDB) DeleteShellSession(sessionID string) (retErr error) {
	endTx := sqlitex.Save(db.conn)
	defer endTx(&retErr)

	if err := sqlitex.Execute(db.conn,
		`DELETE FROM lorri_shell_env_prev WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{Named: map[string]any{":session_id": sessionID}},
	); err != nil {
		return err
	}
	return sqlitex.Execute(db.conn,
		`DELETE FROM lorri_shell_sessions WHERE session_id = :session_id`,
		&sqlitex.ExecOptions{Named: map[string]any{":session_id": sessionID}},
	)
}

// sweepStaleSessions deletes sessions whose shell_pid is no longer alive.
func (db *LorriDB) sweepStaleSessions() (retErr error) {
	// Collect all session IDs and their PIDs.
	type row struct {
		id  string
		pid int
	}
	var rows []row
	err := sqlitex.Execute(db.conn,
		`SELECT session_id, shell_pid FROM lorri_shell_sessions`,
		&sqlitex.ExecOptions{
			ResultFunc: func(stmt *sqlite.Stmt) error {
				rows = append(rows, row{
					id:  stmt.ColumnText(0),
					pid: int(stmt.ColumnInt64(1)),
				})
				return nil
			},
		},
	)
	if err != nil {
		return err
	}

	endTx := sqlitex.Save(db.conn)
	defer endTx(&retErr)

	for _, r := range rows {
		if !pidAlive(r.pid) {
			if err := db.DeleteShellSession(r.id); err != nil {
				return err
			}
		}
	}
	return nil
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
