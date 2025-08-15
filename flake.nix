{
  description = "Rust devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust-bin-tc = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      in
      {
        devShells.default = pkgs.mkShell {

          buildInputs = [
            pkgs.openssl
            pkgs.pkg-config
            pkgs.rust-analyzer
            rust-bin-tc

            # required for
            # cargo install --path ../cargo-call-stack
            pkgs.llvmPackages_20.libllvm
            pkgs.libxml2

            # for call stack graph visualization
            pkgs.graphviz
          ];
        };
      }
    );
}
