'use strict';

import { parseIntegerArray, download, stringToB64 } from "./utils.js";

window.maxDegree = 10;
window.form = document.getElementById("header-form");

const LEFT_MARGIN = 20;
const NODE_OFFSET = 20;

// prime
form.prime.addEventListener("change", () => {
    window.prime = parseInt(form.prime.value);
    if (window.prime == 2) {
        form.qpart.disabled = true;
    } else {
        form.qpart.disabled = false;
    }
});
form.prime.dispatchEvent(new Event("change"));

form.algebra.addEventListener("change", () => {
    if (form.algebra.value == "Milnor") {
        document.getElementById("profile").style.display = "flex";
    } else {
        document.getElementById("profile").style.display = "none";
    }
});
form.algebra.dispatchEvent(new Event("change"));

form.ppart.addEventListener("change", () => {
    let value = form.ppart.value.trim();
    if (value == "") {
        form.ppart.setCustomValidity("");
        return;
    }
    value = parseIntegerArray(value);
    if (value === null) {
        form.ppart.setCustomValidity("Invalid syntax.");
    } else if (!validatePPart(value)) {
        form.ppart.setCustomValidity("Invalid profile function. A profile [p_1, p_2, ...] is valid iff p_i >= min(p_j, p_{i - j} - j) for all i > j.");
    } else {
        form.ppart.setCustomValidity("");
    }
});
form.ppart.dispatchEvent(new Event("change"));

function validatePPart(ppart) {
    for (let k = 1; k <= ppart.length; k++) {
        for (let j = 1; j < ppart[k - 1]; j++) {
            if (ppart[k + j - 1] === undefined)
                ppart[k + j - 1] = 0;
            if (ppart[j + k - 1] < ppart[j - 1] && ppart[j + k - 1] + j < ppart[k - 1]) {
                return false;
            }
        }
    }
    return true;
}

form["module-type"].addEventListener("change", () => {
    switch (form["module-type"].value) {
      case "Finite Dimensional Module":
        window.moduleType = "fdmodule";
        break;
      case "Finitely Presented Module":
        window.moduleType = "fpmodule";
        break;
      case "Stunted Projective Space":
        window.moduleType = "rpn";
        break;
      default:
        console.log(`Invalid module type: ${form["module-type"].value}`);
    }
    for (let x of ["fdmodule", "fpmodule", "rpn"]) {
      if (x == window.moduleType) {
        document.getElementById(x).style.display = "flex";
      } else {
        document.getElementById(x).style.display = "none";
      }
    }
  document.getElementById(window.moduleType).onShow();
});
// Dispatch event at the end when onShow is defined.

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
document.getElementById("fdmodule").onShow = drawCanvas;

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
                value = parseIntegerArray(prompt("Set value of action"));
                if (value !== null &&
                    value.length == targetDim &&
                    value.reduce((b, x) => b && x < window.prime, true)) {
                    break;
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

function getModuleObject(download) {
    // Check qpart and ppart are valid
    if (form.algebra.value == "Milnor") {
        const valid = form.ppart.validity.valid && (prime == 2 || form.qpart.validity.valid);
        if (!valid) {
            alert("Invalid Milnor algebra profile");
            return;
        }
    }
    let result = {};
    result.type = "finite dimensional module";
    if (download) {
        result.name = prompt("Name");
        if (result.name === null)
            return;

        result.file_name = prompt("File Name (without extension)");
        if (result.file_name === null)
            return;
    }

    result.p = window.prime;
    result.generic = (result.p != 2);
    result.gens = {};
    result.actions = [];
    switch (form.algebra.value) {
        case "Adem":
            result.algebra = ["adem"];
            break;
        case "Milnor":
            result.algebra = ["milnor"];
            const hasProfile = (form.ppart.value != "") || (prime > 2 && form.qpart.value != "");
            if (hasProfile) {
                result.profile = {};
                if (form.ppart.value != "") {
                    result.profile.truncated = true;
                    result.profile.p_part = parseIntegerArray(form.ppart.value);
                }
                if (prime > 2 && form.qpart.value != "") {
                    result.profile.q_part = parseInt(form.qpart.value);
                }
            }
            break;
    }
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
    return result;
}

document.getElementById("download").addEventListener("click", () => {
    const result = getModuleObject(true);
    download(`${result.file_name}.json`, JSON.stringify(result), "application/json");
});
document.getElementById("resolve").addEventListener("click", () => {
    const result = getModuleObject(false);
    const history = [
      {
        recipients: ["Resolver"],
        sseq : "Main",
        action : {
          "ConstructJson": {
            algebra_name : "adem",
            data : JSON.stringify(result),
          }
        }
      },
      {
        recipients: ["Resolver"],
        sseq : "Main",
        action : {
          "Resolve": {
            max_degree : 30
          }
        }
      }
    ];
    const historyString = history.map(JSON.stringify).join("\n");
    window.location.href = `index.html?data=${stringToB64(historyString)}`;
});


form["module-type"].dispatchEvent(new Event("change"));
