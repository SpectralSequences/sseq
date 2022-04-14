const worker = new Worker('./steenrod_calculator_worker.js');
worker.addEventListener('message', ev => {
    const elt = document.getElementById('adem-result');
    switch (ev.data.cmd) {
        case 'result':
            katex.render(ev.data.result, elt, { displayMode: true });
            break;
        case 'error':
            elt.innerHTML = `<span style='color: red'>${ev.data.error}</span>`;
            break;
    }
});

function katexMathInDelims(string) {
    const html_list = string.split(
        /(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/,
    );
    for (let i = 1; i < html_list.length; i += 2) {
        html_list[i] = katex.renderToString(html_list[i]);
    }
    return html_list.join('');
}

const description_element = document.getElementById('description-div');
description_element.innerHTML = katexMathInDelims(
    description_element.innerHTML,
);

window.compute = () => {
    document.getElementById('adem-result').innerHTML = '';

    worker.postMessage({
        basis: document.querySelector('input[name="basis"]:checked').value,
        prime: Number.parseInt(
            document.querySelector('input[name="prime"]:checked').value,
        ),
        input: document.getElementById('calculator-input').value,
    });
};
