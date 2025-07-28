{
  description = "ext-rs dev shell";

  inputs = {
    super.url = "path:.."; # points to top-level flake
  };

  outputs = {super, ...}:
    super.flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import super.nixpkgs {inherit system;};

      pythonEnv = pkgs.python3.withPackages (ps: [
        ps.black
        ps.pytest
        ps.ruff
      ]);

      commonPackages = [
        super.defaultPackages.rustToolchain.${system}
        super.defaultPackages.devTools.${system}

        pythonEnv

        pkgs.cargo-cache
        pkgs.cargo-criterion
        pkgs.cargo-flamegraph
        pkgs.cargo-nextest
        pkgs.linuxKernel.packages.linux_zen.perf

        pkgs.maturin
      ];
    in {
      devShells.default = pkgs.mkShell {
        packages = commonPackages;
        shellHook = ''
          export RUST_LOG=info
        '';
      };

      apps.test = {
        type = "app";
        packages = commonPackages;
        program = toString (pkgs.writeShellScript "run-tests" ''
          set -euo pipefail
        '');
      };
    });
}
