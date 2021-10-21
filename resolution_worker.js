'use strict';
importScripts("./sseq_gui_wasm.js");

const { Resolution } = wasm_bindgen;
const promise = wasm_bindgen("./sseq_gui_wasm_bg.wasm").catch(console.error).then(() => {
    self.resolution = Resolution.new(m => self.postMessage(m));
});

self.onmessage = ev => {
    if (!self.resolution) {
        promise.then(() => self.onmessage(ev));
        return;
    }
    self.resolution.run(ev.data);
}
