"use strict";
importScripts("./steenrod_calculator_wasm.js");

const { SteenrodCalculator } = wasm_bindgen;

const promise = wasm_bindgen("./steenrod_calculator_wasm_bg.wasm").catch(console.error)

self.calculators = {};

self.onmessage = async (ev) => {
    await promise;

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
    return str.replace(/((?:P|Sq)(?:\d*)) ((?:P|Sq)(?:\d*))/g, "$1 * $2")
              .replace(/((?:P|Sq)(?:\d*)) ((?:P|Sq)(?:\d*))/g, "$1 * $2")
              .replace(/P/g, `${P_or_Sq}`);
}

message_handlers.calculate = function calculate(m) {
    if(!(m.prime in self.calculators)){
        self.calculators[m.prime] = SteenrodCalculator.new(m.prime);
        self.calculators[m.prime].compute_basis(20);
    }
    let result;
    try {
        if(m.basis === "adem") {
            result = self.calculators[m.prime].evaluate_adem(m.input);
        } else if(m.basis === "milnor") {
            result = self.calculators[m.prime].evaluate_milnor(m.input);
        } else {
            console.log(`Unknown basis ${m.basis}`);
            return;
        }
    } catch(e) {
        self.postMessage({"cmd" : "error", "error" : e});
    }
    self.postMessage({
        "cmd" : "result",
        "latex_input" : to_latex(m.prime, m.input),
        "latex_result" : to_latex(m.prime, result),
        "simple_result" : to_input_form(m.prime, result)
    });
}
