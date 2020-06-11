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

# REPO_NAME="python_ext"
# if ! [ -d "$REPO_NAME" ]; then
#     msg "Downloading $REPO_NAME from https://github.com/SpectralSequences/$REPO_NAME into $(pwd)/$REPO_NAME." \
#          "If you have a local copy of $REPO_NAME, you may wish to replace this with a symlink."
#     git clone --depth 1 https://github.com/SpectralSequences/python_ext
#     git clone --depth 1 https://github.com/SpectralSequences/ext python_ext/rust_ext
# fi

# REPO_NAME="message_passing"
# if ! [ -d "$REPO_NAME" ]; then
#     msg "Downloading $REPO_NAME from https://github.com/SpectralSequences/$REPO_NAME into $(pwd)/$REPO_NAME." \
#          "If you have a local copy of $REPO_NAME, you may wish to replace this with a symlink."
#     git clone --depth 1 https://github.com/SpectralSequences/basic_webclient
# fi

msg "Making Python virtual env"
virtualenv -p python3.8 bin/virtualenv
if [ $? -ne 0 ]; then 
    error "Failed to make virtual environment."
    exit 1
fi
. bin/virtualenv/bin/activate
msg "Installing python packages."
pip install aiofiles
pip install fastapi
pip install lark
pip install jinja2
pip install maturin
pip install pathlib
pip install ptpython
pip install readerwriterlock # Necessary?
pip install uvicorn
pip install websockets
#pip install imageio
#pip install shapely
#pip install scikit-image
msg "Building rust_ext."

cd ../python_ext/pyo3
maturin develop 
cd ../../ext

cd $WORKING_DIRECTORY
