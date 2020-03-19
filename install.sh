#!/bin/bash
virtualenv -p /usr/bin/python3.8 virtualenv
source virtualenv/bin/activate

pip install fastapi
pip install jinja2
pip install uvicorn
pip install pathlib
pip install ptpython
pip install websockets
