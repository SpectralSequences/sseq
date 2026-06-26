'use strict';
importScripts("./sseq_gui_wasm.js");

const { Sseq } = wasm_bindgen;
const promise = wasm_bindgen("./sseq_gui_wasm_bg.wasm").catch(console.error).then(() => {
    self.sseq = Sseq.new((m) => self.postMessage(m));
});

function reportPanic(err) {
    const message =
        err && err.stack ? err.stack : `${err}`;
    self.postMessage(
        JSON.stringify({
            recipients: [],
            sseq: 'Main',
            action: { Error: { message: `Panic in sseq worker:\n${message}` } },
        }),
    );
}

self.onmessage = ev => {
    if (!self.sseq) {
        promise.then(() => self.onmessage(ev));
        return;
    }
    try {
        self.sseq.run(ev.data);
    } catch (err) {
        reportPanic(err);
    }
}
