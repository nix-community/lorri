# Elvish completion for lorri
# Load from your rc.elv (~/.config/elvish/rc.elv) with:
#   eval (slurp < ~/.nix-profile/share/elvish/lib/lorri-completion.elv)

# Descriptions shown alongside candidates in the completion menu via
# edit:complex-candidate &display=...

var hook-shells = [
    [&stem=bash    &desc='Bourne-again shell']
    [&stem=zsh     &desc='Z shell']
    [&stem=fish    &desc='Friendly interactive shell']
    [&stem=elvish  &desc='Elvish shell']
    [&stem=tcsh    &desc='TENEX C shell']
    [&stem=murex   &desc='Murex shell']
    [&stem=pwsh    &desc='PowerShell 7+']
]

var export-shells = [
    [&stem=bash     &desc='Bourne-again shell']
    [&stem=zsh      &desc='Z shell']
    [&stem=fish     &desc='Friendly interactive shell']
    [&stem=elvish   &desc='Elvish shell']
    [&stem=tcsh     &desc='TENEX C shell']
    [&stem=murex    &desc='Murex shell']
    [&stem=pwsh     &desc='PowerShell 7+']
    [&stem=vim      &desc='Vim (export only, no hook)']
    [&stem=json     &desc='JSON output (export only, no hook)']
    [&stem=systemd  &desc='systemd EnvironmentFile format (export only, no hook)']
]

fn -candidates {|items|
    each {|m|
        edit:complex-candidate $m[stem] &display=(
            styled (styled-segment $m[stem] &bold)(styled-segment ' ('$m[desc]')' &dim)
        )
    } $items
}

fn -flag-candidates {|flags|
    # flags is a list of [&flag=... &desc=...] maps
    each {|m|
        edit:complex-candidate $m[flag] &display=(
            styled (styled-segment $m[flag] &bold)(styled-segment ' ('$m[desc]')' &dim)
        )
    } $flags
}

set edit:completion:arg-completer[lorri] = {|@args|
    var n = (count $args)

    # args[0] = "lorri", args[1] = subcommand (or ""), args[2] = ...
    if (== $n 2) {
        # Completing the subcommand name
        -candidates [
            [&stem=daemon    &desc='Start the multi-project build daemon']
            [&stem=direnv    &desc='Emit direnv shell script (for use inside .envrc)']
            [&stem=gc        &desc='Garbage-collect lorri GC roots']
            [&stem=info      &desc='Show project and daemon status']
            [&stem=init      &desc='Write bootstrap shell.nix to the current directory']
            [&stem=prompt    &desc='Generate a lorri status marker for your shell prompt']
            [&stem=hook      &desc='Print the shell hook to eval in your rc file']
            [&stem=export    &desc='Print shell export commands (called by the hook on each prompt)']
            [&stem=internal  &desc='Unstable plumbing commands for scripts and integrations']
            [&stem=help      &desc='Show help']
        ]
        return
    }

    var subcmd = $args[1]

    if (== $n 3) {
        # Completing the first argument/flag of the subcommand
        if (eq $subcmd daemon) {
            -flag-candidates [
                [&flag=--extra-nix-options &desc='JSON with optional "builders" and "substituters" arrays']
            ]
        } elif (eq $subcmd direnv) {
            -flag-candidates [
                [&flag=--shell-file  &desc='Path to shell.nix (or similar)']
                [&flag=--context     &desc='Directory to resolve a flake from']
                [&flag=--flake       &desc='Flake installable descriptor (e.g. .#)']
            ]
        } elif (eq $subcmd gc) {
            -candidates [
                [&stem=info  &desc='List GC roots and whether their projects still exist']
                [&stem=rm    &desc='Remove GC roots for gone or selected projects']
            ]
            -flag-candidates [[&flag=--json &desc='Machine-readable JSON output']]
        } elif (eq $subcmd info) {
            -flag-candidates [
                [&flag=--shell-file  &desc='Path to shell.nix (required if no flake)']
                [&flag=--context     &desc='Directory to resolve a flake from']
                [&flag=--flake       &desc='Flake installable descriptor']
            ]
        } elif (eq $subcmd prompt) {
            -candidates [
                [&stem=default  &desc="Print 'ℓ' if the cwd is inside a lorri-watched project"]
            ]
        } elif (eq $subcmd hook) {
            -candidates $hook-shells
        } elif (eq $subcmd export) {
            -candidates $export-shells
        } elif (eq $subcmd internal) {
            -candidates [
                [&stem=ping_           &desc='Tell the daemon to watch the current project']
                [&stem=stream-events_  &desc='Stream build events from the daemon as JSON lines']
                [&stem=generate-env_   &desc='Write $out/env.json from the current Nix build environment']
            ]
        }
        return
    }

    # n >= 4: completing flags/args deeper in the tree
    if (eq $subcmd gc) {
        var gc-sub = $args[2]
        if (eq $gc-sub rm) {
            -flag-candidates [
                [&flag=--all         &desc='Delete roots of all projects']
                [&flag=--older-than  &desc='Delete roots older than this duration (e.g. 30d, 2m, 1y)']
                [&flag=--dry-run     &desc='Only print what would be deleted']
                [&flag=--shell-file  &desc='Delete root for this shell file (repeatable)']
                [&flag=--json        &desc='Machine-readable JSON output']
            ]
            # Re-enable file completion after --shell-file
            var prev = $args[(- $n 2)]
            if (eq $prev --shell-file) {
                edit:complete-filename $args[-1]
            }
        }
    } elif (eq $subcmd prompt) {
        var prompt-sub = $args[2]
        if (eq $prompt-sub default) {
            -flag-candidates [
                [&flag=--include-leading-space  &desc='Include a leading space before the prompt symbol']
            ]
        }
    } elif (eq $subcmd direnv) {
        var prev = $args[(- $n 2)]
        if (or (eq $prev --shell-file) (eq $prev --context)) {
            edit:complete-filename $args[-1]
        }
    } elif (eq $subcmd info) {
        var prev = $args[(- $n 2)]
        if (or (eq $prev --shell-file) (eq $prev --context)) {
            edit:complete-filename $args[-1]
        }
    } elif (eq $subcmd internal) {
        var int-sub = $args[2]
        if (eq $int-sub ping_) {
            -flag-candidates [
                [&flag=--shell-file  &desc='Path to shell.nix (or similar)']
                [&flag=--context     &desc='Directory to resolve a flake from']
                [&flag=--flake       &desc='Flake installable descriptor (e.g. .#)']
            ]
            var prev = $args[(- $n 2)]
            if (or (eq $prev --shell-file) (eq $prev --context)) {
                edit:complete-filename $args[-1]
            }
        } elif (eq $int-sub stream-events_) {
            var prev = $args[(- $n 2)]
            if (eq $prev --kind) {
                -candidates [
                    [&stem=live      &desc='Live events only']
                    [&stem=snapshot  &desc='Snapshot of current state']
                    [&stem=all       &desc='Both snapshot and live events']
                ]
            } else {
                -flag-candidates [
                    [&flag=--kind  &desc='Event kind to stream (live, snapshot, all)']
                ]
            }
        } elif (eq $int-sub generate-env_) {
            edit:complete-filename $args[-1]
        }
    }
}
