{
  description = "sseq flake for monorepo";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    fenix,
    flake-utils,
    ...
  }: {
    # Expose inputs to subflakes
    inherit nixpkgs fenix flake-utils;

    defaultPackages = flake-utils.lib.eachDefaultSystem (system: {
      rustToolchain = fenix.packages.${system}.complete.withComponents [
        "rustc"
        "cargo"
        "clippy"
        "rustfmt"
        "rust-analyzer"
        "miri"
        "llvm-tools-preview"
      ];
      devTools = let
        pkgs = import nixpkgs {inherit system;};
      in [
        pkgs.git
        pkgs.hyperfine
        pkgs.binutils
        pkgs.cargo-binutils
        pkgs.cargo-edit
      ];
    });
  };
}
