'use strict';

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
        let vec = parseIntegerArray(response);
        if (vec === null || vec.length != length) {
            alert(error);
        } else {
            return vec;
        }
    }
}

export function parseIntegerArray(text) {
    try {
        let vec = JSON.parse(text.trim());
        if (Array.isArray(vec) &&
            vec.reduce((b, x) => b && Number.isInteger(x), true)) {
            return vec;
        }
    } catch(e) { }
    return null;
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
}

export function inflate(x) {
    return new TextDecoder("utf-8").decode(pako.inflate(x));
}

export function deflate(x) {
    return pako.deflate(x, { level : 1});
}

const A_ = 0x41;
const a_ = 0x61;
const zero_ = 0x30;

const BASE64_TABLE = (() => {
    let table = [];
    for (let i = 0; i < 26; i++) {
        table.push(String.fromCharCode(A_ + i));
    }
    for (let i = 0; i < 26; i++) {
        table.push(String.fromCharCode(a_ + i));
    }
    for (let i = 0; i < 10; i++) {
        table.push(String.fromCharCode(zero_ + i));
    }
    table.push("-");
    table.push("_");
    return table;
})();

function base64ToInt(y) {
    let x = y.charCodeAt(0);
    if (x == 0x2d)
        return 62;
    else if (x == 0x5F)
        return 63;

    if (x < 0x3A)
        return x - zero_ + 52;
    else if (x < 0x5B)
        return x - A_;
    else
        return x - a_ + 26;
}

export function encodeB64(bytes) {
    let result = '';
    let i;
    for (i = 2; i < bytes.length; i += 3) {
        result += BASE64_TABLE[bytes[i - 2] >> 2];
        result += BASE64_TABLE[((bytes[i - 2] & 0x03) << 4) | (bytes[i - 1] >> 4)];
        result += BASE64_TABLE[((bytes[i - 1] & 0x0F) << 2) | (bytes[i] >> 6)];
        result += BASE64_TABLE[bytes[i] & 0x3F];
    }
    if (i === bytes.length + 1) { // 1 octet missing
        result += BASE64_TABLE[bytes[i - 2] >> 2];
        result += BASE64_TABLE[(bytes[i - 2] & 0x03) << 4];
    }
    if (i === bytes.length) { // 2 octets missing
        result += BASE64_TABLE[bytes[i - 2] >> 2];
        result += BASE64_TABLE[((bytes[i - 2] & 0x03) << 4) | (bytes[i - 1] >> 4)];
        result += BASE64_TABLE[(bytes[i - 1] & 0x0F) << 2];
    }
    return result;
}

export function decodeB64(x) {
    const l = x.length;
    const new_len = (l - (l % 4)) / 4 * 3 + (() => {
        switch (l % 4) {
            case 2:
                return 1;
            case 3:
                return 2;
            default:
                return 0;
        }
    })();

    let result = new Uint8Array(new_len);
    let counter = 0;
    let push = ((x) => {
        result[counter] = x;
        counter += 1
    });

    let i;

    for (i = 3; i < l; i+=4) { // Handle last entry differently because of padding
        const bytes = [x[i - 3], x[i - 2], x[i - 1], x[i]].map(base64ToInt);
        push((bytes[0] << 2) + (bytes[1] >> 4))
        push((bytes[1] << 4) + (bytes[2] >> 2))
        push((bytes[2] << 6) + bytes[3])
    }

    if (i == l + 1) { // 2 excess bytes
        const bytes = [x[i - 3], x[i - 2]].map(base64ToInt);
        push((bytes[0] << 2) + (bytes[1] >> 4))
    } else if (i == l) { // 3 excess bytes
        const bytes = [x[i - 3], x[i - 2], x[i - 1]].map(base64ToInt);
        push((bytes[0] << 2) + (bytes[1] >> 4))
        push((bytes[1] << 4) + (bytes[2] >> 2))
    }

    return result;
}

export function stringToB64(x) {
    return encodeB64(pako.deflate(x));
}

export function b64ToString(x) {
    return inflate(decodeB64(x));
}
