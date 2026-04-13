package main

// op_shell: build the project environment and exec into a shell.
// op_start_user_shell: exec into the user's shell with the lorri env.
// Mirrors src/ops.rs op_shell(), bash_cmd(), op_start_user_shell(), shell_cmd().

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
)

// opShell builds the project environment and spawns a shell inside it.
// Mirrors op_shell().
func opShell(paths *Paths, projectFile ProjectFile, rtc string, cached bool) error {
	shellEnv := os.Getenv("SHELL")
	if shellEnv == "" {
		return fmt.Errorf("`lorri shell` requires the SHELL environment variable to be set")
	}

	gcRootPath := gcRootPathForProject(paths.GCRootDir, projectFile)
	gcRootExists := fileExists(gcRootPath.String())

	var outputPath BuildOutputPath
	if cached {
		if !gcRootExists {
			return fmt.Errorf("project has not previously been built successfully")
		}
		outputPath = BuildOutputPath{ShellGCRoot: gcRootPath.String()}
	} else {
		// Build the environment.
		fmt.Fprint(os.Stderr, "lorri: building environment")
		result, err := buildForShell(paths, projectFile, rtc)
		fmt.Fprintln(os.Stderr, ". done")
		if err != nil {
			if gcRootExists {
				return fmt.Errorf("build failed. Hint: try `lorri shell --cached` to use "+
					"the most recently built environment.\nBuild error: %w", err)
			}
			return fmt.Errorf("build failed. No cached environment available.\nBuild error: %w", err)
		}
		outputPath = result
	}

	// Write the bash init script to CAS.
	cas, err := NewCAS(paths.CASDir)
	if err != nil {
		return fmt.Errorf("shell: init CAS: %w", err)
	}
	initContent := fmt.Sprintf("\nEVALUATION_ROOT=%q\n\n%s", outputPath.ShellGCRoot, envrcBash)
	initFile, err := cas.FileFromString(initContent)
	if err != nil {
		return fmt.Errorf("shell: write init file: %w", err)
	}

	// Resolve bash from the runtime closure.
	bashPath, err := bashFromRTC(rtc)
	if err != nil {
		return fmt.Errorf("shell: resolve bash: %w", err)
	}

	nixFile := nixFilePathForProject(projectFile)

	// Find our own executable path.
	lorriExe, err := os.Executable()
	if err != nil {
		return fmt.Errorf("shell: resolve lorri path: %w", err)
	}

	// Spawn: bash with BASH_ENV set to our init script, which then execs
	// `lorri internal start-user-shell`.
	cmd := exec.Command(bashPath)
	cmd.Env = append(os.Environ(), "BASH_ENV="+initFile.String())
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Args = []string{
		bashPath,
		"-c",
		`exec "$1" internal start-user-shell --shell-path="$2" --shell-file="$3"`,
		"--",
		lorriExe,
		shellEnv,
		nixFile,
	}
	if err := cmd.Run(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return fmt.Errorf("cannot run lorri shell: failed to execute internal shell command (error: %v)", exitErr)
		}
		return fmt.Errorf("cannot run lorri shell: %w", err)
	}
	return nil
}

// buildForShell runs a single build for the shell op.
func buildForShell(paths *Paths, projectFile ProjectFile, rtc string) (BuildOutputPath, error) {
	cas, err := NewCAS(paths.CASDir)
	if err != nil {
		return BuildOutputPath{}, err
	}
	cfg := BuildLoopConfig{
		ProjectFile:    projectFile,
		NixFile:        nixFilePathForProject(projectFile),
		CAS:            cas,
		Opts:           EmptyNixOptions(),
		RunTimeClosure: rtc,
		GCRootDir:      paths.GCRootDir,
	}
	bl, err := NewBuildLoop(cfg)
	if err != nil {
		return BuildOutputPath{}, err
	}
	return bl.Once()
}

