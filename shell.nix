{
  # Pull in tools & environment variables that are only
  # required for interactive development (i.e. not necessary
  # on CI). Only when this is enabled, Rust nightly is used.
  isDevelopmentShell ? true
, nixpkgs ? ./nix/nixpkgs-stable.nix
, pkgs ? import nixpkgs {
    # This is a hack to work around something requiring libcap on MacOS
    config.allowUnsupportedSystem = true;
  }
}:

let
  ci = import ./nix/ci {
    inherit
      pkgs
      LORRI_ROOT
      BUILD_REV_COUNT
      RUN_TIME_CLOSURE
      ;
  };
  lib = import ./nix/lib { inherit pkgs; };

  # Lorri-specific

  # The root directory of this project
  LORRI_ROOT = toString ./.;
  # Needed by the lorri build.rs to determine its own version
  # for the development repository (non-release), we set it to 1
  BUILD_REV_COUNT = 1;
  # Needed by the lorri build.rs to access some tools used during
  # the build of lorri's environment derivations.
  RUN_TIME_CLOSURE = pkgs.callPackage ./nix/runtime.nix {};

  # Rust-specific

  # Enable printing backtraces for rust binaries
  RUST_BACKTRACE = 1;

  # Only in development shell

  # Needed for racer “jump to definition” editor support
  # In Emacs with `racer-mode`, you need to set
  # `racer-rust-src-path` to `nil` for it to pick
  # up the environment variable with `direnv`.
  RUST_SRC_PATH = "${pkgs.rustc.src}/lib/rustlib/src/rust/src/";
  # Set up a local directory to install binaries in
  CARGO_INSTALL_ROOT = "${LORRI_ROOT}/.cargo";

  buildInputs = [
    # please use rustup to install rust, setting it up via nix is a bother
    pkgs.rustup
    pkgs.git
    pkgs.direnv
    pkgs.crate2nix
    pkgs.nix-prefetch-git
    pkgs.nixpkgs-fmt
    pkgs.ninja
    pkgs.execline
    (lib.binify { exe = lib.nix-run; name = "nix-run"; })

    # To ensure we always have a compatible nix in our shells.
    # CI doesn’t know `nix-env` otherwise.
    pkgs.nix
  ]
  ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.Security
    pkgs.darwin.apple_sdk.frameworks.CoreServices
    pkgs.darwin.apple_sdk.frameworks.CoreFoundation
    pkgs.libiconv
  ]
  ++ pkgs.lib.optionals isDevelopmentShell [
    pkgs.graphviz
    pkgs.zathura
  ];

in
pkgs.mkShell (
  {
    name = "lorri";
    inherit buildInputs;

    inherit BUILD_REV_COUNT RUN_TIME_CLOSURE;

    inherit RUST_BACKTRACE;

    # Executed when entering `nix-shell`
    shellHook = ''
      # this is mirrored from .envrc to make available from nix-shell
      # pick up cargo plugins
      export PATH="$LORRI_ROOT/.cargo/bin:$PATH"

      echo "You opened a nix-shell for lorri; this is fine, but we strongly enourage the use of direnv(1) and lorri(1) to develop lorri ;)" 1>&2

    '' + (
      if !pkgs.stdenv.isDarwin then "" else ''
        # Cargo wasn't able to find CF during a `cargo test` run on Darwin.
        export NIX_LDFLAGS="-F${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -framework CoreFoundation $NIX_LDFLAGS"
      ''
    );

    passthru = {
      inherit
        ci
        ;
    };

    preferLocalBuild = true;
    allowSubstitutes = false;
  }
  // (
    if isDevelopmentShell then {
      inherit RUST_SRC_PATH CARGO_INSTALL_ROOT;
    } else {}
  )
)
