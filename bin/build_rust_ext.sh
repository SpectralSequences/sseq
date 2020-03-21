#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
export WORKING_DIRECTORY="$(pwd)"
export RUSTFLAGS="-Z macro-backtrace"

source $BIN/virtualenv/bin/activate
cd $SSEQ_PYTHON_EXT_PATH/pyo3
rustup show
maturin develop