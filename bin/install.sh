#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
virtualenv -p /usr/bin/python3.8 $BIN/virtualenv
source $BIN/virtualenv/bin/activate

rustup toolchain install nightly
pip install fastapi
pip install jinja2
pip install maturin
pip install pathlib
pip install ptpython
pip install uvicorn
pip install websockets
./$BIN/build_rust_ext.sh