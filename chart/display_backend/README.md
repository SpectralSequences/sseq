# WebGl Chart
This is an implementation of a simple 2D graphics backend using webgl2 and Rust / wasm_bindgen.
The logic is inspired by tikz: there are "nodes" which have known boundary and we draw edges between the nodes.
The edges can be straight or circular arcs and have adjustable thickness, dash pattern, color and arrowheads.


# Build instructions:
To build: 
```sh
    wasm-pack build
```

To build the test app (just displays a test page):
```sh
    npm i
    npm run build
```

To view the demo, you will need a webserver with the correct mimetype for wasm. For convenience a tiny wrapper around Python http-server is in serve_wasm.py. Inside the file, it explicity specifies that it will serve to port 8101, which I chose to avoid clashing with other ports I'm serving stuff to. If you want to serve to a different port, I would suggest copying `serve_wasm.py` into `dist` (in `.gitignore`) and changing the port number.

As a second option, you could use `webpack-dev-server` which automatically serves wasm with the correct mimetype. This you can start with `npm run serve`.