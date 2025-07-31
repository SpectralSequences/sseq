{
  description = "sseq_ext development environment - Python bindings for ext crate";

  inputs = {
    super.url = "path:..";

    pyproject-nix = {
      url = "github:pyproject-nix/pyproject.nix";
      inputs.nixpkgs.follows = "super/nixpkgs";
    };

    uv2nix = {
      url = "github:pyproject-nix/uv2nix";
      inputs.pyproject-nix.follows = "pyproject-nix";
      inputs.nixpkgs.follows = "super/nixpkgs";
    };

    pyproject-build-systems = {
      url = "github:pyproject-nix/build-system-pkgs";
      inputs.pyproject-nix.follows = "pyproject-nix";
      inputs.uv2nix.follows = "uv2nix";
      inputs.nixpkgs.follows = "super/nixpkgs";
    };
  };

  outputs = {
    self,
    super,
    uv2nix,
    pyproject-nix,
    pyproject-build-systems,
    ...
  }:
    super.flake-utils.lib.eachDefaultSystem (system: let
      inherit (super.nixpkgs) lib;

      # Load a uv workspace from a workspace root.
      workspace = uv2nix.lib.workspace.loadWorkspace {workspaceRoot = ./.;};

      # Create package overlay from workspace.
      overlay = workspace.mkPyprojectOverlay {
        sourcePreference = "wheel";
      };

      # Extend generated overlay with build fixups
      pyprojectOverrides = _final: _prev: {
        # Implement build fixups here if needed.
      };

      pkgs = super.nixpkgs.legacyPackages.${system};
      python = pkgs.python311;

      # Construct package set
      pythonSet =
        (pkgs.callPackage pyproject-nix.build.packages {
          inherit python;
        }).overrideScope (
          lib.composeManyExtensions [
            pyproject-build-systems.overlays.default
            overlay
            pyprojectOverrides
          ]
        );

      # Additional packages for PyO3/maturin development
      commonPackages =
        [
          super.defaultPackages.rustToolchain.${system}
          python
          pkgs.python311Packages.pip
          pkgs.maturin
          pkgs.uv
          pkgs.basedpyright
          pkgs.ruff
          pkgs.pkg-config
        ]
        ++ super.defaultPackages.devTools.${system}
        ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ];
    in {
      packages.default = pythonSet.mkVirtualEnv "sseq_ext-env" workspace.deps.default;

      apps.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/python";
      };

      devShells = {
        # Impure development shell using uv and maturin
        default = pkgs.mkShell {
          packages = commonPackages;
          env =
            {
              UV_PYTHON_DOWNLOADS = "never";
              RUST_BACKTRACE = "1";
            }
            // lib.optionalAttrs pkgs.stdenv.isLinux {
              LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.openssl];
            };
          shellHook = ''
            unset PYTHONPATH

            # Create virtual environment if it doesn't exist
            if [ ! -d ".venv" ]; then
              echo "Creating Python virtual environment..."
              ${python}/bin/python -m venv .venv

              # Only upgrade pip/setuptools/wheel when creating new venv
              echo "Setting up virtual environment..."
              source .venv/bin/activate
              pip install --upgrade pip setuptools wheel
            else
              # Just activate existing venv
              source .venv/bin/activate
            fi

            # Override environment variables to ensure maturin uses the venv
            export VIRTUAL_ENV="$PWD/.venv"
            export PATH="$VIRTUAL_ENV/bin:$PATH"
            export UV_PYTHON="$VIRTUAL_ENV/bin/python"
            export PIP_PREFIX="$VIRTUAL_ENV"
            export PYTHONPATH="$VIRTUAL_ENV/lib/python3.11/site-packages"

            echo "🦀🐍 sseq_ext development environment"
            echo "Rust: $(rustc --version)"
            echo "Python: $(python --version) (virtual env at $VIRTUAL_ENV)"
            echo "Maturin: $(maturin --version)"
            echo "uv: $(uv --version)"
            echo ""
            echo "Quick start:"
            echo "  maturin develop            # Build and install in development mode"
            echo "  maturin build              # Build wheel"
            echo "  uv sync                    # Install dependencies"
            echo ""
            echo "For development:"
            echo "  cargo test                 # Run Rust tests"
            echo "  cargo clippy               # Run Rust linter"
            echo "  uv build                   # Build package"
            echo ""
            echo "Note: Using virtual environment at .venv/"
            echo "Python path: $(which python)"
            echo "Pip path: $(which pip)"
          '';
        };

        # Pure development shell using uv2nix
        pure = let
          # Create an overlay enabling editable mode for local dependencies.
          editableOverlay = workspace.mkEditablePyprojectOverlay {
            root = "$REPO_ROOT";
          };

          # Override previous set with our editable overlay.
          editablePythonSet = pythonSet.overrideScope (
            lib.composeManyExtensions [
              editableOverlay
              (final: prev: {
                sseq_ext = prev.sseq_ext.overrideAttrs (old: {
                  src = lib.fileset.toSource {
                    root = old.src;
                    fileset = lib.fileset.unions [
                      (old.src + "/pyproject.toml")
                      (old.src + "/Cargo.toml")
                      (old.src + "/Cargo.lock")
                      (old.src + "/src")
                      (old.src + "/examples")
                    ];
                  };
                  nativeBuildInputs =
                    old.nativeBuildInputs
                    ++ final.resolveBuildSystem {
                      maturin = [];
                    }
                    ++ [super.defaultPackages.rustToolchain.${system}];
                });
              })
            ]
          );

          # Build virtual environment, with local packages being editable.
          virtualenv = editablePythonSet.mkVirtualEnv "sseq_ext-dev-env" workspace.deps.all;
        in
          pkgs.mkShell {
            packages = [
              virtualenv
              super.defaultPackages.rustToolchain.${system}
              super.defaultPackages.devTools.${system}
              pkgs.uv
              pkgs.basedpyright
              pkgs.ruff
              pkgs.git
              pkgs.pkg-config
            ];

            env = {
              UV_NO_SYNC = "1";
              UV_PYTHON = "${virtualenv}/bin/python";
              UV_PYTHON_DOWNLOADS = "never";
              RUST_BACKTRACE = "1";
            };

            shellHook = ''
              unset PYTHONPATH
              export REPO_ROOT=$(git rev-parse --show-toplevel)
              echo "🦀🐍 sseq_ext pure development environment (uv2nix)"
              echo "Rust: $(rustc --version)"
              echo "Python: $(python --version)"
              echo "uv: $(uv --version)"
              echo ""
              echo "Quick start:"
              echo "  maturin develop            # Build and install (editable)"
              echo "  python examples/algebra_dim.py  # Run example"
              echo "  uv build                   # Build package"
              echo ""
            '';
          };
      };
    });
}
