{ pkgs, lib, config, inputs, ... }:

{
  # https://devenv.sh/basics/
  env.RUST_LOG = "info";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
  ];

  # See full reference at https://devenv.sh/reference/options/
}
