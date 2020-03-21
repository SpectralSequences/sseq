#!/bin/bash
export WORKING_DIRECTORY="$(pwd)"
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
source $BIN/virtualenv/bin/activate
python $BIN/_run.py $@