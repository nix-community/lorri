{ pkgs, LORRI_ROOT, RUN_TIME_CLOSURE }:
let

  lib = pkgs.lib;

  inherit (import ../lib { inherit pkgs; })
    allCommandsSucceed
    pathAdd
    getBins
    pathPrependBins
    writeExecline
    ;

  bins = getBins pkgs.shellcheck [ "shellcheck" ]
      // getBins pkgs.gitMinimal [ "git" ]
      // getBins pkgs.bats [ "bats" ]
      // getBins pkgs.coreutils [ "test" "echo" "cat" "mkdir" "mv" "touch" ]
      // getBins pkgs.go [ "go" ]
      ;

  inherit (import ./sandbox.nix { inherit pkgs writeExecline; })
    runInEmptyEnv
    ;

  # shellcheck a file
  shellcheck = file: writeExecline "lint-shellcheck" {} [
    "cd" LORRI_ROOT
    "if" [ bins.echo "shellchecking ${file}" ]
    bins.shellcheck "--shell" "bash" file
  ];

  # the CI tests we want to run
  # Tests should not depend on each other (or block if they do),
  # so that they can run in parallel.
  # If a test changes files in the repository, sandbox it.

  tests = {

    go-test = {
      description = "run go test ./...";
      test = writeExecline "go-test" {}
        (pathPrependBins [ pkgs.go pkgs.nix pkgs.direnv pkgs.git pkgs.coreutils pkgs.bash ]
        ++ [
          "export" "RUN_TIME_CLOSURE" RUN_TIME_CLOSURE
          "cd" LORRI_ROOT
          bins.go "test" "./..."
        ]);
    };

    ci-script = offlineCheck.test {
      name = "lint-ci-script";
      description = "check ci.json is up to date";
      test = { ok, err }:
        writeExecline "ci-script" {}
          (pathPrependBins [ pkgs.go ]
          ++ [
            "cd" LORRI_ROOT
            "ifelse" [ bins.go "run" "./cmd/ci" "--check" ]
            [ ok ]
            "if" [ bins.echo "NOTE: Please run 'go run ./cmd/ci' to re-generate ci.json" ]
            err
          ]);
    };

  };

  # Tests that don't need to be run on different CI runners,
  # and that don't take a long time to be red (so we don't have to wait for them).
  # Also tests that are somewhat annoying but should be fixed nonetheless.
  tests-simple-checks = {

    shellcheck =
      let files = [
        "nix/bogus-nixpkgs/builder.sh"
        "envrc.bash"
      ];
      in {
        description = "shellcheck ${pkgs.lib.concatStringsSep " and " files}";
        test = allCommandsSucceed "lint-shellcheck-all" (map shellcheck files);
      };

  };

  # An offline check is a check that can be run inside a nix build.
  # But instead of crashing the nix build, it will write the result to $out
  # and generate a test runner that will just print the script.
  # This means we don't have to run the check on CI every time
  # if the nix build inputs didn't change.
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
          # We require the use of ok/err, otherwise it's too easy to accidentally
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

  # Remove tests that cannot succeed on certain platforms
  limitTests = if pkgs.stdenv.isLinux then n: v: true else n: v: true;
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
      # add a few things to bats' path that should really be patched upstream instead
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
  testsuite-simple-checks = batsScript "run-testsuite-simple-checks" tests-simple-checks;

  # we want the single test attributes to have their environment emptied as well.
  tests = testsWithEmptyEnv;
}
