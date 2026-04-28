{ pkgs ? import <nixpkgs> { }, ... }:

let
  buildGo = import ./nix/buildGo { inherit pkgs; };
  goDeps  = import ./nix/go-deps.nix { inherit pkgs; };
  # Runtime closure: the Nix file lorri passes to nix-instantiate as --argstr runTimeClosure.
  # Baked into the binary at link time via x_defs.
  rtc = pkgs.callPackage ./nix/runtime.nix {};

  # Vendored direnv shell hook and export machinery (MIT licence).
  direnvVendor = buildGo.package {
    name    = "direnv_vendor";
    path    = "github.com/nix-community/lorri/direnv_vendor";
    srcs    = [
      ./direnv_vendor/shell.go
      ./direnv_vendor/shell_bash.go
      ./direnv_vendor/shell_elvish.go
      ./direnv_vendor/shell_fish.go
      ./direnv_vendor/shell_json.go
      ./direnv_vendor/shell_murex.go
      ./direnv_vendor/shell_pwsh.go
      ./direnv_vendor/shell_systemd.go
      ./direnv_vendor/shell_tcsh.go
      ./direnv_vendor/shell_vim.go
      ./direnv_vendor/shell_zsh.go
      ./direnv_vendor/log.go
    ];
  };

  lorriBin = buildGo.program {
    name = "lorri";

    # Bake the runtime closure store path into the binary at link time.
    x_defs."main.runtimeClosure" = "${rtc}";

    srcs = [
      ./main.go
      ./abspath.go
      ./paths.go
      ./communicate.go
      ./logged-evaluation.nix
      ./builder.go
      ./watch.go
      ./build_loop.go
      ./lorri_db.go
      ./daemon.go
      ./trivial-shell.nix
      ./envjson.go
      ./export.go
    ];

    deps = [
      direnvVendor
      goDeps.fsnotify
      goDeps.golang-x-sys-unix
      goDeps.zombiezen-go-sqlite
      goDeps.zombiezen-go-sqlite.sqlitex
    ];
  };
in
# Wrap the binary together with shell completion files and the man page.
pkgs.symlinkJoin {
  name = "lorri";
  paths = [ lorriBin ];
  nativeBuildInputs = [ pkgs.scdoc ];
  postBuild = ''
    scdoc < ${./lorri.scd} > lorri.1
    install -Dm644 lorri.1 $out/share/man/man1/lorri.1

    install -Dm644 ${./contrib/lorri.bash} \
      $out/share/bash-completion/completions/lorri
    install -Dm644 ${./contrib/lorri.zsh} \
      $out/share/zsh/site-functions/_lorri
    install -Dm644 ${./contrib/lorri.fish} \
      $out/share/fish/vendor_completions.d/lorri.fish
    install -Dm644 ${./contrib/lorri.elv} \
      $out/share/elvish/completions/lorri.elv
    install -Dm644 ${./contrib/lorri.tcsh} \
      $out/share/tcsh-completion/completions/lorri.tcsh
    install -Dm644 ${./contrib/lorri.mx} \
      $out/share/murex/completions/lorri.mx
    install -Dm644 ${./contrib/lorri.ps1} \
      $out/share/powershell/completions/lorri.ps1
  '';
}
