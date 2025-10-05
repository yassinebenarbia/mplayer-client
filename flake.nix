{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      naersk,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        naersk-lib = pkgs.callPackage naersk { };
        pkgs = import <nixpkgs-unstable> { };
      in
      {
        defaultPackage = naersk-lib.buildPackage {
          name = "mplayer-client";
          src = ./.;
          buildInputs = with pkgs; [
            pkg-config
            luajit
            luajitPackages.ldbus
            rustc
          ];
        };
        devShell =
          with pkgs;
          mkShell {
            buildInputs = [
              pkg-config
              luajit
              rustc
              rustfmt
              lsd
              luajitPackages.ldbus
            ];
            shellHook = ''
              export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:${pkgs.luajit}/lib/pkgconfig"
              alias ls="lsd"
            '';
          };
      }
    );
}
