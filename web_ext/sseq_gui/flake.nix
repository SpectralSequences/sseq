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
        # rust-src is needed for `-Z build-std`, which we use to rebuild the
        # standard library with `panic=unwind` for the wasm target (the
        # prebuilt std ships as `panic=abort`).
        fenixPkgs.complete.rust-src
      ];

      pythonEnv = pkgs.python3.withPackages (ps: [
        ps.flake8
        ps.black
        ps.pytest
        ps.selenium
        ps.webdriver-manager
      ]);

      commonPackages =
        [
          rustToolchain

          pythonEnv
          pkgs.openssl
          # wabt provides wasm-objdump, used by `make test-wasm-unwind` to
          # assert the wasm is actually built with unwinding support.
          pkgs.wabt
        ]
        ++ super.defaultPackages.devTools.${system};

      runTestScript = pkgs.writeShellScript "run-tests" ''
        set -euo pipefail

        export RUSTFLAGS="-D warnings"
        export RUSTDOCFLAGS="-D warnings"

        make lint
        make lint-selenium

        cargo install wasm-bindgen-cli --debug
        make lint-wasm
        make wasm
        make test-wasm-unwind

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
