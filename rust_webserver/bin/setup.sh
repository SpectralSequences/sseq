#!/bin/sh

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..

if ! [ -d "ext" ]; then
    echo "Downloading ext from https://github.com/spectralsequences/ext into $(pwd)/ext. If you have a local copy of ext, you may wish to replace this with a symlink"
    git clone --depth 1 https://github.com/spectralsequences/ext
fi

cd $WORKING_DIRECTORY
