# Copyright 2019 Google LLC.
# SPDX-License-Identifier: Apache-2.0
{ pkgs, program, package }:

let
  inherit (builtins)
    elemAt
    foldl'
    fromJSON
    head
    length
    listToAttrs
    readFile
    replaceStrings
    tail
    unsafeDiscardStringContext
    throw;

  inherit (pkgs) lib runCommand go jq ripgrep;

  pathToName = p: replaceStrings [ "/" ] [ "_" ] (toString p);

  # Collect all non-vendored dependencies from the Go standard library
  # into a file that can be used to filter them out when processing
  # dependencies.
  stdlibPackages = runCommand "stdlib-pkgs.json" { } ''
    export HOME=$PWD
    export GOPATH=/dev/null
    ${go}/bin/go list std | \
      ${ripgrep}/bin/rg -v 'vendor' | \
      ${jq}/bin/jq -R '.' | \
      ${jq}/bin/jq -c -s 'map({key: ., value: true}) | from_entries' \
      > $out
  '';

  analyser = program {
    name = "analyser";

    srcs = [
      ./main.go
    ];

    x_defs = {
      "main.stdlibList" = "${stdlibPackages}";
    };
  };

  mkset = path: value:
    if path == [ ] then { gopkg = value; }
    else { "${head path}" = mkset (tail path) value; };

  last = l: elemAt l ((length l) - 1);

  toPackage = self: src: path: depMap: entry:
    let
      localDeps = map
        (d: lib.attrByPath (d ++ [ "gopkg" ])
          (
            let
              localPath = lib.concatStringsSep "/" d;
              fullPath = if localPath == "" then path else "${path}/${localPath}";
            in
            throw "missing local dependency '${fullPath}' in package '${entry.name}' (${path})\n\nThis indicates a package within the same repository is missing from the build.\nCheck that all Go packages in the source are being analyzed correctly."
          )
          self)
        entry.localDeps;

      # Check which foreign dependencies are missing
      missingForeign = builtins.filter
        (d: !(depMap ? ${d.path}))
        entry.foreignDeps;

      # If any are missing, report them all at once
      foreignDeps =
        if missingForeign != [ ]
        then
          let
            depList = lib.concatMapStringsSep "\n"
              (d: "  - ${d.path} (imported at ${d.position})")
              missingForeign;
          in
          throw ''
            Missing ${toString (builtins.length missingForeign)} foreign ${if builtins.length missingForeign == 1 then "dependency" else "dependencies"} in '${path}':

            ${depList}

            Hint: Add these packages to deps in go-deps.nix
                  For subpackage imports (e.g., foo/bar/baz), you may need to add the parent package.
          ''
        else map (d: depMap.${d.path}) entry.foreignDeps;

      args = {
        srcs = map (f: src + ("/" + f)) entry.files;
        deps = localDeps ++ foreignDeps;
      };

      libArgs = args // {
        name = pathToName entry.name;
        path = lib.concatStringsSep "/" ([ path ] ++ entry.locator);
        sfiles = map (f: src + ("/" + f)) entry.sfiles;
      };

      binArgs = args // {
        name = (last ((lib.splitString "/" path) ++ entry.locator));
      };
    in
    if entry.isCommand then (program binArgs) else (package libArgs);

in
{ src, path, deps ? [ ] }:
let
  # Build a map of dependencies (from their import paths to their
  # derivation) so that they can be conditionally imported only in
  # sub-packages that require them.
  depMap = listToAttrs (map
    (d: {
      name = d.goImportPath;
      value = d;
    })
    (map (d: d.gopkg) deps));

  name = pathToName path;
  analysisOutput = runCommand "${name}-structure.json" { } ''
    ${analyser}/bin/analyser -path ${path} -source ${src} > $out
  '';
  # readFile adds the references of the read in file to the string context for
  # Nix >= 2.6 which would break the attribute set construction in fromJSON
  analysis = fromJSON (unsafeDiscardStringContext (readFile analysisOutput));
in
lib.fix (self: foldl' lib.recursiveUpdate { } (
  map (entry: mkset entry.locator (toPackage self src path depMap entry)) analysis
))
