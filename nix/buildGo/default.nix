# Copyright 2019 Google LLC.
# SPDX-License-Identifier: Apache-2.0
#
# buildGo provides Nix functions to build Go packages in the style of Bazel's
# rules_go.

{ pkgs ? import <nixpkgs> { }
, ...
}:

let
  inherit (builtins)
    attrNames
    baseNameOf
    dirOf
    elemAt
    filter
    listToAttrs
    map
    match
    readDir
    replaceStrings
    toString;

  inherit (pkgs) lib runCommand runCommandCC fetchFromGitHub protobuf symlinkJoin go;
  goStdlib = buildStdlib go;

  # Helper to recursively find all paths to gopkg attributes in a dependency
  # Returns a list of dot-separated paths (empty string for root gopkg)
  findGoPkgs = prefix: attrSet:
    let
      # Check if this level has gopkg
      hasGoPkg = attrSet ? gopkg;
      currentPkg = if hasGoPkg then [ prefix ] else [ ];

      # Get all attribute names, filtering out reserved/special attributes
      attrNames = builtins.filter
        (name: name != "gopkg" && name != "goDeps" && name != "goImportPath")
        (builtins.attrNames attrSet);

      # Recursively check children (but not derivations)
      childPkgs = lib.concatMap
        (name:
          let
            val = attrSet.${name};
            newPrefix = if prefix == "" then name else "${prefix}.${name}";
          in
          if (lib.isAttrs val && !lib.isDerivation val)
          then findGoPkgs newPrefix val
          else [ ])
        attrNames;
    in
    currentPkg ++ childPkgs;

  # Validate that a dependency has a gopkg attribute, providing helpful errors if not
  validateDep = depIndex: dep:
    if dep ? gopkg
    then dep
    else
      let
        # Find all available gopkg paths
        availablePkgs = findGoPkgs "" dep;

        # Get top-level attributes for additional context
        topLevelAttrs = builtins.attrNames dep;

        # Format the available packages list
        pkgList = lib.concatMapStringsSep "\n  "
          (path: if path == "" then "(root package)" else path)
          availablePkgs;

        errorMsg = ''
          Dependency at position ${toString depIndex} is missing 'gopkg' attribute.

          ${if availablePkgs == [ ] then ''
            This dependency has no gopkg attributes at all.
            Available attributes: ${lib.concatStringsSep ", " topLevelAttrs}

            This might not be a valid buildGo.external result.
          '' else if (builtins.length availablePkgs == 1 && builtins.head availablePkgs != "") then ''
            This is a repository with only subpackages (no root package).
            Use this instead:
              ${builtins.head availablePkgs}
          '' else ''
            This dependency contains multiple packages. Use one of these instead:
              ${pkgList}

            Top-level attributes: ${lib.concatStringsSep ", " topLevelAttrs}
          ''}
        '';
      in
      throw errorMsg;

  # Helpers for low-level Go compiler invocations
  spaceOut = lib.concatStringsSep " ";

  includeDepSrc = dep: "-I ${dep}";
  includeSources = deps: spaceOut (map includeDepSrc deps);

  includeDepLib = dep: "-L ${dep}";
  includeLibs = deps: spaceOut (map includeDepLib deps);

  srcBasename = src: elemAt (match "([a-z0-9]{32}\-)?(.*\.go)" (baseNameOf src)) 1;
  srcCopy = path: src: "cp ${src} $out/${path}/${srcBasename src}";
  srcList = path: srcs: lib.concatStringsSep "\n" (map (srcCopy path) srcs);

  # anyBasename strips the Nix content-hash prefix from any filename,
  # not just .go files. Used to stage embed targets alongside .go sources.
  # e.g. /nix/store/abc123-envrc.bash → envrc.bash
  anyBasename = src: elemAt (match "([a-z0-9]{32}\-)?(.*)" (baseNameOf src)) 1;

  # stageSrcs emits shell commands to copy a list of srcs into destDir,
  # stripping Nix hash prefixes so files land with their original names.
  stageSrcs = destDir: srcs:
    lib.concatStringsSep "\n"
      (map (src: "cp ${src} ${destDir}/${anyBasename src}") srcs);

  # Collect all transitive dependencies (assumes deps already have gopkg attribute)
  allDeps = deps: lib.unique (lib.flatten (deps ++ (map (d: d.goDeps) deps)));

  xFlags = x_defs: spaceOut (map (k: "-X ${k}=${x_defs."${k}"}") (attrNames x_defs));

  # Add an `overrideGo` attribute to a function result that works
  # similar to `overrideAttrs`, but is used specifically for the
  # arguments passed to Go builders.
  makeOverridable = f: orig: (f orig) // {
    overrideGo = new: makeOverridable f (orig // (new orig));
  };

  buildStdlib = go: runCommandCC "go-stdlib-${go.version}"
    {
      nativeBuildInputs = [ go ];
    } ''
    HOME=$NIX_BUILD_TOP/home
    mkdir $HOME

    goroot="$(go env GOROOT)"
    cp -R "$goroot/src" "$goroot/pkg" .

    chmod -R +w .
    GODEBUG=installgoroot=all GOROOT=$NIX_BUILD_TOP go install -v --trimpath std

    mkdir $out
    cp -r pkg/*_*/* $out

    find $out -name '*.a' | while read -r ARCHIVE_FULL; do
      ARCHIVE="''${ARCHIVE_FULL#"$out/"}"
      PACKAGE="''${ARCHIVE%.a}"
      echo "packagefile $PACKAGE=$ARCHIVE_FULL"
    done > $out/importcfg
  '';

  importcfgCmd = { name, deps, out ? "importcfg" }: ''
    echo "# nix buildGo ${name}" > "${out}"
    cat "${goStdlib}/importcfg" >> "${out}"
    ${lib.concatStringsSep "\n" (map (dep: ''
      find "${dep}" -name '*.a' | while read -r pkgp; do
        relpath="''${pkgp#"${dep}/"}"
        pkgname="''${relpath%.a}"
        echo "packagefile $pkgname=$pkgp"
      done >> "${out}"
    '') deps)}
  '';

  # High-level build functions

  # Simple program builder without embed support (used for internal tools)
  simpleProgram = { name, srcs, deps ? [ ], x_defs ? { } }:
    let
      # Validate dependencies first, then extract gopkg
      validated = lib.imap1 validateDep deps;
      uniqueDeps = allDeps (map (d: d.gopkg) validated);
    in runCommand name { } ''
      ${importcfgCmd { inherit name; deps = uniqueDeps; }}
      ${go}/bin/go tool compile -o ${name}.a -importcfg=importcfg -trimpath=$PWD -trimpath=${go} -p main ${includeSources uniqueDeps} ${spaceOut srcs}
      mkdir -p $out/bin
      export GOROOT_FINAL=go
      ${go}/bin/go tool link -o $out/bin/${name} -importcfg=importcfg -buildid nix ${xFlags x_defs} ${includeLibs uniqueDeps} ${name}.a
    '';

  # Tool to generate embedcfg JSON from go list output
  # Built with simpleProgram to avoid circular dependency
  mkembedcfg = simpleProgram {
    name = "mkembedcfg";
    srcs = [ ./mkembedcfg/main.go ];
  };

  # Build a Go program out of the specified files and dependencies.
  # Supports go:embed directives.
  program = { name, srcs, deps ? [ ], x_defs ? { } }:
    let
      # Validate dependencies first, then extract gopkg
      validated = lib.imap1 validateDep deps;
      uniqueDeps = allDeps (map (d: d.gopkg) validated);

      # Only .go files are passed to the compiler; non-.go files (embedded
      # bash scripts, .nix files, etc.) are staged alongside .go files so
      # go list can resolve //go:embed patterns, but must not be compiled.
      isGoFile = src: lib.hasSuffix ".go" (baseNameOf src);
      goSrcs = builtins.filter isGoFile srcs;

    in
    runCommand name {
      nativeBuildInputs = [ go mkembedcfg ];
    } ''
      export HOME=$NIX_BUILD_TOP/home
      mkdir -p $HOME

      # Stage ALL srcs into a working directory so the compiler can find
      # embedded files relative to the .go sources.
      mkdir -p srcdir
      ${stageSrcs "srcdir" srcs}

      ${importcfgCmd { inherit name; deps = uniqueDeps; }}

      # Generate embedcfg from within our own srcdir so all paths are correct.
      if grep -q "//go:embed" ${spaceOut goSrcs}; then
        cd srcdir
        echo "module tempmodule" > go.mod
        echo "go ${go.version}" >> go.mod
        if ${go}/bin/go list -json . > golist.json 2>golist.err; then
          ${mkembedcfg}/bin/mkembedcfg -srcdir $PWD < golist.json > ../embedcfg.json
        else
          echo "go list failed:" >&2
          cat golist.err >&2
          echo '{"Patterns":{},"Files":{}}' > ../embedcfg.json
        fi
        cd ..
        EMBED_FLAG="-embedcfg $PWD/embedcfg.json"
      else
        EMBED_FLAG=""
      fi

      # Compile only .go files; embed targets are accessed via embedcfg.
      ${go}/bin/go tool compile $EMBED_FLAG -o ${name}.a -importcfg=importcfg -trimpath=$PWD -trimpath=${go} -p main ${includeSources uniqueDeps} ${spaceOut (map (s: "srcdir/${anyBasename s}") goSrcs)}
      mkdir -p $out/bin
      export GOROOT_FINAL=go
      ${go}/bin/go tool link -o $out/bin/${name} -importcfg=importcfg -buildid nix ${xFlags x_defs} ${includeLibs uniqueDeps} ${name}.a
    '';

  # Build a Go library assembled out of the specified files.
  #
  # This outputs both the sources and compiled binary, as both are
  # needed when downstream packages depend on it.
  package = { name, srcs, deps ? [ ], path ? name, sfiles ? [ ] }:
    let
      # Validate dependencies first, then extract gopkg
      validated = lib.imap1 validateDep deps;
      uniqueDeps = allDeps (map (d: d.gopkg) validated);

      # Generate embedcfg for go:embed support.
      # Stage all srcs into a real directory so go list resolves //go:embed patterns.
      embedcfg = runCommand "${name}-embedcfg" {
        nativeBuildInputs = [ go mkembedcfg ];
      } ''
        if grep -q "//go:embed" ${spaceOut srcs}; then
          export HOME=$NIX_BUILD_TOP/home
          mkdir -p $HOME

          mkdir -p srcdir
          ${stageSrcs "srcdir" srcs}

          cd srcdir
          echo "module tempmodule" > go.mod
          echo "go ${go.version}" >> go.mod
          if ${go}/bin/go list -json . > golist.json 2>golist.err; then
            ${mkembedcfg}/bin/mkembedcfg -srcdir $PWD < golist.json > $out
          else
            echo "go list failed:" >&2
            cat golist.err >&2
            echo '{"Patterns":{},"Files":{}}' > $out
          fi
        else
          echo '{"Patterns":{},"Files":{}}' > $out
        fi
      '';

      # The build steps below need to be executed conditionally for Go
      # assembly if the analyser detected any *.s files.
      #
      # This is required for several popular packages (e.g. x/sys).
      ifAsm = do: lib.optionalString (sfiles != [ ]) do;
      asmBuild = ifAsm ''
        ${go}/bin/go tool asm -p ${path} -trimpath $PWD -I $PWD -I ${go}/share/go/pkg/include -D GOOS_linux -D GOARCH_amd64 -gensymabis -o ./symabis ${spaceOut sfiles}
        ${go}/bin/go tool asm -p ${path} -trimpath $PWD -I $PWD -I ${go}/share/go/pkg/include -D GOOS_linux -D GOARCH_amd64 -o ./asm.o ${spaceOut sfiles}
      '';
      asmLink = ifAsm "-symabis ./symabis -asmhdr $out/go_asm.h";
      asmPack = ifAsm ''
        ${go}/bin/go tool pack r $out/${path}.a ./asm.o
      '';

      gopkg = (runCommand "golib-${name}" {
        nativeBuildInputs = [ pkgs.jq ];
      } ''
        export HOME=$NIX_BUILD_TOP/home
        mkdir -p $out/${path}
        ${srcList path (map (s: "${s}") srcs)}
        ${asmBuild}
        ${importcfgCmd { inherit name; deps = uniqueDeps; }}

        # Check if embedcfg has actual embeds and set flag accordingly
        if [ "$(jq '.Patterns | length' < ${embedcfg})" -gt 0 ]; then
          EMBED_FLAG="-embedcfg ${embedcfg}"
        else
          EMBED_FLAG=""
        fi

        ${go}/bin/go tool compile -pack ${asmLink} $EMBED_FLAG -o $out/${path}.a -importcfg=importcfg -trimpath=$PWD -trimpath=${go} -p ${path} ${includeSources uniqueDeps} ${spaceOut srcs}
        ${asmPack}
      '').overrideAttrs (_: {
        passthru = {
          inherit gopkg;
          goDeps = uniqueDeps;
          goImportPath = path;
        };
      });
    in
    gopkg;

  # Build a tree of Go libraries out of an external Go source
  # directory that follows the standard Go layout and was not built
  # with buildGo.nix.
  #
  # The derivation for each actual package will reside in an attribute
  # named "gopkg", and an attribute named "gobin" for binaries.
  external = import ./external { inherit pkgs program package; };

in
{
  # Only the high-level builder functions are exposed, but made
  # overrideable.
  program = makeOverridable program;
  package = makeOverridable package;
  external = makeOverridable external;

  # Internal tools
  inherit mkembedcfg;

  # re-expose the Go version used
  inherit go;
}
