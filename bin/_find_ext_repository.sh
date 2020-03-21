#!/bin/bash

function try_ext_repo() {
    if [ -d "$1" ]; then
        echo "$1"
        exit 0
    fi
}

try_ext_repo "$1" 
try_ext_repo "$EXT_REPOSITORY_PATH"
export BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
try_ext_repo "$BIN/../../ext"
exit 1