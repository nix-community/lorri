{ src, runTimeClosure }:
let
  runtimeCfg = import runTimeClosure;

  # Taken from https://github.com/NixOS/nixpkgs/blob/master/lib/strings.nix
  escapeShellArg = arg: "'${builtins.replaceStrings [ "'" ] [ "'\\''" ] (toString arg)}'";

  # using scopedImport, replace readDir and readFile with
  # implementations which will log files and paths they see.
  overrides = {
    import = scopedImport overrides;
    scopedImport = x: builtins.scopedImport (overrides // x);
    builtins = builtins // {
      readFile = file: builtins.trace "lorri read: '${toString file}'" (builtins.readFile file);
      readDir = path: builtins.trace "lorri readdir: '${toString path}'" (builtins.readDir path);
      filterSource = fn: path: let
        # use the filter function to check which of the direct children of path should be watched
        # this allow to ignore usual suspects like .git or a top-level result symlink
        file_type = builtins.readDir path;
        # list of files
        files = builtins.attrNames file_type;
        # list of kept files
        kept = builtins.filter (name: fn "${toString path}/${name}" file_type.${name}) files;
        # whether all files were kept
        all_kept = builtins.length kept == builtins.length files;
        # the actual value we must return
        result = builtins.filterSource fn path;
        # if all files were kept, let's log only ${path}
        logTopLevel = builtins.trace "lorri read: '${toString path}'" result;
        # otherwise log all kept files individually, and the listing of the toplevel dir
        log1stLevel = builtins.trace "lorri readdir: '${toString path}'" (builtins.foldl' (acc: name: builtins.trace "lorri read: '${toString path}/${name}'" acc) result kept);
      in
      if all_kept then logTopLevel else log1stLevel;
    };
  };

  imported =
    let
      raw = overrides.scopedImport overrides src;
    in
      if (builtins.isFunction raw)
      then raw {}
      else raw;

  # If you add a .drv to a gc-root, the `.drv` itself is protected
  # from GC, and the parent `drv`s up the tree are also protected.
  # However, the output paths referenced in any of the drvs are NOT
  # protected.
  #
  # The keep-env-hack function takes a given derivation and replaces
  # its builder with an `env` dumper.
  #
  # gc rooting the resulting store path from this build will retain
  # references to all the store paths needed, preventing the shell's
  # actual environment from being deleted.
  keep-env-hack = drv: derivation (
    drv.drvAttrs // {
      name = "lorri-keep-env-hack-${drv.name}";

      origExtraClosure = drv.extraClosure or [];
      extraClosure = runtimeCfg.closure;

      origBuilder = drv.builder;
      builder = runtimeCfg.builder;

      origSystem = drv.system;
      system = builtins.currentSystem;

      origPATH = drv.PATH or "";
      PATH = runtimeCfg.path;

      # The derivation we're examining may be multi-output. However,
      # this builder only produces the «out» output. Not specifying a
      # single output means we would fail to start a shell for those
      # projects.
      origOutputs = drv.outputs or [];
      outputs = [ "out" ];

      origPreHook = drv.preHook or "";
      preHook = ''
        # Redefine addToSearchPathWithCustomDelimiter to integrate with
        # lorri's environment variable setup map. Then, call the original
        # function. (dirty bash hack.)
        if declare -f addToSearchPathWithCustomDelimiter > /dev/null 2>&1 ; then
          # 1. Fetch the function body's definition using `head` and `tail`
          # 2. Define our own version of the function, which
          # 3. adds to the `varmap-v1` file the arguments, and
          # 4. calls the original function's body
          #
          # For example on how the `head | tail` bits work:
          #
          #     $ foo() { echo foo; }
          #
          #     $ declare -f foo
          #     foo ()
          #     {
          #         echo foo
          #     }
          #
          #     $ declare -f foo | head -n-1 | tail -n+3
          #         echo foo
          #
          # While yes it is dirty, we have a precisely pinned version of
          # bash which we can count on. Thus, if there is a problem or
          # change in output, it will occur in CI, and not on a customer
          # machine.

          lorri_addToSearchPathWithCustomDelimiter="$(declare -f addToSearchPathWithCustomDelimiter | head -n-1 | tail -n+3)"
          addToSearchPathWithCustomDelimiter() {
            local delimiter=$1
            local varname=$2
            local already_present=0

            while IFS="" read -r -d "" map_instruction \
               && IFS="" read -r -d "" match_varname \
               && IFS="" read -r -d "" match_delimiter; do
              unset IFS
              if [[ $match_varname == $varname &&
                    $match_delimiter == $delimiter ]]; then
                already_present=1
                break
              fi
            done < $out/varmap-v1

            if [ $already_present -eq 0 ]; then
              printf 'append\0%s\0%s\0' "$varname" "$delimiter" >> $out/varmap-v1
            fi

            eval "$lorri_addToSearchPathWithCustomDelimiter"
          }
        fi

        ${drv.preHook or ""}
      '';

      origArgs = drv.args or [];
      args = [
        "-e"
        (
          builtins.toFile "lorri-keep-env-hack" ''
            mkdir -p "$out"
            touch "$out/varmap-v1"

            # Export IN_NIX_SHELL to trick various Nix tooling to export
            # shell-friendly variables
            export IN_NIX_SHELL=impure

            # Export IN_LORRI_SHELL to differentiate between `lorri shell` and
            # `nix-shell`
            export IN_LORRI_SHELL=${escapeShellArg src}

            # https://github.com/NixOS/nix/blob/92d08c02c84be34ec0df56ed718526c382845d1a/src/nix-build/nix-build.cc#
            [ -e $stdenv/setup ] && . $stdenv/setup

            # target/lorri#23
            # https://github.com/NixOS/nix/blob/bfc6bdf222d00d3cb1b0e168a5d55d1a7c9cdb72/src/nix-build/nix-build.cc#L424
            if [ "$(type -t runHook)" = function ]; then
             runHook shellHook;
            fi;

            export > $out/bash-export
          ''
        )
      ];

      # Because it’s a trivially lightweight drv, we should never substitute.
      preferLocalBuild = true;
      allowSubstitutes = false;
    }
  );

  gc-root = keep-env-hack imported;

in
gc-root
