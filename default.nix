{ nixpkgs ? ./nix/nixpkgs-stable.nix
, pkgs ? import nixpkgs {
    # This is a hack to work around something requiring libcap on MacOS
    config.allowUnsupportedSystem = true;
  }
}:
let
  src = pkgs.nix-gitignore.gitignoreSource [
    ".git/"
    ".github/"
    "assets/"
  ] ./.;
  cargoLorri =
    (
      import ./Cargo.nix {
        inherit pkgs;
      }
    ).rootCrate.build;

in
cargoLorri.override {
  crateOverrides = pkgs.defaultCrateOverrides // {
    lorri = attrs: {
      name = "lorri";

      src = pkgs.nix-gitignore.gitignoreSource  [ ".git" "target" "/*.nix" ] ./.;

      # add man and doc outputs to put our documentation into
      outputs = cargoLorri.outputs ++ [ "man" "doc" ];

      RUN_TIME_CLOSURE = pkgs.callPackage ./nix/runtime.nix {};
      NIX_PATH = "nixpkgs=${./nix/bogus-nixpkgs}";

      # required by human-panic, because the nix generator doesn’t
      # set the cargo environment variables correctly
      # (TODO: does crate2nix do it? carnix didn’t.)
      # see https://doc.rust-lang.org/cargo/reference/environment-variables.html
      homepage = "https://github.com/nix-community/lorri";

      preConfigure = ''
        . ${./nix/pre-check.sh}

        # Do an immediate, light-weight test to ensure logged-evaluation
        # is valid, prior to doing expensive compilations.
        nix-build --show-trace ./src/logged-evaluation.nix \
          --arg src ./tests/integration/basic/shell.nix \
          --arg runTimeClosure "$RUN_TIME_CLOSURE" \
          --no-out-link
      '';

      buildInputs = [
        pkgs.nix # required for the preConfigure test
        pkgs.rustPackages.rustfmt
      ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
        pkgs.darwin.Security
        pkgs.darwin.apple_sdk.frameworks.CoreServices
        pkgs.libiconv
      ];
      nativeBuildInputs = [ pkgs.installShellFiles ];

      postInstall = ''
        # copy the docs to the $man and $doc outputs
        ${pkgs.scdoc}/bin/scdoc < lorri.scd > lorri.1
        install -Dm644 lorri.1 $man/share/man/man1/lorri.1
        install -Dm644 -t $doc/share/doc/lorri/ \
          README.md \
          CONTRIBUTING.md \
          LICENSE \
          MAINTAINERS.md
        cp -r contrib/ $doc/share/doc/lorri/contrib

        # install shell completions via the internal command
        mkdir out-completions
        cd out-completions
        # work around /homeless-shelter not being writable on darwin nix builds
        export HOME=$(pwd)
        $out/bin/lorri internal write-shell-completion-scripts
        echo installing shell completions for ./*

        # || true because installShellCompletion is buggy as hell and I don’t care
        # (also it only supports bash/zsh/fish and not e.g. elvish, even though clap_completion does)
        installShellCompletion ./* || true
      '';
    };
  };
}
