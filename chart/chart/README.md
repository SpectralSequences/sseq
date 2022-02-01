This repository contains code for a javascript spectral sequences webclient (in the subdirectory client) and
Python code to allow a Python server to control the client.
The Python code depends on [message_passing_tree](https://github.com/SpectralSequences/message_passing_tree).
The client is a descendent of the prototype [js_spectralsequences](https://github.com/hoodmane/js_spectralsequences).
The server logic here only includes the logic specific to drawing charts.
The rest of the webserver logic is here [server](https://github.com/SpectralSequences/server).

# Server setup

TODO.

# Client Compilation

The directory `client/src` contains the source code for the spectral-sequences module.
This is compiled into `dist/sseq_webclient.js`, which is what you should include into your
webpage.

To install dependencies for compilation, run

```
 $ npm install
```

This only has to be done once. Afterwards, to compile the source code into
dist/sseq_webclient.js, run

```
 $ npm run build
```

# Running

Due to CORS restrictions, opening an file directly in a browser is unlikely to
succeed. Instead, you should run a web server that serves the directory and
then access localhost. To do so, navigate into the directory containing the
source code, and run one of the following two commans:

```
 $ python3 -m http.server 8080
 $ npx http-server
```

Afterwards, direct your browser to http://localhost:8080/
