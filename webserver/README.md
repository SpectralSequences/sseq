spectral sequences server
=========================

This project is a web front end for our resolver tied to a python console.
This python console uses the pyo3 bindings to our resolver which is implemented in rust.
The resolver backend is in [this repository](https://github.com/SpectralSequences/ext).
The javascript client code is in [this respository](https://github.com/SpectralSequences/basic_webclient).

Installation
------------
You will first need to install Python 3.8 and Rust.
Run `bin/setup.sh` to install.

Run
---
To start the process, run `webserver/bin/run.sh`. 

Example
-------
Run `./bin/run.sh examples/C2.py` to set up `C(2)` to resolve. Navigate a browser to `localhost:8000/sseq/C2` and you will see a blank chart. In the console type `res_c2.resolve(80)` and the chart will be populated.

Try running `./bin/run.sh examples/sphere.py examples/C2.py`, then visiting `localhost:8000/sseq/S0` and `localhost:8000/sseq/C2` and running `res_sphere.resolve(80);res_c2.resolve(80)`. The two charts will be populated in parallel.