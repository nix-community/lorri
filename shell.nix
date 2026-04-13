{
  # Pull in tools & environment variables that are only
  # required for interactive development (i.e. not necessary
  # on CI). Only when this is enabled, extra dev tools are included.
  isDevelopmentShell ? true
, nixpkgs ? ./nix/nixpkgs-stable.nix
, pkgs ? import nixpkgs {
    # This is a hack to work around something requiring libcap on MacOS
    config.allowUnsupportedSystem = true;
  }
}:

let
  lib = import ./nix/lib { inherit pkgs; };

  # Needed by go test — gates all Nix-dependent tests and used as a
  # runtime fallback in main.go when the binary isn't built with -X.
  RUN_TIME_CLOSURE = pkgs.callPackage ./nix/runtime.nix {};

  # Root of the repository; used by integration tests to locate fixtures.
  LORRI_ROOT = toString ./.;

  buildInputs = [
    pkgs.go
    pkgs.git
    pkgs.direnv
    pkgs.nix-prefetch-git
    pkgs.nixpkgs-fmt
    pkgs.yj
    pkgs.execline
    (lib.binify { exe = lib.nix-run; name = "nix-run"; })

    # To ensure we always have a compatible nix in our shells.
    # CI doesn't know `nix-env` otherwise.
    pkgs.nix
  ]
  ++ pkgs.lib.optionals isDevelopmentShell [
    pkgs.graphviz
    pkgs.zathura
  ];

in
pkgs.mkShell {
  name = "lorri";
  inherit buildInputs;
  inherit RUN_TIME_CLOSURE LORRI_ROOT;

  shellHook = ''
    echo "You opened a nix-shell for lorri; this is fine, but we strongly encourage the use of direnv(1) and lorri(1) to develop lorri ;)" 1>&2
  '';

  preferLocalBuild = true;
  allowSubstitutes = false;
}
