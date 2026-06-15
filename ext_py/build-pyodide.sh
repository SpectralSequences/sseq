#!/usr/bin/env bash
#
# Build the `ext` Python extension as a WebAssembly wheel for Pyodide.
#
# This cross-compiles the Rust/pyo3 extension to `wasm32-unknown-emscripten`
# against the CPython ABI shipped by a specific Pyodide release. The emscripten
# version MUST match the one the target Pyodide was built with, so both are
# pinned below.
#
# The crate already builds threadless: it depends on `ext`/`algebra`/`fp`/`sseq`
# without the `concurrent` (rayon) feature, so no native-only cargo features
# need to be disabled for the wasm build.
#
# Usage:
#   ./build-pyodide.sh
#
# Output:
#   dist/*-wasm32.whl   (install in Pyodide via micropip)
#
set -euo pipefail

# --- Pinned versions (keep EMSCRIPTEN_VERSION in sync with PYODIDE_VERSION) ---
# Pyodide 314.0.0  ->  CPython 3.14.2, emscripten 5.0.3
PYODIDE_VERSION="${PYODIDE_VERSION:-314.0.0}"
EMSCRIPTEN_VERSION="${EMSCRIPTEN_VERSION:-5.0.3}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EMSDK_DIR="$SCRIPT_DIR/.emsdk"

cd "$SCRIPT_DIR"

# --- 1. Emscripten SDK -------------------------------------------------------
if [[ ! -d "$EMSDK_DIR" ]]; then
    echo ">> Cloning emsdk into $EMSDK_DIR"
    git clone --depth 1 https://github.com/emscripten-core/emsdk.git "$EMSDK_DIR"
fi

echo ">> Installing/activating emscripten $EMSCRIPTEN_VERSION"
"$EMSDK_DIR/emsdk" install "$EMSCRIPTEN_VERSION"
"$EMSDK_DIR/emsdk" activate "$EMSCRIPTEN_VERSION"
# shellcheck disable=SC1091
source "$EMSDK_DIR/emsdk_env.sh"

# --- 2. pyodide-build + cross-build environment ------------------------------
if ! command -v pyodide >/dev/null 2>&1; then
    echo "!! 'pyodide' not found on PATH."
    echo "!! Activate the venv (e.g. 'source .venv/bin/activate') and install it:"
    echo "!!     uv pip install pyodide-build"
    exit 1
fi

echo ">> Ensuring Pyodide cross-build environment $PYODIDE_VERSION is installed"
pyodide xbuildenv install "$PYODIDE_VERSION"

# --- 3. Build the wheel ------------------------------------------------------
echo ">> Building Pyodide wheel"
pyodide build

echo
echo ">> Done. Wheel(s):"
ls -1 dist/*wasm32.whl
