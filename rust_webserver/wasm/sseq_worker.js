'use strict';
importScripts("./ext_webserver_wasm.js");

const { Sseq } = wasm_bindgen;
const promise = wasm_bindgen("./ext_webserver_wasm_bg.wasm").catch(console.error).then(() => {
    self.sseq = Sseq.new((m) => self.postMessage(m));
});

self.onmessage = ev => {
    if (!self.sseq) {
        promise.then(() => self.onmessage(ev));
        return;
    }
    self.sseq.run(ev.data);
}
