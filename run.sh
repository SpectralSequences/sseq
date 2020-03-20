#!/bin/bash
export WORKING_DIRECTORY=$(pwd)
export SCRIPT_ARGS=$@
cd "$( dirname "${BASH_SOURCE[0]}" )"
source virtualenv/bin/activate
uvicorn main:app --log-level=warning
