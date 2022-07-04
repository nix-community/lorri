{ pkgs, writeExecline }:

let

  # remove everything but a few selected environment variables
  runInEmptyEnv = keepVars:
    let
        importas = pkgs.lib.concatMap (var: [ "importas" "-D" "" var var ]) keepVars;
        # we have to explicitly call export here, because PATH is probably empty
        export = pkgs.lib.concatMap (var: [ "${pkgs.execline}/bin/export" var ''''${${var}}'' ]) keepVars;
    in writeExecline "empty-env" {}
         (importas ++ [ "emptyenv" ] ++ export ++ [ "${pkgs.execline}/bin/exec" "$@" ]);

  runWithoutNetwork = writeExecline "run-without-network" {} [
    "${pkgs.bubblewrap}/bin/bwrap"
    "--dev-bind" "/" "/"
    "--unshare-net"
    "$@"
  ];


in {
  inherit
    runInEmptyEnv
    runWithoutNetwork
    ;
}
