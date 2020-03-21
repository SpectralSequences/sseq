#!/bin/bash

cd "$( dirname $0 )"
cd ../dist

python3 -m http.server $1
