#!/bin/bash
if [ ! -d "virtualenv" ]; then
    virtualenv --python=/usr/bin/python3.6 virtualenv/
fi
source virtualenv/bin/activate
cd pyo3
maturin develop
cd ..