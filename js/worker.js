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
            self.wasm.resolve(m.module, m.maxDegree, addClass, addStructline);
            break;
        default:
            break;
    }
}
