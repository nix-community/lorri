{ pkgs }:
let
  srcDef = builtins.fromJSON (builtins.readFile pkgs);
  nixpkgs = builtins.fetchTarball {
    url = srcDef.url;
    sha256 = srcDef.sha256;
  };
in
import nixpkgs {
  overlays = [
    (
      final: super: {
        # Rust lorri (default, unchanged)
        lorri = import ../default.nix { pkgs = final; };
        # Go rewrite — same socket protocol, drop-in daemon replacement
        lorri-go = (import ../default.nix { pkgs = final; }).passthru.go;
      }
    )
  ];
}
