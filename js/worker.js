// import { __esModule } from "copy-webpack-plugin/dist";
import ("../pkg/index.js").catch(console.error).then(wasm => {
    self.wasm = wasm;
    self.postMessage({cmd: "initialized"});
});

function addClass (a, b, name) {
    self.postMessage({cmd: "addClass", "x": b - a, "y": a});
};

self.onmessage = (ev) => {
    if (!self.wasm) {
        console.log("Not yet initialized. Message discarded");
        console.log(ev);
        return;
    }

    let m = ev.data;
    switch (m.cmd) {
        case "resolve":
            self.wasm.resolve(m.module, m.maxDegree, addClass);
            break;
        default:
            break;
    }
}
