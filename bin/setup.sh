#!/bin/sh

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

msg () {
    echo "${GREEN}$1${NC}"
}

error () {
    echo "${RED}$1${NC}"
}

WORKING_DIRECTORY="$(pwd)"

cd "$( dirname $0 )"
cd ..


which python3.8
if [ $? -ne 0 ]; then 
    error "I can't find python3.8. Install it / make sure it's on your path and then try again."
    exit 1
fi

which cargo
if [ $? -ne 0 ]; then 
    error "I can't find cargo. Install Rust / make sure it's on your path and then try again."
    exit 1
fi

REPO_NAME="python_ext"
if ! [ -d "$REPO_NAME" ]; then
    msg "Downloading python_ext from https://github.com/SpectralSequences/$REPO_NAME into $(pwd)/$REPO_NAME." \
         "If you have a local copy of $REPO_NAME, you may wish to replace this with a symlink"
    git clone --depth 1 https://github.com/SpectralSequences/python_ext
    git clone --depth 1 https://github.com/SpectralSequences/ext python_ext/rust_ext
fi

REPO_NAME="basic_webclient"
if ! [ -d "basic_webclient" ]; then
    msg "Downloading basic_webclient from https://github.com/SpectralSequences/basic_webclient into $(pwd)/basic_webclient." \
         "If you have a local copy of basic_webclient, you may wish to replace this with a symlink"
    git clone --depth 1 https://github.com/SpectralSequences/basic_webclient
fi

msg "Making Python virtual env"
virtualenv -p python3.8 bin/virtualenv
if [ $? -ne 0 ]; then 
    error "Failed to make virtual environment."
    exit 1
fi
. bin/virtualenv/bin/activate
msg "Installing python packages."
pip install fastapi
pip install jinja2
pip install maturin
pip install pathlib
pip install ptpython
pip install uvicorn
pip install websockets

msg "Building rust_ext."

cd python_ext/pyo3
maturin develop 
cd ../..

cd $WORKING_DIRECTORY