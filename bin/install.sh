#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
export WORKING_DIRECTORY="$(pwd)"
export EXT_REPOSITORY=$(python $BIN/_get_config_vars.py EXT_REPOSITORY)

virtualenv -p /usr/bin/python3.8 $BIN/virtualenv
source $BIN/virtualenv/bin/activate

pip install fastapi
pip install jinja2
pip install maturin
pip install pathlib
pip install ptpython
pip install uvicorn
pip install websockets

cd $EXT_REPOSITORY
rustup override set nightly-2020-02-29
$BIN/build_rust_ext.sh