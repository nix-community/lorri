// Package direnv vendors the shell hook and export machinery from
// https://github.com/direnv/direnv (MIT licence).
//
// The gzenv shell is omitted (it is an internal direnv format).
// The systemd shell is omitted (it uses internal direnv logging helpers).
// All other direnv shells are included unchanged except:
//   - package renamed from cmd to direnv
//   - hook strings updated to call `lorri export` instead of `direnv export`
package direnv_vendor

import (
	"bytes"
	"fmt"
	"text/template"
)

// Shell is the interface that represents the interaction with the host shell.
type Shell interface {
	// Hook returns a snippet that the user evals into their shell config to
	// install lorri as a prompt hook.
	Hook() (string, error)

	// Export outputs the ShellExport as an evaluatable string on the host shell.
	Export(e ShellExport) (string, error)
}

// ShellExport represents environment variables to add and remove on the host
// shell. A nil value means unset; a non-nil pointer means set to that value.
type ShellExport map[string]*string

// Add records a variable to be set.
func (e ShellExport) Add(key, value string) {
	e[key] = &value
}

// Remove records a variable to be unset.
func (e ShellExport) Remove(key string) {
	e[key] = nil
}

// Shells is the map of supported shell names to Shell implementations.
var Shells = map[string]Shell{
	"bash":    Bash,
	"zsh":     Zsh,
	"fish":    Fish,
	"elvish":  Elvish,
	"tcsh":    Tcsh,
	"murex":   Murex,
	"pwsh":    Pwsh,
	"vim":     Vim,
	"json":    JSON,
	"systemd": Systemd,
}

// RenderHook renders a hook template substituting SelfPath.
// Hook templates use {{.SelfPath}} as the placeholder.
func RenderHook(tmplStr, selfPath string) (string, error) {
	tmpl, err := template.New("hook").Delims("{{", "}}").Parse(tmplStr)
	if err != nil {
		return "", fmt.Errorf("RenderHook: %w", err)
	}
	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, struct{ SelfPath string }{selfPath}); err != nil {
		return "", fmt.Errorf("RenderHook: %w", err)
	}
	return buf.String(), nil
}

// Env is a map of environment variable names to values (used by Dump).
type Env map[string]string
