{
  description = "GoXLR Utility (with configurable Fader Paging)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        goxlr-utility = pkgs.callPackage ./nix/package.nix { };
        default = goxlr-utility;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.callPackage ./nix/shell.nix { };
      });

      overlays.default = final: _prev: {
        goxlr-utility = final.callPackage ./nix/package.nix { };
      };

      # imports = [ goxlr-utility.nixosModules.default ]; services.goxlr-utility.enable = true;
      nixosModules.default = import ./nix/module.nix self;

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
