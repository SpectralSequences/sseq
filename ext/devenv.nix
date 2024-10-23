{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.RUST_LOG = "info";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.hyperfine
    pkgs.python3Packages.pytest

    pkgs.cargo-cache
  ];

  # https://devenv.sh/tests/
  enterTest = ''
    # Lints
    make lint

    # Tests
    make test
    make benchmarks
    make benchmarks-nassau
    make benchmarks-concurrent

    # Miri
    make miri
  '';

  # https://devenv.sh/languages/
  languages = {
    rust = {
      enable = true;
      channel = "nightly";
      components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" "miri" ];
    };
    python = {
      enable = true;
    };
  };

  # See full reference at https://devenv.sh/reference/options/
}
