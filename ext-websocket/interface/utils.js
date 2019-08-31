export function vecToName(v, names) {
    let items = [];
    for (let i = 0; i < v.length; i++) {
        switch (v[i]) {
            case 0 : break;
            case 1 : items.push(names[i]); break;
            default : items.push(`${v[i]} ${names[i]}`);
        }
    }
    return items.join(" + ");
}

export function rowToKaTeX(m) {
    return katex.renderToString(rowToLaTeX(m));
}

export function matrixToKaTeX(m) {
    return katex.renderToString("\\begin{bmatrix}" + m.map(x => x.join("&")).join("\\\\") + "\\end{bmatrix}");
}

export function rowToLaTeX(m) {
    return "\\begin{bmatrix}" + m.join("&") + "\\end{bmatrix}";
}

export function renderLaTeX(html) {
    let html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = katex.renderToString(html_list[i], { throwOnError : false });
    }
    return html_list.join("\n");
}

export function renderLaTeXP(html) {
    html = html.replace(/\n/g, "\n</p><p>\n")
    let html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = katex.renderToString(html_list[i], { throwOnError : false });
    }
    return `<p>${html_list.join("\n")}</p>`;
}

// Prompts for an array of length `length`
export function promptClass(text, error, length) {
    while (true) {
        let response = prompt(text);
        if (!response) {
            return null;
        }
        try {
            let vec = JSON.parse(response.trim());
            if (Array.isArray(vec) &&
                vec.length == length &&
                vec.reduce((b, x) => b && Number.isInteger(x), true)) {
                return vec;
            }
        } catch(e) { // If we can't parse, try again
        }
        alert(error);
    }
}

export function promptInteger(text, error) {
    while (true) {
        let response = prompt(text);
        if (!response) {
            return null;
        }
        let c = parseInt(response.trim());
        if (!isNaN(c)) {
            return c;
            break;
        }
        alert(error);
    }
}

export function download (filename, data, mime="text/plain") {
    if (!Array.isArray(data)) {
        data = [data];
    }
    let element = document.createElement('a');

    element.href = URL.createObjectURL(new Blob(data, {type : mime}));
    element.download = filename;
    element.rel = 'noopener';
    element.dispatchEvent(new MouseEvent('click'));
    setTimeout(() => URL.revokeObjectURL(element.href), 6E4);
};

export function inflate(x) {
    return new TextDecoder("utf-8").decode(pako.inflate(x));
}
