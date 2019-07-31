// import { __esModule } from "copy-webpack-plugin/dist";
import ("../pkg/index.js").catch(console.error).then(wasm => {
    self.wasm = wasm;
    self.postMessage({cmd: "initialized"});
});

function addClass (a, b, name) {
    self.postMessage({cmd: "addClass", "x": b - a, "y": a});
};

function addStructline(type, source_hom_deg, source_int_deg, source_idx, target_hom_deg, target_int_deg, target_idx){
    self.postMessage({
        cmd : "addStructline", 
        type : type,
        source : {x : source_int_deg - source_hom_deg, y : source_hom_deg, idx : source_idx},
        target : {x : target_int_deg - target_hom_deg, y : target_hom_deg, idx : target_idx}
    });
}

self.onmessage = (ev) => {
    if (!self.wasm) {
        console.log("Not yet initialized. Message discarded");
        console.log(ev);
        return;
    }

    let m = ev.data;
    switch (m.cmd) {
        case "resolve":
            let p = 2;
            let algebra = self.wasm.WasmAlgebra.new_adem_algebra(p, p != 2, m.maxDegree);
            console.log(algebra);
            algebra.compute_basis(m.maxDegree);
            let fdmodule = self.wasm.WasmModule.new_adem_module(algebra, m.module);
            let cc = self.wasm.WasmChainComplex.new_ccdz(fdmodule);
            self.res = self.wasm.WasmResolution.new(cc, m.maxDegree, addClass, addStructline);
            self.res.resolve_through_degree(m.maxDegree);
            break;
        default:
            break;
    }
}
