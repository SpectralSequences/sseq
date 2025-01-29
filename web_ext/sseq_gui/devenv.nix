{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.RUST_LOG = "info";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.hyperfine
    pkgs.python3Packages.flake8
    pkgs.python3Packages.black
    pkgs.python3Packages.pytest
    pkgs.python3Packages.selenium

    pkgs.openssl
  ];

  # https://devenv.sh/tests/
  enterTest = ''
    # Lints
    make lint
    make lint-selenium

    # Webserver
    cargo install wasm-bindgen-cli --debug
    make lint-wasm
    make wasm

    # Selenium
    make serve-wasm &
    (sleep 1 && make selenium)

    cargo build &&
    (target/debug/sseq_gui &
    (sleep 1 && make selenium))

    cargo build --features concurrent &&
    (target/debug/sseq_gui &
    (sleep 1 && make selenium))
  '';

  # https://devenv.sh/languages/
  languages = {
    rust = {
      enable = true;
      channel = "nightly";
      components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
      targets = [ "wasm32-unknown-unknown" ];
    };
    python = {
      enable = true;
    };
  };

  # See full reference at https://devenv.sh/reference/options/
}
