import ("../pkg/index.js").catch(console.error).then(wasm => {
    let C2json = '{"name": "$C(2)$", "file_name": "C2", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "sq_actions": [{"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}';
    let max_degree = 80;
    let add_class = (a, b, name) => {
        self.postMessage({cmd: "addClass", "x": b - a, "y": a});
    };
    wasm.resolve(C2json, max_degree, add_class);
});
