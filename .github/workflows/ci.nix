# Usage:
# nix-build ./ci.nix; ./result
{ pkgs ? import ../../nix/nixpkgs-stable.nix {} }:
let
  checkout = { fetch-depth ? null }: {
    name = "Checkout";
    uses = "actions/checkout@v4";
    "with" = {
      inherit fetch-depth;
    };
  };
  setup-nix = {
    name = "Nix";
    uses = "cachix/install-nix-action@v26";
  };
  create-nix-gcroots-user-dir = {
    name = "Create user gcroot dir";
    run = ''
      sudo mkdir /nix/var/nix/gcroots/per-user/$USER
      sudo chown $USER /nix/var/nix/gcroots/per-user/$USER
    '';
  };
  setup-cachix = {
    name = "Cachix";
    uses = "cachix/cachix-action@v14";
    "with" = {
      name = "nix-community";
      authToken = "\${{ secrets.CACHIX_AUTH_TOKEN }}";
    };
  };
  # required to set up rust-cache
  add-rustc-to-path = {
    name = "Add rustc to PATH";
    run = ''
      set -euo pipefail
      rustc_path="$(nix-build -A rustc nix/nixpkgs-stable.nix)/bin"
      echo "$rustc_path" >> "$GITHUB_PATH"
    '';
  };
  print-path = {
    name = "print PATH";
    run = "printenv PATH";
  };
  rust-cache = {
    name = "Rust Cache";
    uses = "Swatinem/rust-cache@v2.7.3";
  };

  githubRunners = {
    ubuntu = "ubuntu-latest";
    macos = "macos-latest";
  };

  builds = {
    rust = { runs-on }: {
      name = "rust-${runs-on}";
      value = {
        name = "Rust and CI tests (${runs-on})";
        inherit runs-on;
        steps = [
          (checkout {})
          setup-nix
          create-nix-gcroots-user-dir
          setup-cachix
          add-rustc-to-path
          print-path
          rust-cache
          {
            name = "CI tests";
            run = ''
              nix-build \
                --out-link ./ci-tests \
                --arg isDevelopmentShell false \
                -A ci.testsuite \
                shell.nix \
                && ./ci-tests
            '';
          }
        ];
      };
    };

    rust-each-unit-test = { runs-on }: {
      name = "rust-each-unit-${runs-on}";
      value = {
        name = "Rust individual tests (${runs-on})";
        inherit runs-on;
        steps = [
          (checkout {})
          setup-nix
          create-nix-gcroots-user-dir
          setup-cachix
          add-rustc-to-path
          print-path
          rust-cache
          {
            name = "CI bisection tests";
            run = ''
              nix-build \
                --out-link ./ci-tests \
                --arg isDevelopmentShell false \
                -A ci.testsuite-eachunit \
                shell.nix \
                && ./ci-tests
            '';
          }
        ];
      };
    };

    stable = { runs-on }: {
      name = "nix-build_stable-${runs-on}";
      value = {
        name = "nix-build [nixos stable] (${runs-on})";
        inherit runs-on;
        steps = [
          (
            checkout {
              # required for lorri self-upgrade local
              fetch-depth = 0;
            }
          )
          setup-nix
          create-nix-gcroots-user-dir
          setup-cachix
          {
            name = "Build";
            run = "nix-build";
          }
          {
            name = "Install";
            run = "nix-env -i ./result";
          }
          {
            name = "Self-upgrade";
            run = "lorri self-upgrade local \$(pwd)";
          }
        ];
      };
    };

    overlay = { runs-on }: {
      name = "overlay-${runs-on}";
      value = {
        name = "Overlay builds (${runs-on})";
        inherit runs-on;
        steps = [
          (checkout {})
          setup-nix
          create-nix-gcroots-user-dir
          setup-cachix
          {
            name = "Build w/ overlay (stable)";
            run = "nix-build ./nix/overlay.nix -A lorri --arg pkgs ./nix/nixpkgs-stable.json";
          }
        ];
      };
    };
  };

  config = {
    name = "CI";
    on = {
      pull_request = { branches = [ "**" ]; };
      push = { branches = [ "master" ]; };
      workflow_dispatch = {};
    };
    env = { LORRI_NO_INSTALL_PANIC_HANDLER = "absolutely"; };

    jobs = builtins.listToAttrs
    [
      (builds.rust { runs-on = githubRunners.ubuntu; })
      (builds.rust { runs-on = githubRunners.macos; })
      # This is meant to cover the one weird panicy test in src/watch.rs
      (builds.rust-each-unit-test { runs-on = githubRunners.macos; })
      (builds.stable { runs-on = githubRunners.ubuntu; })
      (builds.stable { runs-on = githubRunners.macos; })
      (builds.overlay { runs-on = githubRunners.ubuntu; })
      (builds.overlay { runs-on = githubRunners.macos; })
    ];
  };

  yaml = pkgs.runCommand "ci.yml" {

    buildInputs = [ pkgs.yj ];
    passAsFile = [ "config" ];
    config = builtins.toJSON config;
    preferLocalBuild = true;
    allowSubstitutes = false;
  }
    ''
      yj -jy < $configPath > $out
    '';

  # writes the file to the right path (toString is the absolute local path)
  writeConfig = pkgs.writers.writeDash "write-ci.yml" ''
    ${pkgs.coreutils}/bin/cat "${yaml}" > "${toString ./ci.yml}"
  '';

in {
  inherit
    yaml
    writeConfig
    ;
}
