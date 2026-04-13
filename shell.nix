{
  pkgs ? import <nixpkgs> {
    # This is a hack to work around something requiring libcap on MacOS
    config.allowUnsupportedSystem = true;
  }
}:

let
  # Needed by go test — gates all Nix-dependent tests and used as a
  # runtime fallback in main.go when the binary isn't built with -X.
  RUN_TIME_CLOSURE = pkgs.callPackage ./nix/runtime.nix {};

  # Root of the repository; used by integration tests to locate fixtures.
  LORRI_ROOT = toString ./.;

in
pkgs.mkShell {
  name = "lorri";

  packages = [
    pkgs.go
    pkgs.git
    pkgs.direnv
    pkgs.nix-prefetch-git
    pkgs.nixpkgs-fmt
    pkgs.yj
    pkgs.graphviz
    pkgs.zathura
    pkgs.nix
  ];

  inherit RUN_TIME_CLOSURE LORRI_ROOT;

  shellHook = ''
    echo "You opened a nix-shell for lorri; this is fine, but we strongly encourage the use of direnv(1) and lorri(1) to develop lorri ;)" 1>&2
  '';

  preferLocalBuild = true;
  allowSubstitutes = false;
}
