package main

// export.go implements `lorri export <shell>`.
//
// This is the command the shell hook calls on every prompt. It:
//
//  1. Finds the lorri project for the current directory (by walking up).
//  2. Reads env.json from the project's GC root.
//  3. Applies the env changes against the current ambient environment,
//     storing per-var revert state in SQLite keyed by session ID.
//  4. Emits shell export/unset commands for the diff.
//  5. On leaving a project directory, reverts and cleans up session state.
//
// Session state is tracked via three env vars:
//
//	LORRI_SESSION_ID  — random 64-bit value as base64url, generated once
//	LORRI_PROJECT     — shell.nix path of the currently loaded project
//	LORRI_ENV_HASH    — hash of the env.json that was applied

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	direnv "github.com/nix-community/lorri/direnv_vendor"
)

// opExport is the implementation of `lorri export <shell>`.
//
// The special shell name "direnv-adapter" is handled separately: it emits
// watch_file lines + bash export/unset commands for direnv to eval, without
// any session state management. This is for users who need direnv for editor
// plugin support (emacs-direnv, vscode-direnv, etc.) but want lorri to manage
// the environment data. Their .envrc contains: eval "$(lorri export direnv-adapter)"
func opExport(shellName string) error {
	if shellName == "direnv-adapter" {
		return opExportDirenvAdapter()
	}

	sh, ok := direnv.Shells[shellName]
	if !ok {
		return exitUserError(fmt.Sprintf("export: unsupported shell %q", shellName), nil)
	}

	paths, err := mustInitPaths()
	if err != nil {
		return err
	}

	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return fmt.Errorf("export: open db: %w", err)
	}
	defer db.Close()

	// Resolve or generate the session ID. A fresh ID is generated if no
	// project is currently loaded (LORRI_SESSION_ID unset); if one is set
	// it is reused so the revert state remains consistent across prompts.
	sessionID := os.Getenv("LORRI_SESSION_ID")
	if sessionID == "" {
		id, err := newSessionID()
		if err != nil {
			return fmt.Errorf("export: generate session id: %w", err)
		}
		sessionID = id
	}

	currentProject := os.Getenv("LORRI_PROJECT")
	currentHash := os.Getenv("LORRI_ENV_HASH")
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("export: getwd: %w", err)
	}

	// Find lorri project for the current directory.
	foundDBProject, err := db.FindRegisteredProjectForDir(cwd)
	if err != nil {
		return fmt.Errorf("export: find project: %w", err)
	}
	foundProject := ""
	if foundDBProject != nil {
		foundProject = foundDBProject.NixFile
	}

	export := make(direnv.ShellExport)

	switch {
	case foundProject == "" && currentProject == "":
		// Not in a lorri project, never were. Nothing to do.

	case foundProject == "" && currentProject != "":
		// Left the project. Revert environment.
		if err := revertEnv(db, sessionID, export); err != nil {
			fmt.Fprintf(os.Stderr, "lorri: warning: %v\n", err)
		}
		export.Remove("LORRI_SESSION_ID")
		export.Remove("LORRI_PROJECT")
		export.Remove("LORRI_ENV_HASH")

	case foundProject != "" && currentProject == "":
		// Entered a new project.
		envHash, err := applyProject(db, sessionID, foundProject, paths, export)
		if err != nil {
			fmt.Fprintln(os.Stderr, "lorri: has not completed an evaluation for this project yet")
			_ = opPingWithRebuild(paths, NewShellNixProjectFile(mustAbsPath(foundProject)), RebuildAlways)
			break
		}
		export.Add("LORRI_SESSION_ID", sessionID)
		export.Add("LORRI_PROJECT", foundProject)
		export.Add("LORRI_ENV_HASH", envHash)

	case foundProject == currentProject && foundProject != "":
		// Same project — check if a new build has landed.
		envHash, newHash, err := currentEnvHash(foundProject, paths)
		if err != nil {
			// GC root may not exist yet — daemon hasn't built it. Silently skip.
			break
		}
		if newHash == currentHash {
			// No change.
			break
		}
		// New build landed: revert old env, apply new.
		if err := revertEnv(db, sessionID, export); err != nil {
			fmt.Fprintf(os.Stderr, "lorri: warning: %v\n", err)
		}
		if _, err := applyProjectFromHash(db, sessionID, foundProject, envHash, paths, export); err != nil {
			fmt.Fprintln(os.Stderr, "lorri: has not completed an evaluation for this project yet")
			_ = opPingWithRebuild(paths, NewShellNixProjectFile(mustAbsPath(foundProject)), RebuildAlways)
			break
		}
		export.Add("LORRI_SESSION_ID", sessionID)
		export.Add("LORRI_PROJECT", foundProject)
		export.Add("LORRI_ENV_HASH", newHash)

	case foundProject != currentProject:
		// Switched from one lorri project to another.
		if err := revertEnv(db, sessionID, export); err != nil {
			fmt.Fprintf(os.Stderr, "lorri: warning: %v\n", err)
		}
		envHash, err := applyProject(db, sessionID, foundProject, paths, export)
		if err != nil {
			fmt.Fprintln(os.Stderr, "lorri: has not completed an evaluation for this project yet")
			_ = opPingWithRebuild(paths, NewShellNixProjectFile(mustAbsPath(foundProject)), RebuildAlways)
			break
		}
		export.Add("LORRI_SESSION_ID", sessionID)
		export.Add("LORRI_PROJECT", foundProject)
		export.Add("LORRI_ENV_HASH", envHash)
	}

	out, err := sh.Export(export)
	if err != nil {
		return fmt.Errorf("export: render: %w", err)
	}
	fmt.Print(out)
	return nil
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// newSessionID generates a random 64-bit value encoded as base64url (no padding).
func newSessionID() (string, error) {
	b := make([]byte, 8)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}



