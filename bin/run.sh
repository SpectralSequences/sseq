#!/bin/bash
echo "Initializing SpectralSequences/webserver..."
# This is really just here to activate the virtualenv before running python.
export WORKING_DIRECTORY="$(pwd)"
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
source $BIN/virtualenv/bin/activate
CMDLINE_ARGS="$@"
python $BIN/_run.py $@