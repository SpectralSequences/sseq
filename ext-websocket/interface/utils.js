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
    return Interface.renderMath(rowToLaTeX(m));
}

export function matrixToKaTeX(m) {
    return Interface.renderMath("\\begin{bmatrix}" + m.map(x => x.join("&")).join("\\\\") + "\\end{bmatrix}");
}

export function rowToLaTeX(m) {
    return "\\begin{bmatrix}" + m.join("&") + "\\end{bmatrix}";
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