// gcRootEnvJSON returns the path to env.json for a given project's GC root.
func gcRootEnvJSON(project string, paths *Paths) (string, error) {
	projectFile := NewShellNixProjectFile(mustAbsPath(project))
	gcRootLink := gcRootPathForProject(paths.GCRootDir, projectFile).String()
	target, err := os.Readlink(gcRootLink)
	if err != nil {
		return "", fmt.Errorf("gc root not ready for %s: %w", project, err)
	}
	return filepath.Join(target, "env.json"), nil
}

// readEnvJSON reads and parses env.json from a store path.
func readEnvJSON(path string) (*LorriEnv, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("read env.json: %w", err)
	}
	defer f.Close()
	var env LorriEnv
	if err := json.NewDecoder(f).Decode(&env); err != nil {
		return nil, fmt.Errorf("parse env.json: %w", err)
	}
	return &env, nil
}

// hashEnvJSON returns a short hash of env.json contents for change detection.
func hashEnvJSON(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(data)
	return base64.RawURLEncoding.EncodeToString(sum[:8]), nil
}

// currentEnvHash returns the env.json path and its hash for a project.
func currentEnvHash(project string, paths *Paths) (envJSONPath, hash string, err error) {
	envJSONPath, err = gcRootEnvJSON(project, paths)
	if err != nil {
		return "", "", err
	}
	hash, err = hashEnvJSON(envJSONPath)
	return envJSONPath, hash, err
}

// applyProject reads the current env.json for a project, saves revert state,
// and populates export with the changes. Returns the env hash.
func applyProject(db *LorriDB, sessionID, project string, paths *Paths, export direnv.ShellExport) (string, error) {
	envJSONPath, hash, err := currentEnvHash(project, paths)
	if err != nil {
		return "", fmt.Errorf("project %s: %w", project, err)
	}
	_, err = applyProjectFromHash(db, sessionID, project, envJSONPath, paths, export)
	return hash, err
}

