{
  # description = "flake file";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildINputs = with pkgs; [
          pkg-config
          luajit
          luajitPackages.ldbus
          rustc
        ];
        nativeBuildInputs = with pkgs; [
          pkg-config
          luajit
          rustc
          rustfmt
          lsd
          luajitPackages.ldbus
        ];
        shellHook = ''echo "Welcome!"'';

      };
    };
}
