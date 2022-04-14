'use strict';
importScripts('./steenrod_calculator_wasm.js');

const { SteenrodCalculator } = wasm_bindgen;

const promise = wasm_bindgen('./steenrod_calculator_wasm_bg.wasm').catch(
    console.error,
);

self.calculators = {};

self.onmessage = async ev => {
    await promise;

    const m = ev.data;
    if (ev.data.input.trim() === '') {
        return;
    }

    if (!(m.prime in self.calculators)) {
        self.calculators[m.prime] = SteenrodCalculator.new(m.prime);
    }

    try {
        let result;
        if (m.basis === 'adem') {
            result = self.calculators[m.prime].evaluate_adem(m.input);
        } else {
            result = self.calculators[m.prime].evaluate_milnor(m.input);
        }
        self.postMessage({
            cmd: 'result',
            result: `${to_latex(m.prime, m.input)} = ${to_latex(
                m.prime,
                result,
            )}`,
        });
    } catch (e) {
        self.postMessage({ cmd: 'error', error: e });
    }
};

function to_latex(p, str) {
    let P_or_Sq = p == 2 ? 'Sq' : 'P';
    return str
        .replace(/(?:P|Sq)(\d*)/g, 'P^{$1}')
        .replace(/P/g, `${P_or_Sq}`)
        .replace(/b/g, `\\beta`)
        .replace(/\*/g, '')
        .replace(/Q(\d*)/g, 'Q_{$1}');
}
