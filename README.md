spectral sequences server
=========================

This project is a web front end for our resolver tied to a python console.
This python console uses the pyo3 bindings to our resolver which is implemented in rust.
The resolver backend is in [this repository](https://github.com/SpectralSequences/ext).
The javascript client code is in [this respository](https://github.com/SpectralSequences/basic_webclient).

Installation
------------
If you are running windows 10, you should first [install windows system linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10). If you are running linux, this hopefully should work fine. Probably on mac you will need to figure out how to fix the build scripts. If you are trying to build this on a mac let me know.

1. Make a folder called `sseq` anywhere.
2. Clone `SpectralSequences/ext`, `SpectralSequences/basic_webclient`, and `SpectralSequences/webserver` into `sseq`:
```bash
mkdir sseq
cd sseq
git clone git@github.com:SpectralSequences/ext.git
git clone git@github.com:SpectralSequences/basic_webclient.git
git clone git@github.com:SpectralSequences/webserver.git
````
3. [Install Rust](https://www.rust-lang.org/tools/install) if you dont already have it. For convenience, you can install rust with the script `./bin/install_rust.sh` if you are on linux (or windows system linux).
4. Install python 3.8. Python 3.8 is so new that it is not present on many package managers. If you are using linux or Windows system linux, you can use the install script in `./bin/install_python3.8.sh`. Here are [mac installation instructions](https://installvirtual.com/install-python-3-8-on-mac/).
5. Run `./bin/install.sh`

Run
---
To start the process, run `./bin/run.sh`. 

Example
-------
Run `./bin/run.sh examples/C2.py` to set up `C(2)` to resolve. Navigate a browser to `localhost:8000/sseq/C2` and you will see a blank chart. In the console type `res_c2.resolve(80)` and the chart will be populated.

Try running `./bin/run.sh examples/sphere.py examples/C2.py`, then visiting `localhost:8000/sseq/S0` and `localhost:8000/sseq/C2` and 