func applyProjectFromHash(db *LorriDB, sessionID, project, envJSONPath string, paths *Paths, export direnv.ShellExport) (string, error) {
	start := time.Now()
	lorriEnv, err := readEnvJSON(envJSONPath)
	if err != nil {
		return "", err
	}

	hash, err := hashEnvJSON(envJSONPath)
	if err != nil {
		return "", err
	}

	// Snapshot the current values of every var we're about to touch.
	// Skip "pass" vars — they are never modified on enter so there is nothing
	// to restore on leave. Snapshotting them would cause revertEnv to emit
	// set/unset commands for read-only shell variables (e.g. SHLVL, PWD in fish).
	prev := make(map[string]*string)
	for _, change := range lorriEnv.Env {
		if change.Op == "pass" {
			continue
		}
		cur, exists := os.LookupEnv(change.Name)
		if exists {
			v := cur
			prev[change.Name] = &v
		} else {
			prev[change.Name] = nil
		}
	}

	// Save session + revert state atomically.
	if err := db.SaveSessionAndEnvPrev(sessionID, project, hash, prev); err != nil {
		return "", fmt.Errorf("save revert state: %w", err)
	}

	// Compute the export diff: apply each change against the ambient env.
	ambient := ambientEnv()
	for _, change := range lorriEnv.Env {
		applyChange(change, ambient, export)
	}

	printEnvSummary(lorriEnv, prev, time.Since(start))
	return hash, nil
}

// revertEnv reads the saved revert state and populates export with the
// rollback commands, then deletes the session from the DB.
func revertEnv(db *LorriDB, sessionID string, export direnv.ShellExport) error {
	session, err := db.GetShellSession(sessionID)
	if err != nil {
		return fmt.Errorf("revert: get session (session_id=%s): %w", sessionID, err)
	}

	prev, err := db.GetEnvPrev(sessionID)
	if err != nil {
		return fmt.Errorf("revert: get prev state (session_id=%s): %w", sessionID, err)
	}
	if len(prev) == 0 {
		// No state — may have been cleaned up already.
		return fmt.Errorf("revert: no session state found (session_id=%s) — environment may not revert cleanly", sessionID)
	}
	for varName, prevValue := range prev {
		if prevValue == nil {
			export.Remove(varName)
		} else {
			export.Add(varName, *prevValue)
		}
	}
	_ = db.DeleteShellSession(sessionID)
	project := ""
	if session != nil {
		project = session.Project
	}
	if project != "" {
		fmt.Fprintf(os.Stderr, "lorri: unloaded %s\n", project)
	} else {
		fmt.Fprintf(os.Stderr, "lorri: unloaded\n")
	}
	return nil
}

// ambientEnv returns the current environment as a map.
func ambientEnv() map[string]string {
	env := make(map[string]string)
	for _, kv := range os.Environ() {
		idx := strings.IndexByte(kv, '=')
		if idx < 0 {
			continue
		}
		env[kv[:idx]] = kv[idx+1:]
	}
	return env
}

// applyChange converts one EnvChange into a ShellExport entry, applying
// prepend/append semantics against the ambient environment.
func applyChange(change EnvChange, ambient map[string]string, export direnv.ShellExport) {
	switch change.Op {
	case "pass":
		// Deliberately leave the ambient value untouched — no-op.
	case "unset":
		export.Remove(change.Name)
	case "set":
		export.Add(change.Name, change.Value)
	case "prepend":
		existing := ambient[change.Name]
		if existing == "" {
			export.Add(change.Name, change.Value)
		} else {
			export.Add(change.Name, change.Value+change.Sep+existing)
		}
	case "append":
		existing := ambient[change.Name]
		if existing == "" {
			export.Add(change.Name, change.Value)
		} else {
			export.Add(change.Name, existing+change.Sep+change.Value)
		}
	}
}

// printEnvSummary prints a compact human-readable summary of what changed
// to stderr, filtering out Nix build internals.
func printEnvSummary(lorriEnv *LorriEnv, prev map[string]*string, elapsed time.Duration) {
	var added, changed, removed []string
	for _, change := range lorriEnv.Env {
		if isNixInternal(change.Name) {
			continue
		}
		switch change.Op {
		case "unset":
			if prev[change.Name] != nil {
				removed = append(removed, change.Name)
			}
		case "set", "prepend", "append":
			if prev[change.Name] == nil {
				added = append(added, change.Name)
			} else {
				changed = append(changed, change.Name)
			}
		}
	}
	if len(added)+len(changed)+len(removed) == 0 {
		fmt.Fprintf(os.Stderr, "lorri: loaded [%s]\n", elapsed.Round(time.Millisecond))
		return
	}
	var parts []string
	if len(added) > 0 {
		parts = append(parts, "+"+strings.Join(added, " +"))
	}
	if len(changed) > 0 {
		parts = append(parts, "~"+strings.Join(changed, " ~"))
	}
	if len(removed) > 0 {
		parts = append(parts, "-"+strings.Join(removed, " -"))
	}
	fmt.Fprintf(os.Stderr, "lorri: loaded (%s) [%s]\n", strings.Join(parts, " "), elapsed.Round(time.Millisecond))
}

