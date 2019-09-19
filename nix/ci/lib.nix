{ pkgs, writeExecline }:
let
  # Write commands to script which aborts immediately if a command is not successful.
  # The status of the unsuccessful command is returned.
  allCommandsSucceed = name: commands: pkgs.lib.pipe commands [
    (pkgs.lib.concatMap (cmd: [ "if" [ cmd ] ]))
    (cmds: cmds ++ [ "true" ])
    (writeExecline name {})
  ];

  # Takes a `mode` string and produces a script,
  # which modifies PATH given by $1 and execs into the rest of argv.
  # `mode`s:
  #   "set": overwrite PATH, set it to $1
  #   "append": append the given $1 to PATH
  #   "prepend": prepend the given $1 to PATH
  pathAdd = mode:
    let
      exec = [ "exec" "$@" ];
      importPath = [ "importas" "PATH" "PATH" ];
      set = [ "export" "PATH" "$1" ] ++ exec;
      append = importPath ++ [ "export" "PATH" ''''${PATH}:''${1}'' ] ++ exec;
      prepend = importPath ++ [ "export" "PATH" ''''${1}:''${PATH}'' ] ++ exec;
    in
      writeExecline "PATH_${mode}" { readNArgs = 1; }
        (
          if mode == "set" then set
          else if mode == "append" then append
          else if mode == "prepend" then prepend
          else abort "don’t know mode ${mode}"
        );

  # Takes a derivation and a list of binary names
  # and returns an attribute set of `name -> path`.
  # The list can also contain renames in the form of
  # `{ use, as }`, which goes `as -> usePath`.
  #
  # It is usually used to construct an attrset `bins`
  # containing all the binaries required in a file,
  # similar to a simple import system.
  #
  # Example:
  #
  #   bins = getBins pkgs.hello [ "hello" ]
  #       // getBins pkgs.coreutils [ "printf" "ln" "echo" ]
  #       // getBins pkgs.execline
  #            [ { use = "if"; as = "execlineIf" } ]
  #       // getBins pkgs.s6-portable-utils
  #            [ { use = "s6-test"; as = "test" }
  #              { use = "s6-cat"; as = "cat" }
  #            ];
  #
  #   provides
  #     bins.{hello,printf,ln,echo,execlineIf,test,cat}
  #
  getBins = drv: xs:
    let
      f = x:
      # TODO(Profpatsch): typecheck
        let
          x' = if builtins.isString x then { use = x; as = x; } else x;
        in
          {
            name = x'.as;
            value = "${pkgs.lib.getBin drv}/bin/${x'.use}";
          };
    in
      builtins.listToAttrs (builtins.map f xs);

in
{
  inherit
    allCommandsSucceed
    pathAdd
    getBins
    ;
}
