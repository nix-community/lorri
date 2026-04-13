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
    ./panic.go
    ./communicate.go
    ./socket.go
    ./ops_ping.go
    ./ops_stream_events.go
    ./ops_direnv.go
    ./envrc.bash
    ./nix_options.go
    ./path_reduction.go
    ./logged-evaluation.nix
    ./builder.go
    ./watch.go
    ./build_loop.go
    ./lorri_db.go
    ./daemon.go
    ./ops_init.go
    ./trivial-shell.nix
    ./default-envrc
    ./ops_info.go
    ./ops_gc.go
    ./ops_prompt.go
  ];

  deps = [
    goDeps.fsnotify
    goDeps.golang-x-sys-unix
    goDeps.zombiezen-go-sqlite
    goDeps.zombiezen-go-sqlite.sqlitex
  ];
}
