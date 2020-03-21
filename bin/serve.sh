#!/bin/bash
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export REPOSITORY_ROOT="$(dirname "$BIN")"
export DIST=$REPOSITORY_ROOT/dist
cd $DIST
python3 -m http.server $1
