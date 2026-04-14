{ pkgs ? import <nixpkgs> { }, ... }:

let
  buildGo = import ./nix/buildGo { inherit pkgs; };
  goDeps  = import ./nix/go-deps.nix { inherit pkgs; };
  # Runtime closure: the Nix file lorri passes to nix-instantiate as --argstr runTimeClosure.
  # Baked into the binary at link time via x_defs.
  rtc = pkgs.callPackage ./nix/runtime.nix {};
in
buildGo.program {
  name = "lorri";

  # Bake the runtime closure store path into the binary at link time.
  x_defs."main.runtimeClosure" = "${rtc}";

  srcs = [
    ./main.go
    ./abspath.go
    ./paths.go
    ./communicate.go
    ./envrc.bash
    ./logged-evaluation.nix
    ./builder.go
    ./watch.go
    ./build_loop.go
    ./lorri_db.go
    ./daemon.go
    ./trivial-shell.nix
    ./default-envrc
  ];

  deps = [
    goDeps.fsnotify
    goDeps.golang-x-sys-unix
    goDeps.zombiezen-go-sqlite
    goDeps.zombiezen-go-sqlite.sqlitex
  ];
}
