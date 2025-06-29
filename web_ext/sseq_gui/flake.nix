{
  description = "sseq_gui flake devshell";

  inputs = {
    super.url = "path:../.."; # points to top-level flake
  };

  outputs = {super, ...}:
    super.flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import super.nixpkgs {inherit system;};
      fenixPkgs = super.fenix.packages.${system};

      rustToolchain = fenixPkgs.combine [
        super.defaultPackages.rustToolchain.${system}
        fenixPkgs.targets.wasm32-unknown-unknown.latest.toolchain
      ];

      pythonEnv = pkgs.python3.withPackages (ps: [
        ps.flake8
        ps.black
        ps.pytest
        ps.selenium
      ]);

      commonPackages = [
        rustToolchain
        super.defaultPackages.devTools.${system}

        pythonEnv
        pkgs.openssl
      ];

      runTestScript = pkgs.writeShellScript "run-tests" ''
        set -euo pipefail

        make lint
        make lint-selenium

        cargo install wasm-bindgen-cli --debug
        make lint-wasm
        make wasm

        make serve-wasm &
        (sleep 1 && make selenium)

        cargo build &&
        (target/debug/sseq_gui &
         (sleep 1 && make selenium))

        cargo build --features concurrent &&
        (target/debug/sseq_gui &
         (sleep 1 && make selenium))
      '';
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
        program = toString runTestScript;
      };
    });
}
