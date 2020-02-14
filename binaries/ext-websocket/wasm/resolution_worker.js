self.promise = import("../pkg/index.js").catch(console.error).then(wasm => {
    self.resolution = wasm.Resolution.new(m => self.postMessage(m));
});

self.onmessage = ev => {
    if (!self.resolution) {
        self.promise.then(() => self.onmessage(ev));
        return;
    }
    let m = ev.data;
    self.resolution.run(m);   
}
