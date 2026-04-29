lorri(1)

# NAME

lorri — nix-shell replacement for project development

# SYNOPSIS

*lorri daemon* [*--extra-nix-options* _json_]

*lorri hook* [*--how*] _shell_++
*lorri export* _shell_

*lorri watch*++
*lorri unwatch*

*lorri info* {*--shell-file* _shell.nix_ | *--flake* _installable_} [*--context* _dir_]++
*lorri prompt default* [*--include-leading-space*]++
*lorri gc* [*--json*] *info*++
*lorri gc* [*--json*] *rm* [*--all*] [*--older-than* _duration_] [*--dry-run*] [*--shell-file* _shell.nix_]

*lorri direnv* [*--shell-file* _shell.nix_] [*--flake* _installable_] [*--context* _dir_] _(deprecated)_

# DESCRIPTION

*lorri* is a nix-shell replacement for project development.
It provides a daemon that automatically rebuilds your project
environment whenever its nix files change, and a shell hook
that instantly loads the updated environment on each prompt.

*lorri* supports Linux and macOS.

This man page documents the available commands, but we also
have setup and installation instructions which you can find
on the project page: https://github.com/nix-community/lorri/.

## Subcommands

## Daemon

*lorri daemon* [*--extra-nix-options* _json_]

	Start the *lorri* daemon. It is required for most other
	commands to function. The daemon keeps track of the projects
	watched and will rebuild whenever a dependency of your
	project's _shell.nix_ file changes.
	This includes _nix files_, _source files_ as well as _non-nix
	source files_, like for example json files loaded via
	'builtins.fromJSON', 'builtins.readFile', &c.

	The *--extra-nix-options* flag takes a json description of
	nix options to overwrite. Only a subset is supported.
	See 'lorri daemon --help' for a description of the supported
	options.

## Shell hook

*lorri hook* [*--how*] _shell_

	Print a shell hook snippet to eval in your shell's rc file so
	that lorri updates your environment on each prompt. Supported
	shells: _bash_, _zsh_, _fish_, _elvish_, _tcsh_, _murex_, _pwsh_.

	```
	# bash (~/.bashrc):
	eval "$(lorri hook bash)"

	# zsh (~/.zshrc):
	eval "$(lorri hook zsh)"

	# fish (~/.config/fish/config.fish):
	lorri hook fish | source
	```

	Pass *--how* to print setup instructions for all supported shells
	instead of emitting a hook snippet.

*lorri export* _shell_

	Print shell export commands reflecting the current lorri
	environment. This is called automatically by the hook installed
	via *lorri hook* on each prompt; you do not normally need to run
	it directly. Supported shells are the same as for *lorri hook*,
	plus _vim_, _json_, and _systemd_.

## Project management

*lorri watch*

	Register the project in the current directory with lorri and start
	watching it. Auto-detects _shell.nix_, _flake.nix_, or _default.nix_,
	preferring _shell.nix_ first. If the project is already registered a
	message is printed and the command exits successfully.

	If no nix file is found, lorri will suggest how to create one:

	```
	nix flake init                       # minimal flake
	nix flake init -t templates#devShell # with a devShell template
	```

*lorri unwatch*

	Stop watching the project in the current directory. Removes the
	project's GC root and its database entry. The current shell's
	environment will be reverted on the next prompt; other shells with
	the project loaded will revert when the user leaves the project
	directory.

## Status

*lorri info* {*--shell-file* _shell.nix_ | *--flake* _installable_} [*--context* _dir_]

	Show project and daemon status for the given project. Either
	*--shell-file* or *--flake* is required.

*lorri prompt default* [*--include-leading-space*]

	Print _ℓ_ if the current directory is inside a lorri-watched
	project, for use in shell prompt strings. Pass
	*--include-leading-space* to prepend a space.

## Maintenance

*lorri gc* [*--json*] *info*

	List all GC roots lorri has created and whether their project
	directories still exist. Pass *--json* for machine-readable output.

*lorri gc* [*--json*] *rm* [*--all*] [*--older-than* _duration_] [*--dry-run*] [*--shell-file* _shell.nix_]

	Remove GC roots. By default only roots whose project directory
	no longer exists are removed. Use *--all* to remove every root,
	*--older-than* to remove roots older than a duration (e.g. _30d_,
	_2m_, _1y_), or *--shell-file* (repeatable) to target specific
	projects. *--dry-run* prints what would be removed without acting.

## Deprecated

*lorri direnv* [*--shell-file* _shell.nix_] [*--flake* _installable_] [*--context* _dir_]

	Deprecated. Print a _direnv_(1) snippet for use in your project's
	_.envrc_. Kept for backwards compatibility; new projects should use
	*lorri hook* instead. A migration notice is printed to stderr on
	each invocation.

# RELATED WORK

direnv's 'use nix'
	Built-in support for watching a nix file with direnv.
	Will block on the evaluation of the _shell.nix_.
	If a dependency of the _shell.nix_ changes, direnv will
	not pick it up, so you have to manually add them to the
	watch list with *watch_file*. See _direnv-stdlib_(1).

_shadowenv_(1)
	In spirit an alternative to _direnv_(1), it defines
	a domain-specific S-expression format, _shadowlisp_(5),
	to specify environment changes. In the future, we might
	want to provide a way to generate this format.

# SEE ALSO

_direnv_(1), _direnv-stdlib_(1), _nix-instantiate_(1),
_nix-shell_(1), _nix-store_(1), _shadowenv_(1), _shadowlisp_(5)

# KNOWN ISSUES

Source files referenced by _Import from Derivation (IfD)_
are not picked up. So for example if you use the *callCabal2nix*
function to convert your project's '.cabal' file to a
nix expression on the fly, and you change the '.cabal' file,
*lorri* will not notice.
