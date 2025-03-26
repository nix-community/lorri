lorri(1)

# NAME

lorri — nix-shell replacement for project development

# SYNOPSIS

*lorri daemon* [*--extra-nix-options* _json_]++
*lorri direnv* [*--shell-file* _shell.nix_]++
*lorri info* *--shell-file* _shell.nix_++
*lorri init*++
*lorri shell* [*--cached*] [*--shell-file* _shell.nix_]

# DESCRIPTION

*lorri* is a nix-shell replacement for project development.
*lorri* is based around fast _direnv_(1) integration for robust
CLI and editor integration.

*lorri* is about experimenting with and improving the
developer's experience with Nix. A particular focus is managing
your project's external dependencies, editor integration,
and quick feedback.

*lorri* provides a daemon that automatically builds project
environment whenever their nix files change, and the
_direnv_(1) integration instantly loads the project environment,
in your shell and in your editor.

*lorri* supports Linux and macOS.

This man page documents the available commands, but we also
have setup and installation instructions which you can find
on the project page: https://github.com/nix-community/lorri/.

## Subcommands

*lorri daemon* [*--extra-nix-options* _json_]

	Start the *lorri* daemon. It is required for most other
	commands to function. The daemon keeps track of the projects
	watched and will rebuild whenever a dependency of your
	project’s _shell.nix_ file changes.
	This includes _nix files_, _source files_ as well as _non-nix
	source files_, like for example json files loaded via
	‘builtins.fromJSON’, ‘builtins.readFile’, &c.

	The *--extra-nix-options* flag takes a json description of
	nix options to overwrite. Only a subset is supported.
	See ‘lorri daemon --help’ for a description of the supported
	options.

*lorri direnv* [*--shell-file* _shell.nix_]

	Print a piece of _direnv_(1) bash code, intended to be sourced
	in the _.envrc_ of your project. Source it by adding

	```
	eval "$(lorri direnv)"
	```

	to your project’s _.envrc_.
	It will then cause *direnv* to reload your environment whenever
	*lorri* finishes a rebuild.

	_shell.nix_ is the file built and watched by *lorri*.
	Use the *--shell-file* flag to overwrite the file name:

	```
	eval "$(lorri direnv --shell-file foo.nix)"
	```

	The path is relative to the location of your _.envrc_.

*lorri info* *--shell-file* _shell.nix_

	Show project information for the given shell file.

*lorri init*

	Bootstrap a *lorri* project. The following files are created
	if they do not exist yet:

	- _shell.nix_: Minimal nix shell file
	- _.envrc_: A direnv config with the ‘lorri direnv’ call.

*lorri shell* [*--cached*] [*--shell-file* _shell.nix_]

	Open a project shell. This is essentially a beefed-up
	_nix-shell_(1), with the following improvements:

	- It will use your interactive shell instead of _bash_(1)
	  (_bash_(1), _fish_(1) and _zsh_(1) are supported currently).
	- If given the *--cached* flag, it will use the environment
	  that *lorri* built most recently. This means the project shell
	  starts up instantly, similar to ‘lorri direnv’.

	*--shell-file* opens a shell file for the given _shell.nix_
	instead of the one in the current directory.

# RELATED WORK

direnv’s ‘use nix’
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
function to convert your project’s ‘.cabal’ file to a
nix expression on the fly, and you change the ‘.cabal’ file,
*lorri* will not notice.
