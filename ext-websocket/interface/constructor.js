'use strict';

import { download } from "./utils.js";

window.maxDegree = 10;
window.form = document.getElementById("header-form");

const LEFT_MARGIN = 20;
const NODE_OFFSET = 20;
 
// prime
window.prime = parseInt(form.prime.value);
form.prime.addEventListener("change", () => {
    window.prime = parseInt(form.prime.value);
});

/******************************
 * Finite Dimensional Modules *
 * ****************************/
window.fdform = document.getElementById("fdmodule-form");
// mode
fdform.mode.value = "add";
let updateMode = () => {
    window.mode = fdform.mode.value;
    window.fdform.mode.forEach(y => {
        if (y.value == window.mode) {
            y.parentElement.classList.add("active");
        } else {
            y.parentElement.classList.remove("active");
        }
    });
}

updateMode();
fdform.mode.forEach(x => {
    x.addEventListener('change', updateMode);
})


window.minDegree = parseInt(fdform["min-degree"].value);
window.maxDegree = parseInt(fdform["max-degree"].value);

fdform["min-degree"].addEventListener("change", () => {
    window.minDegree = parseInt(fdform["min-degree"].value);
    drawCanvas();
});
fdform["max-degree"].addEventListener("change", () => {
    window.maxDegree = parseInt(fdform["max-degree"].value);
    drawCanvas();
});

// canvas size
window.canvas = document.getElementById("fdmodule-canvas");
window.context = canvas.getContext("2d");
window.addEventListener("resize", drawCanvas);

window.dimensions = new Map();
window.actions = new Map(); // degree -> index -> actions;

function drawCanvas() {
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const mid = Math.round(0.5 + width / 2) - 0.5;

    canvas.width = width;
    canvas.height = height;

    const numDegree = maxDegree - minDegree;

    context.strokeStyle = "#CCC";
    for (let i = minDegree; i <= maxDegree; i++) {
        const y = getY(i - minDegree, numDegree, height);
        context.fillText(i, LEFT_MARGIN, y);
        context.fillText(i, width - LEFT_MARGIN, y);

        context.beginPath();
        context.moveTo(LEFT_MARGIN * 2, y);
        context.lineTo(width - LEFT_MARGIN * 2, y);
        context.stroke();
    }
    context.strokeStyle = "black";

    for (let i = minDegree; i <= maxDegree; i++) {
        const y = getY(i - minDegree, numDegree, height);
        const dim = getDimension(i);
        const actions = getAction(i);
        for (let j = 0; j < dim; j++) {
            const x = mid + NODE_OFFSET * (j - (dim - 1) / 2);
            // Draw nodes
            context.beginPath();
            context.arc(x, y, 3, 0, 2 * Math.PI);
            context.stroke();
            context.fill();

            // Draw actions
            for (let op = 0; op < actions[j].length; op++) {
                if (!actions[j][op]) {
                    continue;
                }

                const targetDegree = i + (1 << op);
                const targetY = getY(targetDegree - minDegree, numDegree, height);
                const targetDim = getDimension(targetDegree);

                for (let l = 0; l < targetDim; l++) {
                    const targetX = mid + NODE_OFFSET * (l - (targetDim - 1) / 2);
                    if (actions[j][op][l] != 0) {
                        context.beginPath();
                        if (op == 0) {
                            context.moveTo(x, y);
                            context.lineTo(targetX, targetY);
                        } else {
                            const distance = Math.sqrt((targetX - x)*(targetX - x) + (targetY - y)*(targetY - y));
                            const looseness = 0.4;
//                            if (side[op] == 1) {
                                const angle = Math.acos((x - targetX)/distance);
                                context.arc(x - distance * Math.cos(angle - Math.PI/3),
                                    y - distance * Math.sin(angle - Math.PI/3),
                                    distance,
                                    angle - 2 * Math.PI/3,
                                    angle - Math.PI/3
                                );
/*                            } else {
                                const angle = Math.PI - Math.acos((x - targetX)/distance);
                                context.arc(x + distance * Math.cos(angle - Math.PI/3),
                                    y - distance * Math.sin(angle - Math.PI/3),
                                    distance,
                                    4 * Math.PI / 3 - angle,
                                    5 * Math.PI / 3 - angle
                                );

                            }*/
                        }
                        context.stroke();
                    }
                }
            }
        }
    }
}

function getAction(degree) {
    if (actions.has(degree)) {
        return actions.get(degree)
    } else {
        let result = new Array();
        let dim = getDimension(degree);
        for (let i = 0; i < dim; i++) {
            result.push(new Array());
        }
        actions.set(degree, result);
        return result;
    }
}

