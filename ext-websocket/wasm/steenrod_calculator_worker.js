"use strict";

let wasm_promise = import ("../pkg/index.js").catch(console.error).then(wasm => {
    self.wasm = wasm;
});
self.calculators = {};

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
}

let message_handlers = {};

function to_latex(p, str) {
    let P_or_Sq = p == 2 ? "Sq" : "P";
    return str.replace(/(?:P|Sq)(\d*)/g, "P^\{$1\}")
              .replace(/P/g, `${P_or_Sq}`)
              .replace(/\*/g, "")
              .replace(/Q(\d*)/g,"Q_\{$1\}");
}

function to_input_form(p, str) {
    let P_or_Sq = p == 2 ? "Sq" : "P";
    return str.replace(/((?:P|Sq)(?:\d*)) ((?:P|Sq)(?:\d*))/, "$1 * $2")
              .replace(/P/g, `${P_or_Sq}`);
}

message_handlers.calculate = function calculate(m){
    if(!(m.prime in self.calculators)){
        self.calculators[m.prime] = self.wasm.SteenrodCalculator.new(m.prime);
        self.calculators[m.prime].compute_basis(20);
    }
    console.log("hi!");
    let result;
    if(m.basis === "adem") {
        result = self.calculators[m.prime].evaluate_adem(m.input);
    } else if(m.basis === "milnor") {
        result = self.calculators[m.prime].evaluate_milnor(m.input);
    } else {
        // Unknown basis...
    }
    let latex_input = to_latex(m.prime, m.input);
    let latex_result = to_latex(m.prime, result);
    let simple_result = to_input_form(m.prime, result);
    self.postMessage({"cmd" : "result", "latex_input" : latex_input, "latex_result" : latex_result, "simple_result" : simple_result});
    console.log(`Total time : ${getTotalTime()}`);
}

