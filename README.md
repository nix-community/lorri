# ℓ lorri

lorri is a `nix-shell` replacement for project development. lorri monitors
your `shell.nix` and `flake.nix` and rebuilds the development environment
in the background, automatically applying changes on your next shell prompt.

When changes are made that would affect a project's development shell, lorri
builds the new shell in the background and applies the result automatically.
The result is that development tools are kept in sync with the current
Nix shell configuration (even e.g. as you switch branches) without blocking
your use of the terminal.

Unlike e.g. the `use nix` `direnv` integration, lorri will cache the last
shell environment and instantly apply it, so you can start work where you
left off. After a few seconds, the new environment will be ready to load.

lorri supports both `shell.nix` and `devShells` in `flake.nix` files.

The project is about experimenting with and improving the developer's experience
with Nix. A particular focus is managing your project's external dependencies,
editor integration, and quick feedback.

lorri supports Linux and macOS. macOS support is brittle, we are looking for
funding to improve it (see below).

## Business Support

We provide support to businesses who wish to use lorri in their development team
or wish to pay for further improvements or set up support contracts.

See [BUSINESS_SUPPORT.md](BUSINESS_SUPPORT.md) for further information and
[ROADMAP.md](ROADMAP.md) for an overview of planned improvements that need
funding.

## Demo

TODO: add a new demo screencast showing the native shell hook workflow.

## Setup on NixOS or with `home-manager` on Linux

