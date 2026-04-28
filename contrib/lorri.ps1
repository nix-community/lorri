# PowerShell completion for lorri
# Requires PowerShell 7.2+
#
# Add to your $PROFILE:
#   . ~/.nix-profile/share/powershell/completions/lorri.ps1

Register-ArgumentCompleter -Native -CommandName lorri -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    # Parse command elements, skipping the command name itself
    $elements = $commandAst.CommandElements
    $n = $elements.Count

    # Extract subcommand and sub-subcommand (first and second non-flag words)
    $subcmd  = $null
    $subcmd2 = $null
    for ($i = 1; $i -lt $n; $i++) {
        $w = $elements[$i].ToString()
        if (-not $w.StartsWith('-')) {
            if ($null -eq $subcmd)       { $subcmd  = $w }
            elseif ($null -eq $subcmd2)  { $subcmd2 = $w }
        }
    }

    # The word immediately before the cursor (for flag-value completion)
    $prev = if ($n -ge 2) { $elements[$n - 1].ToString() } else { '' }
    # If the cursor is on a new word, prev is the last completed element
    if ($wordToComplete -ne '') { $prev = if ($n -ge 3) { $elements[$n - 2].ToString() } else { '' } }

    function Cand {
        param([string]$text, [string]$desc)
        [System.Management.Automation.CompletionResult]::new(
            $text,   # completionText
            $text,   # listItemText
            'ParameterValue',
            $desc    # toolTip
        )
    }

    function CandFilter {
        param([string]$text, [string]$desc)
        if ($text -like "$wordToComplete*") { Cand $text $desc }
    }

    # --shell-file / --context: let PowerShell do path completion
    if ($prev -in '--shell-file', '--context') {
        # Return nothing — PowerShell falls back to path completion automatically
        return
    }

    switch ($subcmd) {
        $null {
            CandFilter 'daemon'    'Start the multi-project build daemon'
            CandFilter 'direnv'    'Emit direnv shell script (for use inside .envrc)'
            CandFilter 'gc'        'Garbage-collect lorri GC roots'
            CandFilter 'info'      'Show project and daemon status'
            CandFilter 'init'      'Write bootstrap shell.nix to the current directory'
            CandFilter 'prompt'    'Generate a lorri status marker for your shell prompt'
            CandFilter 'hook'      'Print the shell hook to eval in your rc file'
            CandFilter 'export'    'Print shell export commands (called by the hook on each prompt)'
            CandFilter 'internal'  'Unstable plumbing commands for scripts and integrations'
            CandFilter 'help'      'Show help'
        }

        'daemon' {
            CandFilter '--extra-nix-options' 'JSON with optional "builders" and "substituters" arrays'
        }

        'direnv' {
            CandFilter '--shell-file'  'Path to shell.nix (or similar)'
            CandFilter '--context'     'Directory to resolve a flake from'
            CandFilter '--flake'       'Flake installable descriptor (e.g. .#)'
        }

        'gc' {
            if ($null -eq $subcmd2) {
                CandFilter 'info'   'List GC roots and whether their projects still exist'
                CandFilter 'rm'     'Remove GC roots for gone or selected projects'
                CandFilter '--json' 'Machine-readable JSON output'
            } else {
                switch ($subcmd2) {
                    'info' {
                        CandFilter '--json' 'Machine-readable JSON output'
                    }
                    'rm' {
                        CandFilter '--all'         'Delete roots of all projects'
                        CandFilter '--older-than'  'Delete roots older than this duration (e.g. 30d, 2m, 1y)'
                        CandFilter '--dry-run'     'Only print what would be deleted'
                        CandFilter '--shell-file'  'Delete root for this shell file (repeatable)'
                        CandFilter '--json'        'Machine-readable JSON output'
                    }
                }
            }
        }

        'info' {
            CandFilter '--shell-file'  'Path to shell.nix (required if no flake)'
            CandFilter '--context'     'Directory to resolve a flake from'
            CandFilter '--flake'       'Flake installable descriptor'
        }

        'init' {
            # No flags or arguments
        }

        'prompt' {
            if ($null -eq $subcmd2) {
                CandFilter 'default' "Print 'l' if the cwd is inside a lorri-watched project"
            } elseif ($subcmd2 -eq 'default') {
                CandFilter '--include-leading-space' 'Include a leading space before the prompt symbol'
            }
        }

        'hook' {
            if ($null -eq $subcmd2) {
                CandFilter 'bash'    'Bourne-again shell'
                CandFilter 'zsh'     'Z shell'
                CandFilter 'fish'    'Friendly interactive shell'
                CandFilter 'elvish'  'Elvish shell'
                CandFilter 'tcsh'    'TENEX C shell'
                CandFilter 'murex'   'Murex shell'
                CandFilter 'pwsh'    'PowerShell 7+'
            }
        }

        'export' {
            if ($null -eq $subcmd2) {
                CandFilter 'bash'     'Bourne-again shell'
                CandFilter 'zsh'      'Z shell'
                CandFilter 'fish'     'Friendly interactive shell'
                CandFilter 'elvish'   'Elvish shell'
                CandFilter 'tcsh'     'TENEX C shell'
                CandFilter 'murex'    'Murex shell'
                CandFilter 'pwsh'     'PowerShell 7+'
                CandFilter 'vim'      'Vim (export only, no hook)'
                CandFilter 'json'     'JSON output (export only, no hook)'
                CandFilter 'systemd'  'systemd EnvironmentFile format (export only, no hook)'
            }
        }

        'internal' {
            if ($null -eq $subcmd2) {
                CandFilter 'ping_'           'Tell the daemon to watch the current project'
                CandFilter 'stream-events_'  'Stream build events from the daemon as JSON lines'
                CandFilter 'generate-env_'   'Write $out/env.json from the current Nix build environment'
            } else {
                switch ($subcmd2) {
                    'ping_' {
                        CandFilter '--shell-file'  'Path to shell.nix (or similar)'
                        CandFilter '--context'     'Directory to resolve a flake from'
                        CandFilter '--flake'       'Flake installable descriptor (e.g. .#)'
                    }
                    'stream-events_' {
                        if ($prev -eq '--kind') {
                            CandFilter 'live'      'Live events only'
                            CandFilter 'snapshot'  'Snapshot of current state'
                            CandFilter 'all'       'Both snapshot and live events'
                        } else {
                            CandFilter '--kind' 'Event kind to stream (live, snapshot, all)'
                        }
                    }
                    'generate-env_' {
                        # File argument — fall through to path completion
                    }
                }
            }
        }

        'help' {
            CandFilter 'daemon'    'Start the multi-project build daemon'
            CandFilter 'direnv'    'Emit direnv shell script (for use inside .envrc)'
            CandFilter 'gc'        'Garbage-collect lorri GC roots'
            CandFilter 'info'      'Show project and daemon status'
            CandFilter 'init'      'Write bootstrap shell.nix to the current directory'
            CandFilter 'prompt'    'Generate a lorri status marker for your shell prompt'
            CandFilter 'hook'      'Print the shell hook to eval in your rc file'
            CandFilter 'export'    'Print shell export commands (called by the hook on each prompt)'
            CandFilter 'internal'  'Unstable plumbing commands for scripts and integrations'
        }
    }
}
