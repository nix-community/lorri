{ pkgs ? import <nixpkgs> { }, ... }:

let
  buildGo = import ../nix/buildGo { inherit pkgs; };
  goDeps  = import ../nix/go-deps.nix { inherit pkgs; };
  # Runtime closure: the Nix file lorri passes to nix-instantiate as --argstr runTimeClosure.
  # Baked into the binary at link time via x_defs, mirroring how build.rs does it for Rust.
  rtc = pkgs.callPackage ../nix/runtime.nix {};
in
buildGo.program {
  name = "lorri";

  # Bake the runtime closure store path into the binary at link time.
  # Mirrors: pub const RUN_TIME_CLOSURE: &str = "..." in Rust's build.rs.
  x_defs."main.runtimeClosure" = "${rtc}";

  srcs = [
    ./runtime_closure.go  # var runtimeClosure + requireRTC() — must come first
    ./main.go
    ./abspath.go
    ./paths.go
    ./cas.go
    ./socket_framing.go
    ./socket_path.go
    ./communicate.go
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
    ./ops_watch.go
    ./ops_shell.go
    ./ops_prompt.go
  ];

  deps = [
    goDeps.fsnotify
    goDeps.golang-x-sys-unix
    goDeps.zombiezen-go-sqlite
    goDeps.zombiezen-go-sqlite.sqlitex
  ];
}
