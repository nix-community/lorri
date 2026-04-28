#compdef lorri
# Zsh completion for lorri
# Install to: $fpath — e.g. /usr/share/zsh/site-functions/_lorri
# or ~/.local/share/zsh/site-functions/_lorri (add that dir to $fpath)

_lorri() {
    local context state state_descr line
    typeset -A opt_args

    local -a subcommands
    subcommands=(
        'daemon:start the multi-project build daemon'
        'direnv:emit direnv shell script (for use inside .envrc)'
        'gc:garbage-collect lorri GC roots'
        'info:show project and daemon status'
        'init:write bootstrap shell.nix to the current directory'
        'prompt:generate a lorri status marker for your shell prompt'
        'hook:print the shell hook to eval in your shell rc file'
        'export:print shell export commands (called by the hook on each prompt)'
        'internal:unstable plumbing commands for scripts and integrations'
        'help:show help'
    )

    local -a hook_shells
    hook_shells=(
        'bash:Bourne-again shell'
        'zsh:Z shell'
        'fish:friendly interactive shell'
        'elvish:Elvish shell'
        'tcsh:TENEX C shell'
        'murex:Murex shell'
        'pwsh:PowerShell 7+'
    )

    local -a export_shells
    export_shells=(
        'bash:Bourne-again shell'
        'zsh:Z shell'
        'fish:friendly interactive shell'
        'elvish:Elvish shell'
        'tcsh:TENEX C shell'
        'murex:Murex shell'
        'pwsh:PowerShell 7+'
        'vim:Vim (export only, no hook)'
        'json:JSON output (export only, no hook)'
        'systemd:systemd EnvironmentFile format (export only, no hook)'
    )

    _arguments -C \
        '1:subcommand:->subcmd' \
        '*::args:->args' \
        && return

    case $state in
        subcmd)
            _describe 'lorri subcommand' subcommands
            ;;

        args)
            case $line[1] in
                daemon)
                    _arguments \
                        '--extra-nix-options[JSON object with optional builders and substituters arrays]:json object'
                    ;;

                direnv)
                    _arguments \
                        '--shell-file[path to shell.nix (or similar)]:nix file:_files' \
                        '--context[directory to resolve a flake from]:directory:_files -/' \
                        '--flake[flake installable descriptor (e.g. .#)]:flake'
                    ;;

                gc)
                    local -a gc_subcommands
                    gc_subcommands=(
                        'info:list GC roots and whether their projects still exist'
                        'rm:remove GC roots for gone or selected projects'
                    )
                    _arguments -C \
                        '--json[machine-readable JSON output]' \
                        '1:gc subcommand:->gc_subcmd' \
                        '*::gc args:->gc_args' \
                        && return

                    case $state in
                        gc_subcmd)
                            _describe 'lorri gc subcommand' gc_subcommands
                            ;;
                        gc_args)
                            case $line[1] in
                                info)
                                    _arguments '--json[machine-readable JSON output]'
                                    ;;
                                rm)
                                    _arguments \
                                        '--all[delete roots of all projects]' \
                                        '--older-than[delete roots older than this duration (e.g. 30d, 2m, 1y)]:duration' \
                                        '--dry-run[only print what would be deleted]' \
                                        '--shell-file[delete root for this shell file (repeatable)]:nix file:_files' \
                                        '--json[machine-readable JSON output]'
                                    ;;
                            esac
                            ;;
                    esac
                    ;;

                info)
                    _arguments \
                        '--shell-file[path to shell.nix (required if no flake)]:nix file:_files' \
                        '--context[directory to resolve a flake from]:directory:_files -/' \
                        '--flake[flake installable descriptor]:flake'
                    ;;

                init)
                    # No flags or arguments
                    ;;

                prompt)
                    local -a prompt_subcommands
                    prompt_subcommands=(
                        "default:print 'ℓ' if the cwd is inside a lorri-watched project"
                    )
                    _arguments -C \
                        '1:prompt subcommand:->prompt_subcmd' \
                        '*::prompt args:->prompt_args' \
                        && return

                    case $state in
                        prompt_subcmd)
                            _describe 'lorri prompt subcommand' prompt_subcommands
                            ;;
                        prompt_args)
                            case $line[1] in
                                default)
                                    _arguments \
                                        '--include-leading-space[include a leading space before the prompt symbol]'
                                    ;;
                            esac
                            ;;
                    esac
                    ;;

                hook)
                    _arguments '1:shell:->shell_name'
                    case $state in
                        shell_name) _describe 'shell' hook_shells ;;
                    esac
                    ;;

                export)
                    _arguments '1:shell:->shell_name'
                    case $state in
                        shell_name) _describe 'shell' export_shells ;;
                    esac
                    ;;

                internal)
                    local -a internal_subcommands
                    internal_subcommands=(
                        'ping_:tell the daemon to watch the current project'
                        'stream-events_:stream build events from the daemon as JSON lines'
                        'generate-env_:write $out/env.json from the current Nix build environment'
                    )
                    _arguments -C \
                        '1:internal subcommand:->int_subcmd' \
                        '*::internal args:->int_args' \
                        && return

                    case $state in
                        int_subcmd)
                            _describe 'lorri internal subcommand' internal_subcommands
                            ;;
                        int_args)
                            case $line[1] in
                                ping_)
                                    _arguments \
                                        '--shell-file[path to shell.nix (or similar)]:nix file:_files' \
                                        '--context[directory to resolve a flake from]:directory:_files -/' \
                                        '--flake[flake installable descriptor (e.g. .#)]:flake'
                                    ;;
                                stream-events_)
                                    _arguments \
                                        '--kind[event kind to stream]:kind:(live snapshot all)'
                                    ;;
                                generate-env_)
                                    _arguments '1:varmap file:_files'
                                    ;;
                            esac
                            ;;
                    esac
                    ;;

                help)
                    _describe 'lorri subcommand' subcommands
                    ;;
            esac
            ;;
    esac
}

_lorri "$@"
