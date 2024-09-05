{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };
      in
      with pkgs;
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "chrs";
          version = self.shortRev or self.dirtyShortRev;
          src = ./.;
          cargoBuildFlags = "--package chrs";
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [ pkg-config ];
          doCheck = false;
        };
      });
}
