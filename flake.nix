{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        unstable = import <nixpkgs-unstable> {};
        naersk-lib = pkgs.callPackage naersk {};
        pkgs = import <nixpkgs> {};
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          buildInputs = [  unstable.rustc ];
        };
      });
}
