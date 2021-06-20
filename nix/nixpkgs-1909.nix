let
  srcDef = builtins.fromJSON (builtins.readFile ./nixpkgs-1909.json);
  nixpkgs = builtins.fetchTarball {
    url = srcDef.url;
    sha256 = srcDef.sha256;
  };
in
import nixpkgs { overlays = [ (import ./overrides.nix) ]; }
