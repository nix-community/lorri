# Bash completion for lorri
# Source this file or install to:
#   /usr/share/bash-completion/completions/lorri
# or:
#   ~/.local/share/bash-completion/completions/lorri

_lorri() {
    local cur prev words cword
    _init_completion || return

    # All top-level subcommands
    local subcommands='daemon direnv gc info watch unwatch prompt hook export internal help'

    # Shell names that have a working Hook() implementation
    local hook_shells='bash zsh fish elvish tcsh murex pwsh'

    # All shell names with Export() support
    local export_shells='bash zsh fish elvish tcsh murex pwsh vim json systemd'

    # Find the first non-flag word after 'lorri' — this is the subcommand
    local subcmd=''
    local subcmd2=''
    local i
    for (( i=1; i < cword; i++ )); do
        if [[ ${words[i]} != -* ]]; then
            if [[ -z $subcmd ]]; then
                subcmd=${words[i]}
            elif [[ -z $subcmd2 ]]; then
                subcmd2=${words[i]}
            fi
        fi
    done

    # If the previous word is an option that takes a file argument, complete files
    case "$prev" in
        --shell-file|--context)
            _filedir
            return
            ;;
        --extra-nix-options|--flake|--older-than|--kind)
            # These take free-form values; no completion
            return
            ;;
    esac

    # Dispatch on subcommand
    case "$subcmd" in
        '')
            # No subcommand yet: complete subcommand names
            COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
            ;;

        daemon)
            COMPREPLY=( $(compgen -W '--extra-nix-options' -- "$cur") )
            ;;

        direnv)
            COMPREPLY=( $(compgen -W '--shell-file --context --flake' -- "$cur") )
            ;;

        gc)
            case "$subcmd2" in
                '')
                    COMPREPLY=( $(compgen -W 'info rm --json' -- "$cur") )
                    ;;
                info)
                    COMPREPLY=( $(compgen -W '--json' -- "$cur") )
                    ;;
                rm)
                    COMPREPLY=( $(compgen -W '--all --older-than --dry-run --shell-file --json' -- "$cur") )
                    ;;
            esac
            ;;

        info)
            COMPREPLY=( $(compgen -W '--shell-file --context --flake' -- "$cur") )
            ;;

        watch|unwatch)
            # No flags or arguments
            ;;

        prompt)
            case "$subcmd2" in
                '')
                    COMPREPLY=( $(compgen -W 'default' -- "$cur") )
                    ;;
                default)
                    COMPREPLY=( $(compgen -W '--include-leading-space' -- "$cur") )
                    ;;
            esac
            ;;

        hook)
            if [[ $cur == --* ]]; then
                COMPREPLY=( $(compgen -W '--how' -- "$cur") )
            elif [[ -z $subcmd2 ]]; then
                COMPREPLY=( $(compgen -W "$hook_shells" -- "$cur") )
            fi
            ;;

        export)
            # Argument is a shell name
            if [[ -z $subcmd2 ]]; then
                COMPREPLY=( $(compgen -W "$export_shells" -- "$cur") )
            fi
            ;;

        internal)
            case "$subcmd2" in
                '')
                    COMPREPLY=( $(compgen -W 'ping_ stream-events_ generate-env_' -- "$cur") )
                    ;;
                ping_)
                    COMPREPLY=( $(compgen -W '--shell-file --context --flake' -- "$cur") )
                    ;;
                stream-events_)
                    if [[ $prev == '--kind' ]]; then
                        COMPREPLY=( $(compgen -W 'live snapshot all' -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -W '--kind' -- "$cur") )
                    fi
                    ;;
                generate-env_)
                    # Takes a single file argument
                    _filedir
                    ;;
            esac
            ;;

        help)
            COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
            ;;

    esac
}

complete -F _lorri lorri
