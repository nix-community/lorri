{ pkgs, LORRI_ROOT, BUILD_REV_COUNT, RUN_TIME_CLOSURE }:
let

  lib = pkgs.lib;
  lorriBinDir = "${LORRI_ROOT}/target/debug";

  inherit (import ./execline.nix { inherit pkgs; })
    writeExecline;

  inherit (import ./lib.nix { inherit pkgs writeExecline; })
    allCommandsSucceed
    pathAdd
    getBins
    pathPrependBins
    ;

  bins = getBins pkgs.shellcheck [ "shellcheck" ]
      // getBins pkgs.gitMinimal [ "git" ]
      // getBins pkgs.mandoc [ "mandoc" ]
      // getBins pkgs.cargo [ "cargo" ]
      // getBins pkgs.gnused [ "sed" ]
      // getBins pkgs.bats [ "bats" ]
      // getBins pkgs.coreutils [ "test" "echo" "cat" "mkdir" "mv" "touch" ]
      // getBins pkgs.diffutils [ "diff" ]
      ;

  inherit (import ./sandbox.nix { inherit pkgs writeExecline; })
    runInEmptyEnv
    runWithoutNetwork
    ;

  # shellcheck a file
  shellcheck = file: writeExecline "lint-shellcheck" {} [
    "cd" LORRI_ROOT
    "if" [ bins.echo "shellchecking ${file}" ]
    bins.shellcheck "--shell" "bash" file
  ];

  # Dump the environment inside a `stdenv.mkDerivation` builder
  # into an envdir (can be read in again with `s6-envdir`).
  # This captures all magic `setupHooks` and linker paths and the like.
  stdenvDrvEnvdir = drvAttrs: pkgs.stdenv.mkDerivation ({
    name = "dumped-env";
    phases = [ "buildPhase" ];
    buildPhase = ''
      mkdir $out
      unset HOME TMP TEMP TEMPDIR TMPDIR
      # unset user-requested variables as well
      unset ${pkgs.lib.concatStringsSep " "
        # if these are set, the non-sandboxed test build complains about
        # linker paths outside of the nix store.
        [ "NIX_ENFORCE_PURITY" "NIX_SSL_CERT_FILE" "SSL_CERT_FILE" ]
      }
      ${pkgs.s6-portable-utils}/bin/s6-dumpenv $out
    '';
  } // drvAttrs);

  # On linux we need to setup a viable CC environment for compilation.
  linuxCCEnv = stdenvDrvEnvdir {};

  # On darwin we have to get the system libraries
  # from their setup hooks, by exporting the variables
  # from the builder.
  # Otherwise building & linking the rust binaries fails.
  darwinImpureEnv = stdenvDrvEnvdir {
    buildInputs = [
      # TODO: duplicated in shell.nix and default.nix
      pkgs.darwin.Security
      pkgs.darwin.apple_sdk.frameworks.Security
      pkgs.darwin.apple_sdk.frameworks.CoreServices
      pkgs.darwin.apple_sdk.frameworks.CoreFoundation
      pkgs.stdenv.cc.bintools.bintools
      pkgs.libiconv
    ];
  };

  # import an envdir, as e.g. produced by stdenvDrvEnvdir
  importDrvEnvdir = envdir: [
    "importas" "OLDPATH" "PATH"
    "${pkgs.s6}/bin/s6-envdir" envdir
    (pathAdd "prepend") "$OLDPATH"
  ];

  cargoEnvironment =
    # set the environment to a normal CC builder environment.
    importDrvEnvdir (if pkgs.stdenv.isDarwin then darwinImpureEnv else linuxCCEnv)
    ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
       # TODO: duplicated in default.nix
       # Cargo wasn't able to find CF during a `cargo test` run on Darwin.
       # see https://stackoverflow.com/questions/51161225/how-can-i-make-macos-frameworks-available-to-clang-in-a-nix-environment
       "importas" "NIX_LDFLAGS" "NIX_LDFLAGS"
       "export" "NIX_LDFLAGS" "-F${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -framework CoreFoundation \${NIX_LDFLAGS}"
       # Same for core services
       "export" "NIX_LDFLAGS" "-F${pkgs.darwin.apple_sdk.frameworks.CoreServices}/Library/Frameworks -framework CoreServices \${NIX_LDFLAGS}"
    ])
    # we have to add the bin to PATH,
    # otherwise cargo doesn’t find its subcommands
    ++ (pathPrependBins [
        # all cargo executables call themselves recursively, smh
        pkgs.rustc
        pkgs.gcc
        pkgs.rustfmt
        pkgs.clippy
        pkgs.cargo
    ])
    ++ [
      "export" "RUST_BACKTRACE" "full"
      "export" "BUILD_REV_COUNT" (toString BUILD_REV_COUNT)
      "export" "RUN_TIME_CLOSURE" RUN_TIME_CLOSURE
    ];

  writeCargo = name: setup: args:
    writeExecline name {} (cargoEnvironment ++ setup ++ [ bins.cargo ] ++ args);

  # the CI tests we want to run
  # Tests should not depend on each other (or block if they do),
  # so that they can run in parallel.
  # If a test changes files in the repository, sandbox it.

  tests = {

    shellcheck =
      let files = [
        "nix/bogus-nixpkgs/builder.sh"
        "src/ops/direnv/envrc.bash"
      ];
      in {
        description = "shellcheck ${pkgs.lib.concatStringsSep " and " files}";
        test = allCommandsSucceed "lint-shellcheck-all" (map shellcheck files);
      };

    cargo-fmt = {
      description = "cargo fmt was done";
      test = writeCargo "lint-cargo-fmt" [] [ "fmt" "--" "--check" ];
    };

    # TODO: it would be good to sandbox this (it changes files in the tree)
    # can crate2nix generate nix files without any compilation?
    crate2nix = {
      description = "check crate2nix up-to-date";
      test = writeExecline "lint-crate2nix" {}
        (pathPrependBins [pkgs.crate2nix]
        ++ [
          "if" [ pkgs.runtimeShell "${LORRI_ROOT}/nix/update-nix.sh" ]
          bins.git "diff" "--exit-code"
        ]);
    };

    ci-script = offlineCheck.test {
      name = "lint-ci-script";
      description = "check ci script was generated";
      test = { ok, err }:
        let yaml = (import ../../.github/workflows/ci.nix { inherit pkgs; }).yaml;
        in writeExecline "ci-script" {} [
          "ifelse" [bins.diff "--" ../../.github/workflows/ci.yml yaml]
          [ ok ]
          "if" [ bins.echo ''NOTE: Please run `nix-build ./.github/workflows/ci.nix -A writeConfig && ./result"` to re-generate the CI yaml file'' ]
          err
      ];
    };


    lint-manpage = offlineCheck.test {
      name = "lint-manpage";
      description = "lint the manpage";
      test = { ok, err }: pkgs.writers.writeDash "mandoc-lint" ''
        lint_warnings="$(
          ${bins.mandoc} -Tlint < ${../../lorri.1} \
            | ${bins.sed} -e '/referenced manual not found/d'
        )"

        # only succeed if theer were no warnings
        if [ ! -z "$lint_warnings" ]; then
          echo "$lint_warnings" >&2
          ${err}
        else
          ${ok}
        fi
      '';
    };

    cargo-test = {
      description = "run cargo test";
      test = writeCargo "cargo-test"
        # the tests need bash and nix and direnv
        (pathPrependBins [ pkgs.coreutils pkgs.bash pkgs.nix pkgs.direnv ])
        [ "test" "--no-fail-fast" ];
    };

    cargo-clippy = {
      description = "run cargo clippy";
      test = writeCargo "cargo-clippy" [
        "if" [ "env" ]
        # print the absolute path of the cargo-clippy version used
        "if" [ "sh" "-c" "type cargo-clippy" ]
        # first make sure all packages are fetched already
        (writeCargo "cargo-fetch" [] [ "fetch" ])
        # turn of network to prevent clippy from downloading anything
        runWithoutNetwork
        "if" [ "cargo-clippy" "--version" ]
        "export" "RUSTFLAGS" "-D warnings"
      ] [ "clippy" "--offline" ];
    };

  };

  eachUnitTest = let
    troubled = import ./troubled-tests.nix;

    tests = lib.imap1 (idx: test: {
      name = "test ${toString idx}";
      value = {
        description = "run cargo test ${test}";
        test = writeCargo "cargo-test"
          # the tests need bash and nix and direnv
          (pathPrependBins [ pkgs.coreutils pkgs.bash pkgs.nix pkgs.direnv ])
          [ "test" "--" "--include-ignored" test ];
        };
      }) troubled;
  in
    lib.listToAttrs tests;

  # An offline check is a check that can be run inside a nix build.
  # But instead of crashing the nix build, it will write the result to $out
  # and generate a test runner that will just print the script.
  # This means we don’t have to run the check on CI every time
  # if the nix build inputs didn’t change.
  offlineCheck = {

    # create an offline check test
    # the test is passed `{ ok, err }`, which are the commands to call
    # at the end, depending on whether the test succeeded or failed.
    test = { name, description, test }: {
      inherit description;
      test =
        let genResult = pkgs.runCommandLocal "${name}-result" {} ''
          mkdir -p "$out"
          set +e
          ${test { ok = offlineCheck.ok; err = offlineCheck.err; }} \
            2> "$out/stderr"
          code=$?
          set -e
          # should the test exit 123 by chance, this check will not work, but better than nothing
          # We require the use of ok/err, otherwise it’s too easy to accidentally
          # succeed tests in scripts (e.g. forgot "set -e").
          if [ ! $code -eq 123 ]; then
            echo "offlineCheck: please call ok or err in order to finish the test" >&2
            echo "test finished with exit code: $code" >&2
            exit 100
          fi
        '';
        in writeExecline name {} [
          offlineCheck.checkResult genResult
        ];
    };

    # end the test successfully
    ok = writeExecline "offline-check-ok" {} [
      "importas" "-ui" "out" "out"
      "if" [ bins.mkdir "-p" "$out" ]
      "if" [ bins.touch "\${out}/ok" ]
      "exit" "123"
    ];
    # end the test with an error, the output of stderr will be the test result
    err = writeExecline "offline-check-ok" {} [
      "importas" "-ui" "out" "out"
      "if" [ bins.echo "The test signaled an error, finishing." ]
      "if" [ bins.touch "\${out}/err" ]
      "exit" "123"
    ];

    # check whether the result of the test was successful or not
    checkResult = writeExecline "offline-check-getResult" { readNArgs = 1; } [
      "ifelse"
          [ bins.test "-e" "\${1}/err" ]
        [ "if" [ "redirfd" "-r" "0" "\${1}/stderr" bins.cat ]
          "exit" "1"
        ]
      "ifelse"
          [ bins.test "-e" "\${1}/ok" ]
        # write error message to stderr
        [ "exit" "0" ]
      "redirfd" "-w" "2"
      bins.echo "neither err no ok files existed, should not happen"
      "exit" "101"
    ];
  };

  # Remove tests that cannot succeed
  limitTests = if pkgs.stdenv.isLinux then n: v: true else n: v: !(builtins.elem n [
    "cargo-clippy" # requires bubblewrap
  ]);
  limitedTests = lib.filterAttrs limitTests tests;


  # clean the environment;
  # this is the only way we can have a non-diverging
  # environment between developer machine and CI
  emptyTestEnv = test:
    writeExecline "${test.name}-empty-env" {}
      [ (runInEmptyEnv [ "USER" "HOME" "TERM" ]) test ];

  testsWithEmptyEnv = tests: pkgs.lib.mapAttrs
    (_: test: test // { test = emptyTestEnv test.test; }) tests;

  # Write a attrset which looks like
  # { "test description" = test-script-derviation }
  # to a script which can be read by `bats` (a simple testing framework).
  batsScript =
    let
      # add a few things to bats’ path that should really be patched upstream instead
      # TODO: upstream
      bats = writeExecline "bats" {}
        (pathPrependBins [ pkgs.coreutils pkgs.gnugrep ]
        ++ [ "${pkgs.bats}/bin/bats" "$@" ]);
      # see https://github.com/bats-core/bats-core/blob/f3a08d5d004d34afb2df4d79f923d241b8c9c462/README.md#file-descriptor-3-read-this-if-bats-hangs
      closeFD3 = "3>&-";
    in name: tests: pkgs.lib.pipe tests [
      testsWithEmptyEnv
      (pkgs.lib.mapAttrsToList
        # a bats test looks like:
        # @test "name of test" {
        #   … test code …
        # }
        # bats is very picky about the {} block (and the newlines).
        (_: test: "@test ${pkgs.lib.escapeShellArg test.description} {\n${test.test} ${closeFD3}\n}"))
      (pkgs.lib.concatStringsSep "\n")
      (pkgs.writeText "testsuite")
      (test-suite: writeExecline name {} [
        bats
          "--tap"
          test-suite
      ])
    ];

in {
  testsuite = batsScript "run-testsuite" limitedTests;
  testsuite-eachunit = batsScript "each-unit-test" eachUnitTest;

  # we want the single test attributes to have their environment emptied as well.
  tests = testsWithEmptyEnv;
  inherit darwinImpureEnv;
}
