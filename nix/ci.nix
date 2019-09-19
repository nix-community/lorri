{ pkgs, LORRI_ROOT }:
let
  inherit (import ./execline.nix { inherit pkgs; })
    writeExecline;

  # the CI tests we want to run
  # Tests should not depend on each other (or block if they do),
  # so that they can run in parallel.
  # If a test changes files in the repository, sandbox it.
  tests = {
    cargo-fmt = {
      description = "cargo fmt was done";
      test = writeExecline "lint-cargo-fmt" {} [ "${pkgs.cargo}/bin/cargo" "fmt" "--" "--check" ];
    };
    cargo-test = {
      description = "run cargo test";
      test = writeExecline "cargo-test" {} [ "${pkgs.cargo}/bin/cargo" "test" ];
    };
    cargo-clippy = {
      description = "run cargo clippy";
      test = writeExecline "cargo-clippy" {} [
        "export" "RUSTFLAGS" "-D warnings"
        "${pkgs.cargo}/bin/cargo" "clippy"
      ];
    };
  };

in {
  inherit tests;
}