If you are using [NixOS][nixos] or [`home-manager`][home-manager] on Linux and
a Nixpkgs channel at least as recent as `nixos-19.09`, you can get started with
lorri as follows. Otherwise see the next section, [Setup on other
platforms](#setup-on-other-platforms).

1. **Enable the daemon service.** Set `services.lorri.enable = true;` in your
   NixOS [`configuration.nix`][nixos-service] or your home-manager
   [`home.nix`][home-manager-service].

   This will automatically install the `lorri` command.

   **Note**: There's [a known issue](https://github.com/target/lorri/issues/374)
   preventing the lorri daemon from starting automatically upon installation.
   Until it's resolved, you'll have to reload the user daemon by hand by running
   `systemctl --user daemon-reload`, or reboot.

2. **Install the shell hook.** Add the following to your shell's rc file:

   ```bash
   # bash (~/.bashrc)
   eval "$(lorri hook bash)"

   # zsh (~/.zshrc)
   eval "$(lorri hook zsh)"

   # fish (~/.config/fish/config.fish)
   lorri hook fish | source
   ```

3. **Watch your project.** Run `lorri watch` in your project directory. If
   you don't have a `shell.nix` or `flake.nix` yet, lorri will suggest how
   to create one with `nix flake init`. Then open a new shell and `cd` into
   your project directory — lorri will start building the environment
   automatically.

## Setup on other platforms

If you are running Nix on a Linux distribution other than NixOS or on macOS,
the following instructions will help you get started with lorri.

1. **Install lorri.** If you are using a Nixpkgs channel at least as recent as
   `nixos-19.09`, you can install lorri using `nix-env -i lorri`.

   Otherwise, install lorri from the repository as follows:

   ```console
   # With flakes:
   $ nix profile install github:nix-community/lorri

   # Without flakes:
   $ nix-env -if https://github.com/nix-community/lorri/archive/canon.tar.gz
   ```

2. **Start the daemon.** For testing, you can start the daemon in a separate
   terminal by running `lorri daemon`.

   See [`contrib/daemon.md`](contrib/daemon.md) for ways to start the daemon
   automatically in the background.

3. **Install the shell hook.** Add the following to your shell's rc file:

   ```bash
   # bash (~/.bashrc)
   eval "$(lorri hook bash)"

   # zsh (~/.zshrc)
   eval "$(lorri hook zsh)"

   # fish (~/.config/fish/config.fish)
   lorri hook fish | source
   ```

4. **Watch your project.** Run `lorri watch` in your project directory. If
   you don't have a `shell.nix` or `flake.nix` yet, lorri will suggest how
   to create one with `nix flake init`. Then open a new shell and `cd` into
   your project directory — lorri will start building the environment
   automatically.

## Usage

Once the daemon is running and the shell hook is set up, lorri monitors your
`shell.nix` and its dependencies and triggers builds as required. Changes are
applied automatically on the next prompt.

The cached environment is loaded even when the daemon is not running. However,
the daemon must be running for the environment to be updated when `shell.nix`
changes.

### Managing projects

lorri only loads environments for projects it knows about. Run `lorri watch` to
register a project — it will auto-detect your `shell.nix` or `flake.nix`, and
suggest `nix flake init` if neither exists. To stop managing a project, run
`lorri unwatch` — this removes its GC root and stops the daemon from rebuilding
it.

## Migrating from direnv

If you previously used lorri with direnv, `lorri direnv` continues to work
during the transition but is deprecated. To migrate:

1. Add `eval "$(lorri hook bash)"` (or the equivalent for your shell) to your
   shell rc file.
2. Remove your `.envrc`.
3. The next time the daemon rebuilds your project, the native hook will load the
   environment automatically.

## Editor integration

Since lorri manages the shell environment directly, any editor launched from a
shell with the lorri hook active will inherit the correct environment
automatically.

If you use Emacs, our [`direnv-mode` tutorial](./contrib/emacs.md) describes
editor-level environment integration. Note that with the native lorri hook,
per-buffer environment switching in Emacs requires `envrc.el` or similar; see
the note in that document.

Editors like `vscode` will proxy e.g. debug commands through your
user shell, so try without a direnv plugin first and see whether it works.
For projects that don’t change very often, it can be enough to `cd` into
the project directory, let the lorri shell do its magic and then run your
editor from inside the changed environment directly.

If you *really* rely on direnv for editor integration (e.g. `emacs-direnv`,
`vscode-direnv`), you can use the direnv adapter instead of the full migration.
Replace the contents of your `.envrc` with:

```bash
eval "$(lorri export direnv-adapter)"
```


## Associated Projects

lorri embodies a Unix philosophy of doing one thing well. As a result, it works
very well with other tools to provide a powerful and streamlined development
experience.

lorri is hugely inspired by [direnv](https://github.com/direnv/direnv), and
its `lorri hook` support reuses the direnv modules that implement the shell
integration.

As a foundation, lorri relies on [Nix and Nixpkgs](https://nixos.org/nix/) to
install and manage software packages.

For pinning versions of software during development,
[niv](https://github.com/nmattia/niv) is very helpful.

---

## Support & Questions

Please use the [issue tracker](https://github.com/nix-community/lorri/issues)
for any problems or bugs you encounter.

We are on IRC/Matrix, `libera/#lorri` ([Webchat][]), though we might not be responsive at all times.

[Webchat]: https://matrix.to/#/#lorri:libera.chat

## How To Help

All development on lorri happens on the Github repository, in the open. You can
propose a change in an issue, then create a pull request after some discussion.
Some issues are marked with the "good first issue" label, those are a good place
to start. Just remember to leave a comment when you start working on something.

## Debugging

Set `LORRI_DEBUG_PANIC=1` to get a raw panic stack trace instead of the
formatted crash report when lorri crashes.

### lorri reevaluates more than expected

lorri sometimes recursively watches a directory that the user did not expect.
This can happen for a number of reasons:

1. When using a local checkout instead of a channel for `nixpkgs`, lorri watches
   that directory recursively, and will trigger on any file change.
2. When specifying `src` via a path, (like the much-used `src = ./.;`) lorri
   watches that path recursively (see
   https://github.com/target/lorri/issues/6 for details). To get around this,
   use a `builtins.filterSource`-based function to filter `src`, e.g., use
   [`nix-gitignore`](https://github.com/NixOS/nixpkgs/blob/8c1f1b2324bb90f8e1ea33db3253eb30c330ed99/pkgs/build-support/nix-gitignore/default.nix):
   `src = pkgs.nix-gitignore.gitignoreSource [] ./.`, or one of the functions in
   [`nixpkgs/lib/sources.nix`](https://github.com/NixOS/nixpkgs/blob/8c1f1b2324bb90f8e1ea33db3253eb30c330ed99/lib/sources.nix)

---

## Evaluator + watch design

The evaluator should eagerly reevaluate the Nix expressions as soon as anything
material to their output changes. This takes place in a few stages.

### Initial evaluation

`InstantiateAndBuild` instantiates (and builds) the Nix expression with
`nix-instantiate -vv`. The evaluator prints each imported Nix file, and each
copied source file. `InstantiateAndBuild` parses the log and notes each of these
paths out as an "input" path.

Each input path is the absolute path which Nix examined.

Each input path is then passed to `PathReduction` which examines each path
referenced, and reduces it to a minimum set of paths with the following rules:

1. Discard any store paths which isn't a symlink to outside the store: they are
   immutable.
2. Replace any store path which is a symlink to outside the store to the
   destination of the symlink.
3. Replace a reference to a Nix Channel with the updateable symlink root of the
   channel. Concretely, replace the path
   `/nix/var/nix/profiles/per-user/root/channels/nixos/default.nix` with
   `/nix/var/nix/profiles/per-user/root/` to watch for the channels symlink to
   change.

Initial testing collapses over 2,000 paths to just five.

### Loop

Each identified path is watched for changes with inotify (Linux) or fsevent
(macOS). If the watched path is a directory, all of its sub-directories are also
watched for changes.

Each new batch of change notifications triggers a fresh evaluation. Newly
discovered paths are added to the watch list.

### Garbage Collection Roots

lorri creates an indirect garbage collection root for each .drv in
`$XDG_CACHE_HOME/lorri` (`~/.cache/lorri/` by default) each time it evaluates
your project.

### License & Copyright

Copyright 2019–2020 Target, Copyright 2021 The Nix Community
License: Apache 2.0 (see [`LICENSE` file](./LICENSE))

---

## Historical

Lorri was originally started by [Tweag]() for [Target](https://target.com). See
the [original announcement Blogpost](https://www.tweag.io/blog/2019-03-28-introducing-lorri/)
(2018-03-28) for more details and a sense of the original feature set.

###### ASCII Art

    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    #################( )############################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################
    ################################################################

_([Nix as observed by LORRI on 2015-07-13](https://www.nasa.gov/newhorizons/lorri-gallery))_

[contrib]: ./contrib
[home-manager-service]: https://nix-community.github.io/home-manager/options.xhtml#opt-services.lorri.enable
[home-manager]: https://nix-community.github.io/home-manager/
[lorri-blog-post]: https://www.tweag.io/posts/2019-03-28-introducing-lorri.html
[nixos-service]: https://nixos.org/nixos/options.html#services.lorri.enable
[nixos]: https://nixos.org/