// bashFromRTC resolves the bash binary path from the lorri runtime closure.
// Runs: nix-instantiate --eval --json --expr "(import <rtc>).path"
func bashFromRTC(rtc string) (string, error) {
	out, err := exec.Command(
		"nix-instantiate", "--eval", "--json", "--expr",
		fmt.Sprintf("(import %s).path", rtc),
	).Output()
	if err != nil {
		return "", fmt.Errorf("nix-instantiate for bash path: %w", err)
	}
	var storePath string
	if err := json.Unmarshal([]byte(strings.TrimSpace(string(out))), &storePath); err != nil {
		return "", fmt.Errorf("parse bash store path: %w (output: %q)", err, string(out))
	}
	return filepath.Join(storePath, "bin", "bash"), nil
}

// opStartUserShell execs into the user's shell with lorri's prompt customisation.
// Mirrors op_start_user_shell() + shell_cmd().
// This function never returns on success (it execs into the shell).
func opStartUserShell(shellPath, nixFile string, cas *CAS) error {
	shellName := filepath.Base(shellPath)

	// Build the exec args and env.
	args := []string{shellPath}
	env := os.Environ()

	switch shellName {
	case "bash":
		rcfile, err := cas.FileFromString(`
[ -e /etc/bash.bashrc ] && . /etc/bash.bashrc
[ -e ~/.bashrc ] && . ~/.bashrc
PS1="(lorri) $PS1"
`)
		if err != nil {
			return fmt.Errorf("start-user-shell: write bash rcfile: %w", err)
		}
		args = append(args, "--rcfile", rcfile.String())

	case "zsh":
		// Zsh init: create a temp dir with .zshrc that sets up the prompt.
		tmpDir, err := os.MkdirTemp("", "lorri-zsh-*")
		if err != nil {
			return fmt.Errorf("start-user-shell: create temp dir: %w", err)
		}
		// Intentionally NOT deferred — we exec, so this dir persists until reboot.
		zshrc := `
unset RCS # disable automatic sourcing of startup scripts

# reset ZDOTDIR
if [ ! -z ${ZDOTDIR_BEFORE} ]; then
    ZDOTDIR="${ZDOTDIR_BEFORE}"
else
    unset ZDOTDIR
fi

ZDOTDIR_OR_HOME="${ZDOTDIR:-${HOME}}"
test -f "$ZDOTDIR_OR_HOME/.zshenv" && . "$ZDOTDIR_OR_HOME/.zshenv"
test -f "/etc/zshrc"               && . "/etc/zshrc"
ZDOTDIR_OR_HOME="${ZDOTDIR:-${HOME}}"
test -f "$ZDOTDIR_OR_HOME/.zshrc"  && . "$ZDOTDIR_OR_HOME/.zshrc"

PS1="(lorri) ${PS1}"
`
		if err := os.WriteFile(filepath.Join(tmpDir, ".zshrc"), []byte(zshrc), 0o644); err != nil {
			return fmt.Errorf("start-user-shell: write .zshrc: %w", err)
		}
		// Pass current ZDOTDIR as ZDOTDIR_BEFORE so the .zshrc can restore it.
		if zdotdir := os.Getenv("ZDOTDIR"); zdotdir != "" {
			env = setEnv(env, "ZDOTDIR_BEFORE", zdotdir)
		}
		env = setEnv(env, "ZDOTDIR", tmpDir)

	default:
		fmt.Fprintf(os.Stderr,
			"lorri: we can only open shells for bash and zsh at the moment, "+
				"try using our direnv support instead. It supports as many shells as direnv does!\n")
	}

	// Exec into the shell — replaces the current process.
	if err := syscall.Exec(shellPath, args, env); err != nil {
		return fmt.Errorf("failed to exec into %q: %w", shellPath, err)
	}
	panic("unreachable")
}

// setEnv sets (or replaces) a KEY=VALUE pair in an environment slice.
func setEnv(env []string, key, value string) []string {
	prefix := key + "="
	for i, e := range env {
		if strings.HasPrefix(e, prefix) {
			env[i] = prefix + value
			return env
		}
	}
	return append(env, prefix+value)
}
