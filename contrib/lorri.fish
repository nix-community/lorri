# Fish completion for lorri
# Install to: ~/.config/fish/completions/lorri.fish
# or let Nix install it to $out/share/fish/vendor_completions.d/lorri.fish

# Disable file completion by default for lorri
complete -c lorri -f

# ---------------------------------------------------------------------------
# Helper conditions
# ---------------------------------------------------------------------------

function __lorri_no_subcommand
    not __fish_seen_subcommand_from \
        daemon direnv gc info watch unwatch prompt hook export internal help
end

function __lorri_seen_gc
    __fish_seen_subcommand_from gc
end

function __lorri_gc_no_subcommand
    __fish_seen_subcommand_from gc
    and not __fish_seen_subcommand_from info rm
end

function __lorri_seen_gc_rm
    __fish_seen_subcommand_from gc
    and __fish_seen_subcommand_from rm
end

function __lorri_seen_prompt
    __fish_seen_subcommand_from prompt
end

function __lorri_prompt_no_subcommand
    __fish_seen_subcommand_from prompt
    and not __fish_seen_subcommand_from default
end

function __lorri_seen_internal
    __fish_seen_subcommand_from internal
end

function __lorri_internal_no_subcommand
    __fish_seen_subcommand_from internal
    and not __fish_seen_subcommand_from ping_ stream-events_ generate-env_
end

# ---------------------------------------------------------------------------
# Top-level subcommands
# ---------------------------------------------------------------------------

complete -c lorri -n __lorri_no_subcommand -a daemon    -d 'Start the multi-project build daemon'
complete -c lorri -n __lorri_no_subcommand -a direnv    -d 'Emit direnv shell script (for use inside .envrc)'
complete -c lorri -n __lorri_no_subcommand -a gc        -d 'Garbage-collect lorri GC roots'
complete -c lorri -n __lorri_no_subcommand -a info      -d 'Show project and daemon status'
complete -c lorri -n __lorri_no_subcommand -a watch     -d 'Register the current project with lorri and start watching it'
complete -c lorri -n __lorri_no_subcommand -a unwatch   -d 'Stop watching the current project'
complete -c lorri -n __lorri_no_subcommand -a prompt    -d 'Generate a lorri status marker for your shell prompt'
complete -c lorri -n __lorri_no_subcommand -a hook      -d 'Print the shell hook to eval in your rc file'
complete -c lorri -n __lorri_no_subcommand -a export    -d 'Print shell export commands (called by the hook on each prompt)'
complete -c lorri -n __lorri_no_subcommand -a internal  -d 'Unstable plumbing commands for scripts and integrations'
complete -c lorri -n __lorri_no_subcommand -a help      -d 'Show help'

# ---------------------------------------------------------------------------
# daemon
# ---------------------------------------------------------------------------

complete -c lorri -n '__fish_seen_subcommand_from daemon' \
    -l extra-nix-options \
    -d 'JSON object with optional "builders" and "substituters" arrays' \
    -r

# ---------------------------------------------------------------------------
# direnv
# ---------------------------------------------------------------------------

complete -c lorri -n '__fish_seen_subcommand_from direnv' \
    -l shell-file -d 'Path to shell.nix (or similar)' -r -F
complete -c lorri -n '__fish_seen_subcommand_from direnv' \
    -l context -d 'Directory to resolve a flake from' -r -F
complete -c lorri -n '__fish_seen_subcommand_from direnv' \
    -l flake -d 'Flake installable descriptor (e.g. .#)' -r

# ---------------------------------------------------------------------------
# gc
# ---------------------------------------------------------------------------

complete -c lorri -n __lorri_gc_no_subcommand -a info -d 'List GC roots and whether their projects still exist'
complete -c lorri -n __lorri_gc_no_subcommand -a rm   -d 'Remove GC roots for gone or selected projects'
complete -c lorri -n __lorri_seen_gc          -l json -d 'Machine-readable JSON output'

# gc rm flags
complete -c lorri -n __lorri_seen_gc_rm \
    -l all -d 'Delete roots of all projects'
complete -c lorri -n __lorri_seen_gc_rm \
    -l older-than -d 'Delete roots older than this duration (e.g. 30d, 2m, 1y)' -r
complete -c lorri -n __lorri_seen_gc_rm \
    -l dry-run -d 'Only print what would be deleted'
