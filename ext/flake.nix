{
  description = "ext-rs dev shell";

  inputs = {
    super.url = "path:.."; # points to top-level flake
  };

  outputs = {super, ...}:
    super.flake-utils.lib.eachDefaultSystem (system: let
      # Allow CUDA (unfree in nixpkgs). Scoped to the CUDA / NVIDIA prefix so
      # we don't accidentally unfree-allow anything else. nixpkgs splits the
      # toolkit into many sub-derivations (cuda_nvcc, cuda_cudart, cuda-merged,
      # cuda_cuobjdump, libcublas, ...) — listing them individually is whack-
      # a-mole, so we match by prefix.
      pkgs = import super.nixpkgs {
        inherit system;
        config.allowUnfreePredicate = pkg:
          let
            lib = super.nixpkgs.lib;
            name = lib.getName pkg;
          in
            lib.hasPrefix "cuda" name
            || lib.hasPrefix "libcu" name
            || lib.hasPrefix "libnv" name
            || lib.hasPrefix "libnpp" name;
      };

      pythonEnv = pkgs.python3.withPackages (ps: [
        ps.black
        ps.pytest
      ]);

      commonPackages =
        [
          super.defaultPackages.rustToolchain.${system}

          pythonEnv

          pkgs.cargo-cache
          pkgs.cargo-criterion
          pkgs.cargo-flamegraph
          pkgs.cargo-nextest
          pkgs.perf
        ]
        ++ super.defaultPackages.devTools.${system};

      # CUDA toolkit for building fp-cuda's Hopper wgmma.b1 kernel: nvcc + headers
      # at build time. Kept out of `commonPackages` (and the default shell) so
      # contributors and the `apps.test`/CI closure don't fetch the multi-GB unfree
      # CUDA tree for the opt-in backend. cudarc dlopens libcuda at runtime, so
      # only running — not building the Rust — needs the host driver.
      cudatoolkit = pkgs.cudaPackages.cudatoolkit;
    in {
      devShells.default = pkgs.mkShell {
        packages = commonPackages;
        shellHook = ''
          export RUST_LOG=info
        '';
      };

      # GPU dev shell: `nix develop .#gpu`. Adds the CUDA toolkit (nvcc + headers)
      # and points the loader at both it and the host driver's libcuda.
      devShells.gpu = pkgs.mkShell {
        packages = commonPackages ++ [cudatoolkit];
        shellHook = ''
          export RUST_LOG=info
          export CUDA_PATH="${cudatoolkit}"
          export LD_LIBRARY_PATH="${cudatoolkit}/lib:/run/opengl-driver/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        '';
      };

      apps.test = {
        type = "app";
        packages = commonPackages;
        program = toString (pkgs.writeShellScript "run-tests" ''
          set -euo pipefail

          export RUSTFLAGS="-D warnings"
          export RUSTDOCFLAGS="-D warnings"

          just lint
          just test
          just benchmarks
          just benchmarks-nassau
          just benchmarks-concurrent
          just miri
        '');
      };
    });
}
