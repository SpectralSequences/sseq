spectral sequences server
=========================

This project is a web front end for our resolver tied to a python console.
This python console uses the pyo3 bindings to our resolver which is implemented in rust.
The resolver backend is in [this repository](https://github.com/SpectralSequences/ext).
The javascript client code is in [this respository](https://github.com/SpectralSequences/basic_webclient).

This requires Python 3.8

Installation
------------
1. Python
* 1a. Install python 3.8. Python 3.8 is so new that it is not present on many package managers. See installation instructions for [linux](https://tecadmin.net/install-python-3-8-ubuntu/) and [mac](https://installvirtual.com/install-python-3-8-on-mac/). If you are running windows 10, you should [install windows system linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10) and then follow the linux instructions.
* 1b. cd into the project root directory and run `./install.sh`. This makes a virtual environment.

2. Rust
 * 2a. [Install Rust](https://www.rust-lang.org/tools/install).
 * 2b. Run the command `rustup toolchain install nightly`
 * 2c. Clone https://github.com/SpectralSequences/ext.
 * 2d. cd into `<SpectralSequences/ext>/python/pyo3` (where ext is the directory you cloned the SpectralSequences/ext repository into).
 * 2e. Run `maturin develop`. If you get an error about virtual environments, run `source <SpectralSequences/webserver>/virtualenv/bin/activate` and then try again.

3. Javascript
 * 3a. You only need the single file `<SpectralSequences/basic_webclient>/target/release/sseq_basic_webclient.js`.
    Either clone `SpectralSequences/basic_webclient` or just copy that file somewhere.
 * 3b. The file `<SpectralSequences/webserver>/user/config.py` contains a line `SSEQ_BASIC_WEBCLIENT_JS_FILE = "../basic_webclient/  target/debug/sseq_basic_webclient.js"`. Update that line in the config file with the path to the file `sseq_basic_webclient.js`, wherever you put it.


Run
---
To start the process, run `./run.sh`. It takes space delimited file names as arguments, each of which gets run on initialization.
For instance `./run.sh examples/C2.py` sets up C(2) to resolve. Run `./run.sh examples/C2.py`, then navigate a browser to `localhost:8000/sseq/C2` and you will see a blank chart. In the console type `res_c2.resolve(80)` and the chart will be populated.
