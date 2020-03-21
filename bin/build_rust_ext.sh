#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
export WORKING_DIRECTORY="$(pwd)"
export RUSTFLAGS="-Z macro-backtrace"

source $BIN/virtualenv/bin/activate
export EXT_REPOSITORY=$(python $BIN/_get_config_vars.py EXT_REPOSITORY)
cd $EXT_REPOSITORY/python/pyo3
maturin develop