complete -c lorri -n __lorri_seen_gc_rm \
    -l shell-file -d 'Delete root for this shell file (repeatable)' -r -F

# ---------------------------------------------------------------------------
# info
# ---------------------------------------------------------------------------

complete -c lorri -n '__fish_seen_subcommand_from info' \
    -l shell-file -d 'Path to shell.nix (required if no flake)' -r -F
complete -c lorri -n '__fish_seen_subcommand_from info' \
    -l context -d 'Directory to resolve a flake from' -r -F
complete -c lorri -n '__fish_seen_subcommand_from info' \
    -l flake -d 'Flake installable descriptor' -r

# ---------------------------------------------------------------------------
# prompt
# ---------------------------------------------------------------------------

complete -c lorri -n __lorri_prompt_no_subcommand \
    -a default -d "Print 'ℓ' if the cwd is inside a lorri-watched project"

complete -c lorri -n '__fish_seen_subcommand_from prompt; and __fish_seen_subcommand_from default' \
    -l include-leading-space -d 'Include a leading space before the prompt symbol'

# ---------------------------------------------------------------------------
# hook  — --how flag or shell name as positional argument
# ---------------------------------------------------------------------------

complete -c lorri -n '__fish_seen_subcommand_from hook' \
    -l how -d 'Print setup instructions for all supported shells'

complete -c lorri -n '__fish_seen_subcommand_from hook' -a bash    -d 'Bourne-again shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a zsh     -d 'Z shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a fish    -d 'Friendly interactive shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a elvish  -d 'Elvish shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a tcsh    -d 'TENEX C shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a murex   -d 'Murex shell'
complete -c lorri -n '__fish_seen_subcommand_from hook' -a pwsh    -d 'PowerShell 7+'

# ---------------------------------------------------------------------------
# export  — shell name as positional argument
# ---------------------------------------------------------------------------

complete -c lorri -n '__fish_seen_subcommand_from export' -a bash     -d 'Bourne-again shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a zsh      -d 'Z shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a fish     -d 'Friendly interactive shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a elvish   -d 'Elvish shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a tcsh     -d 'TENEX C shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a murex    -d 'Murex shell'
complete -c lorri -n '__fish_seen_subcommand_from export' -a pwsh     -d 'PowerShell 7+'
complete -c lorri -n '__fish_seen_subcommand_from export' -a vim      -d 'Vim (export only, no hook)'
complete -c lorri -n '__fish_seen_subcommand_from export' -a json     -d 'JSON output (export only, no hook)'
complete -c lorri -n '__fish_seen_subcommand_from export' -a systemd  -d 'systemd EnvironmentFile format (export only, no hook)'

# ---------------------------------------------------------------------------
# internal
# ---------------------------------------------------------------------------

complete -c lorri -n __lorri_internal_no_subcommand \
    -a ping_          -d 'Tell the daemon to watch the current project'
complete -c lorri -n __lorri_internal_no_subcommand \
    -a stream-events_ -d 'Stream build events from the daemon as JSON lines'
complete -c lorri -n __lorri_internal_no_subcommand \
    -a generate-env_  -d 'Write $out/env.json from the current Nix build environment'

# internal ping_ flags
complete -c lorri -n '__fish_seen_subcommand_from internal; and __fish_seen_subcommand_from ping_' \
    -l shell-file -d 'Path to shell.nix (or similar)' -r -F
complete -c lorri -n '__fish_seen_subcommand_from internal; and __fish_seen_subcommand_from ping_' \
    -l context -d 'Directory to resolve a flake from' -r -F
complete -c lorri -n '__fish_seen_subcommand_from internal; and __fish_seen_subcommand_from ping_' \
    -l flake -d 'Flake installable descriptor (e.g. .#)' -r

# internal stream-events_ flags
complete -c lorri -n '__fish_seen_subcommand_from internal; and __fish_seen_subcommand_from stream-events_' \
    -l kind -d 'Event kind to stream' -r -a 'live\tLive events only  snapshot\tSnapshot of current state  all\tBoth snapshot and live'

# internal generate-env_ takes a file argument
complete -c lorri -n '__fish_seen_subcommand_from internal; and __fish_seen_subcommand_from generate-env_' \
    -F
