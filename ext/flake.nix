{
  description = "ext-rs dev shell";

  inputs = {
    super.url = "path:.."; # points to top-level flake
  };

  outputs = {super, ...}:
    super.flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import super.nixpkgs {inherit system;};

      # CUDA is unfree, so it needs its own nixpkgs instance. Only the `gpu` dev
      # shell pulls it in — the default shell and `nix run .#test` stay CUDA-free.
      cudaPkgs = import super.nixpkgs {
        inherit system;
        config.allowUnfree = true;
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

      # CUDA toolkit for the CubeCL `cuda` backend (algebra `gpu` feature).
      # `cubecl-cuda` JIT-compiles kernels with NVRTC — it needs `CUDA_PATH` to
      # point at a tree with `include/` (NVRTC `--include-path`) plus libnvrtc, and
      # drives them via the CUDA driver API (`libcuda`, supplied by the host NVIDIA
      # driver at /run/opengl-driver/lib, not nixpkgs). The monolithic `cudatoolkit`
      # gives one prefix with both headers and libs. cudarc dlopens the libs at
      # runtime (no build-time link), so only running — not building — needs this.
      cudatoolkit = cudaPkgs.cudaPackages.cudatoolkit;
    in {
      devShells.default = pkgs.mkShell {
        packages = commonPackages;
        shellHook = ''
          export RUST_LOG=info
        '';
      };

      # GPU dev shell: `nix develop .#gpu`. Adds the CUDA toolkit and points the
      # loader at both it and the host driver's libcuda.
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