function getDimension(degree) {
    if (dimensions.has(degree)) {
        return dimensions.get(degree)
    } else {
        dimensions.set(degree, 0);
        return 0;
    }
}

function setDimension(degree, dim) {
    dimensions.set(degree, dim);
}

function getY(index, total, height) {
    return Math.round(0.5 + height - ((index + 1) / (total + 2) * height)) - 0.5;
}

function fromY(y, total, height) {
    return Math.round((1 - y / height) * (total + 2) - 1);
}

canvas.addEventListener("click", (e) => {
    const rect = canvas.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    const numDegree = maxDegree - minDegree;
    const degree = fromY(clickY, numDegree, canvas.height) + minDegree;
    const dim = getDimension(degree);
    const mid = Math.round(0.5 + canvas.width / 2) - 0.5;
    let idx = Math.round((clickX - mid) / NODE_OFFSET + (dim - 1) / 2);
    if (idx < 0) {
        idx = 0;
    } else if (idx >= dim) {
        idx = dim - 1;
    }

    switch (mode) {
        case "add":
            setDimension(degree, dim + 1);
            getAction(degree).push(new Array());
            for (let op = 0; degree - (1 << op) >= minDegree; op++) {
                const sourceDegree = degree - (1 << op);
                const sourceDimension = getDimension(sourceDegree);
                const action = getAction(sourceDegree);
                for (let i = 0; i < sourceDimension; i++) {
                    if (action[i].length > op) {
                        action[i][op].push(0);
                    }
                }
            }
            break;
        case "rm":
            if (dim > 0) {
                setDimension(degree, dim - 1);
                getAction(degree).splice(idx, 1);
                for (let op = 0; degree - (1 << op) >= minDegree; op++) {
                    const sourceDegree = degree - (1 << op);
                    const sourceDimension = getDimension(sourceDegree);
                    const action = getAction(sourceDegree);
                    for (let i = 0; i < sourceDimension; i++) {
                        if (action[i].length > op) {
                            action[i][op].splice(idx, 1);
                        }
                    }
                }
            }
            break;
        case "action":
            let op;
            while (true) {
                op = parseInt(prompt("Set Sq^{2^i} for i = ?"));
                if (isNaN(op)) {
                    if (!confirm("Invalid value of i")) return;
                } else if (degree + (1 << op) > maxDegree) {
                    if (!confirm("Value of i too large")) return;
                } else if (getDimension(degree + (1 << op)) == 0) {
                    if (!confirm("Target dimension is 0")) return;
                } else {
                    break;
                }
            }
            const targetDim = getDimension(degree + (1 << op));
            let value;
            while (true) {
                try {
                    value = JSON.parse(prompt("Set value of action").trim());
                    if (Array.isArray(value) &&
                        value.length == targetDim &&
                        value.reduce((b, x) => b && Number.isInteger(x) && b < window.prime, true)) {
                        break;
                    }
                } catch(e) { // If we can't parse, try again
                }
                if (!confirm("Invalid value. Please write action in the form [0, 1, ...]")) return;
            }

            getAction(degree)[idx][op] = value;
            break;
        default:
            console.log("Invalid mode");
    }
    drawCanvas();
});
drawCanvas();

document.getElementById("download").addEventListener("click", () => {
    let result = {};
    result.type = "finite dimensional module";
    result.name = prompt("Name");
    result.file_name = prompt("File Name (without extension)");
    result.p = window.prime;
    result.generic = (result.p != 2);
    result.gens = {};
    result.actions = [];
    for (let deg = minDegree; deg <= maxDegree; deg++) {
        const dim = getDimension(deg);
        const action = getAction(deg);
        for (let j = 0; j < dim; j++) {
            result.gens[`x_{${deg},${j}}`] = deg;
            for (let op = 0; op < action[j].length; op++) {
                const targetDeg = deg + (1 << op);
                if (!action[j][op]) {
                    continue;
                }
                let nonZero = false;

                let actionString = `Sq${1 << op} x_{${deg},${j}} = `;
                for (let k = 0 ; k < action[j][op].length; k++) {
                    const c = action[j][op][k];
                    if (c == 1) {
                        actionString += `x_{${targetDeg},${k}} + `;
                        nonZero = true;
                    } else if (c > 0) {
                        actionString += `${c} x_{${targetDeg},${k}} + `;
                        nonZero = true;
                    }
                }
                if (nonZero) {
                    result.actions.push(actionString.substring(0, actionString.length - 3));
                }
            }
        }
    }
    download(`${result.file_name}.json`, JSON.stringify(result), "application/json");
});
