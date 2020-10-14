# WebGl Chart
This is a test implementation of a simple 2D graphics backend using webgl2 and Rust/wasm_bindgen.
The logic is inspired by tikz: there are "nodes" which have known boundary and we draw edges between the nodes.
The edges can be straight or circular arcs and have adjustable thickness, dash pattern, color and arrowheads.