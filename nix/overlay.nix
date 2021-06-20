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
        lorri = import ../default.nix { pkgs = final; };
      }
    )
  ];
}
