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

      # CUDA toolkit is only needed for `cargo build -p fp-cuda` (the Hopper
      # wgmma.b1 backend). Kept out of `commonPackages` to avoid pulling
      # multi-GB CUDA into the `apps.test` closure used by CI.
      cudaPackages = [
        pkgs.cudaPackages.cudatoolkit
        # cuda-oxide's `cuda-bindings` crate runs `bindgen` against cuda.h,
        # which needs libclang at build time.
        pkgs.llvmPackages.libclang.lib
      ];
    in {
      devShells.default = pkgs.mkShell {
        packages = commonPackages ++ cudaPackages;
        shellHook = ''
          export RUST_LOG=info

          # CUDA: make nvcc find headers + libs, and satisfy cuda-oxide's
          # cuda-bindings build.rs (which reads CUDA_TOOLKIT_PATH, defaulting
          # to /usr/local/cuda otherwise).
          export CUDA_PATH=${pkgs.cudaPackages.cudatoolkit}
          export CUDA_TOOLKIT_PATH=${pkgs.cudaPackages.cudatoolkit}
          export CPATH="$CUDA_PATH/include''${CPATH:+:$CPATH}"
          export LIBRARY_PATH="$CUDA_PATH/lib64''${LIBRARY_PATH:+:$LIBRARY_PATH}"

          # libclang for bindgen (used by cuda-oxide's cuda-bindings crate).
          # libclang loaded as a .so doesn't pick up the wrapped clang's
          # auto-discovered libc/gcc include paths the way the clang binary
          # does, so we feed them via BINDGEN_EXTRA_CLANG_ARGS.
          export LIBCLANG_PATH=${pkgs.llvmPackages.libclang.lib}/lib
          export BINDGEN_EXTRA_CLANG_ARGS="$(< ${pkgs.stdenv.cc}/nix-support/libc-crt1-cflags) $(< ${pkgs.stdenv.cc}/nix-support/libc-cflags) $(< ${pkgs.stdenv.cc}/nix-support/cc-cflags) $(< ${pkgs.stdenv.cc}/nix-support/libcxx-cxxflags 2>/dev/null || true)"
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
