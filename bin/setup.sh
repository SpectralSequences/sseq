#!/bin/sh

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..

REPO_NAME="python_ext"
if ! [ -d "$REPO_NAME" ]; then
    echo "Downloading python_ext from https://github.com/SpectralSequences/$REPO_NAME into $(pwd)/$REPO_NAME." \
         "If you have a local copy of $REPO_NAME, you may wish to replace this with a symlink"
    git clone --depth 1 https://github.com/SpectralSequences/python_ext
    cd python_ext
    git clone --depth 1 https://github.com/SpectralSequences/ext
fi

REPO_NAME="basic_webclient"
if ! [ -d "basic_webclient" ]; then
    echo "Downloading basic_webclient from https://github.com/SpectralSequences/basic_webclient into $(pwd)/basic_webclient." \
         "If you have a local copy of basic_webclient, you may wish to replace this with a symlink"
    git clone --depth 1 https://github.com/SpectralSequences/basic_webclient
fi

echo "Making Python virtual env"
virtualenv -p /usr/bin/python3.8 bin/virtualenv
source bin/virtualenv/bin/activate

echo "Installing python packages."
pip install fastapi
pip install jinja2
pip install maturin
pip install pathlib
pip install ptpython
pip install uvicorn
pip install websockets

cd $WORKING_DIRECTORY