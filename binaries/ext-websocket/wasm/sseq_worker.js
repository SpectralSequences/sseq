self.promise = import("../pkg/index.js").catch(console.error).then(wasm => {
    self.sseq = wasm.Sseq.new((m) => self.postMessage(m));
});

self.onmessage = ev => {
    if (!self.sseq) {
        self.promise.then(() => self.onmessage(ev));
        return;
    }
    let m = ev.data;
    self.sseq.run(m);   
}