// opExportDirenvAdapter implements `lorri export direnv-adapter`.
//
// Emits watch_file lines + bash export/unset commands for direnv to eval.
// No session state management — direnv owns the env lifecycle here; lorri
// only provides the data. The user's .envrc should contain:
//
//	eval "$(lorri export direnv-adapter)"
func opExportDirenvAdapter() error {
	paths, err := mustInitPaths()
	if err != nil {
		return err
	}

	// Find the lorri project for the current directory.
	db, err := OpenLorriDB(paths.SQLiteDB.String())
	if err != nil {
		return fmt.Errorf("export direnv-adapter: open db: %w", err)
	}
	defer db.Close()

	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("export direnv-adapter: getwd: %w", err)
	}

	foundDBProj, err := db.FindRegisteredProjectForDir(cwd)
	if err != nil {
		return fmt.Errorf("export direnv-adapter: %w", err)
	}
	project := ""
	if foundDBProj != nil {
		project = foundDBProj.NixFile
	}

	socketPath := string(paths.DaemonSocketFile)

	if project == "" {
		// No registered project — just watch the socket.
		fmt.Fprintf(os.Stdout, "watch_file %q\n", socketPath)
		return nil
	}

	gcRootPath := gcRootPathForProject(paths.GCRootDir, NewShellNixProjectFile(mustAbsPath(project)))
	storePath, _ := os.Readlink(string(gcRootPath))
	envJSONPath := filepath.Join(storePath, "env.json")

	if storePath == "" || !fileExists(envJSONPath) {
		// Not yet built — watch socket so direnv re-evals when done.
		_ = opPingWithRebuild(paths, NewShellNixProjectFile(mustAbsPath(project)), RebuildAlways)
		fmt.Fprintf(os.Stdout, "watch_file %q\n", socketPath)
		fmt.Fprintln(os.Stderr, "lorri: has not completed an evaluation for this project yet")
		return nil
	}

	lorriEnv, err := readEnvJSON(envJSONPath)
	if err != nil {
		return fmt.Errorf("export direnv-adapter: %w", err)
	}

	ambient := ambientEnv()
	export := make(direnv.ShellExport)
	for _, change := range lorriEnv.Env {
		applyChange(change, ambient, export)
	}

	shellScript, err := direnv.Shells["bash"].Export(export)
	if err != nil {
		return fmt.Errorf("export direnv-adapter: render: %w", err)
	}

	// watch_file on socket + GC root so direnv re-evals when a new build lands.
	fmt.Fprintf(os.Stdout, "watch_file %q\nwatch_file %q\n%s\n",
		socketPath, string(gcRootPath), shellScript)

	_ = opPingWithRebuild(paths, NewShellNixProjectFile(mustAbsPath(project)), RebuildOnlyIfNotYetWatching)
	return nil
}

// isNixInternal returns true for variables that are Nix build internals and
// not meaningful to show in the summary.
func isNixInternal(name string) bool {
	if strings.HasPrefix(name, "NIX_") ||
		strings.HasPrefix(name, "orig") ||
		strings.HasPrefix(name, "__") {
		return true
	}
	switch name {
	case "stdenv", "builder", "system", "out", "outputs", "name",
		"buildInputs", "nativeBuildInputs", "propagatedBuildInputs",
		"propagatedNativeBuildInputs", "buildPhase", "phases",
		"preferLocalBuild", "allowSubstitutes", "doCheck", "doInstallCheck",
		"extraClosure", "origExtraClosure", "strictDeps",
		"depsBuildBuild", "depsBuildBuildPropagated",
		"depsBuildTarget", "depsBuildTargetPropagated",
		"depsHostHost", "depsHostHostPropagated",
		"depsTargetTarget", "depsTargetTargetPropagated",
		"SOURCE_DATE_EPOCH", "RUN_TIME_CLOSURE":
		return true
	}
	return false
}
