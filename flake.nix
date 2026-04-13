{
  description = "A fast lorri daemon written in Go";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = pkgs.callPackage ./default.nix { inherit pkgs; };
        });

      devShells = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          # Full interactive development shell.
          default = pkgs.callPackage ./shell.nix { inherit pkgs; };

          # Minimal shell used by CI: just enough to run go test ./...
          ci = pkgs.mkShell {
            name = "lorri-ci";
            packages = with pkgs; [ go nix direnv git bash yj ];
            RUN_TIME_CLOSURE = pkgs.callPackage ./nix/runtime.nix {};
            LORRI_ROOT       = toString ./.;
            NIX_PATH         = "nixpkgs=${nixpkgs}";
          };
        });
    };
}
