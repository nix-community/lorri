# tcsh completion for lorri
# Source this from your ~/.tcshrc:
#   source ~/.nix-profile/share/tcsh-completion/completions/lorri.tcsh
#
# tcsh completion does not support descriptions alongside candidates.
#
# All specs must be in a single `complete lorri` call — tcsh replaces
# rather than merges on repeated calls for the same command.

complete lorri \
    'p/1/(daemon direnv gc info init prompt hook export internal help)/' \
    'n/gc/(info rm)/' \
    'n/prompt/(default)/' \
    'n/internal/(ping_ stream-events_ generate-env_)/' \
    'n/hook/(bash zsh fish elvish tcsh murex pwsh)/' \
    'n/export/(bash zsh fish elvish tcsh murex pwsh vim json systemd)/' \
    'n/--shell-file/f/' \
    'n/--context/d/' \
    'n/--flake/n/' \
    'n/--extra-nix-options/n/' \
    'n/--older-than/n/' \
    'n/--kind/(live snapshot all)/'
