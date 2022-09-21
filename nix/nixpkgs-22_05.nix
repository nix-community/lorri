let
  srcDef = builtins.fromJSON (builtins.readFile ./nixpkgs-22_05.json);
  nixpkgs = builtins.fetchTarball {
    url = srcDef.url;
    sha256 = srcDef.sha256;
  };
in
args@{...}: import nixpkgs (args // { overlays = args.overlays or []; })
