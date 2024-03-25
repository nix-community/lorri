{
  description = "test flake";
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ...}:
    flake-utils.lib.eachDefaultSystem (system: let
      mkShell = nixpkgs.legacyPackages.${system}.mkShell;
    in {
      devShell = mkShell {
          inherit system;
          env = {
            MARKER = "present";
            PATH = "./bin";
          };
        };
      });
}
