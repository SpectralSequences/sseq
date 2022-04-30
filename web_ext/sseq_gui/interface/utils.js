import './components.js';

export const KATEX_ARGS = {
    throwOnError: false,
};

// Parses HTML string into a DOM object
export function html(s) {
    const wrapper = document.createElement('div');
    wrapper.innerHTML = s.trim();
    return wrapper.firstChild;
}

export function vecToName(v, names) {
    const items = [];
    for (let i = 0; i < v.length; i++) {
        switch (v[i]) {
            case 0:
                break;
            case 1:
                items.push(names[i]);
                break;
            default:
                items.push(`${v[i]} ${names[i]}`);
        }
    }
    return items.join(' + ');
}

export function rowToKaTeX(m) {
    return katex.renderToString(rowToLaTeX(m), KATEX_ARGS);
}

export function matrixToKaTeX(m) {
    return katex.renderToString(
        '\\begin{bmatrix}' +
            m.map(x => x.join('&')).join('\\\\') +
            '\\end{bmatrix}',
        KATEX_ARGS,
    );
}

export function rowToLaTeX(m) {
    return '\\begin{bmatrix}' + m.join('&') + '\\end{bmatrix}';
}

export function renderLaTeX(html) {
    const html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for (let i = 1; i < html_list.length; i += 2) {
        html_list[i] = katex.renderToString(html_list[i], KATEX_ARGS);
    }
    return html_list.join('\n');
}

export function download(filename, data, mime = 'text/plain') {
    if (!Array.isArray(data)) {
        data = [data];
    }
    const element = document.createElement('a');

    element.href = URL.createObjectURL(new Blob(data, { type: mime }));
    element.download = filename;
    element.rel = 'noopener';
    element.dispatchEvent(new MouseEvent('click'));
    setTimeout(() => URL.revokeObjectURL(element.href), 6e4);
}

export function dialog(title, contents, callback, submitText) {
    const dialog = html(`
    <dialog is="my-dialog">
        ${contents}
        <footer>
            <button class="button" value="submit">${
                submitText || 'Add'
            }</button>
        </footer>
    </dialog>`);
    dialog.setAttribute('header', title);

    document.body.appendChild(dialog);
    dialog.showModal();

    dialog.addEventListener('close', () => {
        document.body.removeChild(dialog);
        if (dialog.returnValue !== 'submit') {
            return;
        }
        callback(dialog);
    });
}
