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

  # Common env vars shared by all dev shells.
  # Must be 0 so the test binary is statically linked and can be copied
  # into the Nix build sandbox by logged-evaluation.nix.
  # Go's std/net uses CGO for DNS by default; 0 switches to the pure-Go
  # resolver.
  commonEnv = {
    inherit RUN_TIME_CLOSURE LORRI_ROOT;
    NIX_PATH    = "nixpkgs=${pkgs.path}";
    CGO_ENABLED = "0";
  };

  # Packages needed to run go test ./... including all shell integration tests.
  ciPackages = with pkgs; [
    go nix direnv git bash yj
    zsh fish elvish tcsh nushell
  ];

  ci = pkgs.mkShell ({
    name = "lorri-ci";
    packages = ciPackages;
  } // commonEnv);

in
pkgs.mkShell ({
  name = "lorri";

  packages = ciPackages ++ (with pkgs; [
    nix-prefetch-git
    nixpkgs-fmt
    graphviz
    zathura
  ]);

  shellHook = ''
    echo "You opened a nix-shell for lorri; this is fine, but we strongly encourage the use of and lorri(1) to develop lorri ;)" 1>&2
  '';

  preferLocalBuild = true;
  allowSubstitutes = false;

  passthru = { inherit ci; };
} // commonEnv)
