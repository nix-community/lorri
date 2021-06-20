let
  srcDef = builtins.fromJSON (builtins.readFile ./nixpkgs-stable.json);
  nixpkgs = builtins.fetchTarball {
    url = srcDef.url;
    sha256 = srcDef.sha256;
  };
in
import nixpkgs { overlays = [ (import ./overrides.nix) ]; }
