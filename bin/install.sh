#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
virtualenv -p /usr/bin/python3.8 $BIN/virtualenv
source $BIN/virtualenv/bin/activate

pip install fastapi
pip install jinja2
pip install uvicorn
pip install pathlib
pip install ptpython
pip install websockets