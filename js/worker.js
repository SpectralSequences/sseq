let wasm_promise = import ("../pkg/index.js").catch(console.error).then(wasm => {
    self.wasm = wasm;
});

self.max_int_deg = 0;
function addClass(hom_deg, int_deg, name) {
    if(int_deg > self.max_int_deg){
        self.max_int_deg = int_deg;
        if(self.max_int_deg % 10 === 0){
            console.log(`Time to compute stems ${self.max_int_deg - 10} to ${self.max_int_deg} : ${getTime()}`);
            console.log(`Total time to compute first ${self.max_int_deg} stems : ${getTotalTime()}`);
        }
    }
    self.postMessage({cmd: "addClass", "x": int_deg - hom_deg, "y": hom_deg});
};

function addStructline(mult, source_hom_deg, source_int_deg, source_idx, target_hom_deg, target_int_deg, target_idx){
    self.postMessage({
        cmd : "addStructline", 
        mult : mult,
        source : {x : source_int_deg - source_hom_deg, y : source_hom_deg, idx : source_idx},
        target : {x : target_int_deg - target_hom_deg, y : target_hom_deg, idx : target_idx}
    });
}

let t0 = performance.now();
let t_last = t0;

function getTime(){
    let t_cur = performance.now();
    let duration = (t_cur - t_last) / 1000;
    t_last = t_cur;
    return duration;
}

function getTotalTime(){
    let t_cur = performance.now();
    return (t_cur - t0) / 1000;
}

self.onmessage = (ev) => {
    if (!self.wasm) {
        wasm_promise.then(() => self.onmessage(ev));
        return;
    }
    let m = ev.data;
    if(!(m.cmd in message_handlers)){
        console.error(`Unknown command '${m.cmd}'`);
        return;
    }
    message_handlers[m.cmd](m);
    self.postMessage({ cmd: "complete", data: m });
}

let message_handlers = {};

message_handlers.resolve = function resolve(m){
    self.p = m.p;
    self.algebra = self.wasm.WasmAlgebra.new_adem_algebra(m.p, m.p != 2);
    resolution_constructors[m.type](self.algebra, m);
    self.res.resolve_through_degree(m.maxDegree);
    console.log(`Total time : ${getTotalTime()}`);
}

let resolution_constructors = {};
resolution_constructors["finite dimensional module"] = function constructFDModule(algebra, m){
    self.module = self.wasm.WasmFDModule.new_adem_module(algebra, m.module);
    self.cc = self.wasm.WasmCCDZFDModule.new_ccdz(self.module);
    self.res_no_maps = self.wasm.WasmResolutionCCDZFDModule.new(cc, m.maxDegree, addClass, addStructline);
    self.res = self.wasm.WasmResolutionWithChainMapsCCDZFDModule.new(self.res_no_maps, self.res_no_maps, m.module);
}

resolution_constructors["finitely presented module"] = function constructFPModule(algebra, m){
    self.module = self.wasm.WasmFPModule.new_adem_module(algebra, m.module);
    self.cc = self.wasm.WasmCCDZFPModule.new_ccdz(self.module);
    self.res = self.wasm.WasmResolutionCCDZFPModule.new(cc, m.maxDegree, addClass, addStructline);
}

message_handlers.getCocycle = function getCocycle(m){
    if(!self.res){
        console.log("No resolution yet, can't get cocycle");
        return;
    }
    let hom_deg = m.class.y;
    let int_deg = m.class.x + m.class.y;
    let cocycle_string = self.res.get_cocycle_string(hom_deg, int_deg, m.class.idx);
    if(self.p == 2){
        cocycle_string = cocycle_string.replace(/P/g, "Sq");
    }
    self.postMessage({
        "cmd" : "cocycleResult",
        "class" : m.class,
        "cocycle" : cocycle_string
    });
}
