'use strict';

let url = new URL(document.location);
let params = {};
for(let [k,v] of url.searchParams.entries()){
    params[k] = v;
}

const LEFT_MARGIN = 20;
const BOTTOM_MARGIN = 20;

let filename = params["class"];
if (filename) {
  (async () => {
    let f = await fetch(`./${filename}.json`);
    let data = await f.json();

    window.canvas = document.createElement("canvas");
    document.body.appendChild(canvas);

    window.context = canvas.getContext("2d");

    window.zoom = d3.zoom()
        .scaleExtent([1, 8])
        .on("zoom", () => drawRepresentative(data));

    d3.select(window.canvas).call(window.zoom);

    drawRepresentative(data);
    window.addEventListener("resize", () => drawRepresentative(data));
  })();
} else {
  let home = document.querySelector("#home");
  home.style.removeProperty("display");

  HTMLCollection.prototype.forEach = Array.prototype.forEach;
  let div = home.children[1];

  div.children.forEach(a => {
    a.href = `?class=${a.getAttribute("data")}`;
  });
}

function drawRepresentative(data) {
    const transform = d3.zoomTransform(window.canvas);

    const boundingRectangle = document.body.getBoundingClientRect();
    const width = boundingRectangle.width;
    const height = boundingRectangle.height;

    canvas.width = width;
    canvas.height = height;

    let num_degree = 0;
    for (let d of data) {
        num_degree = Math.max(num_degree, d.graded_dimension.length);
    }
    let module_width = Math.floor((width - LEFT_MARGIN * 2)/data.length);

    context.textAlign = "center";
    context.textBaseline = "middle";
    for (let i = 0; i < num_degree; i++) {
        let y = getY(i, num_degree, height - BOTTOM_MARGIN);
        y = transform.apply([0, y])[1];

        context.fillText(i, LEFT_MARGIN, y);
        context.fillText(i, width - LEFT_MARGIN, y);
    }

    let saved = context.save();
    context.beginPath();
    context.rect(LEFT_MARGIN * 2, 0, width - LEFT_MARGIN * 4, height);
    context.clip();
    window.zoom.translateExtent([[0, 0], [width, height]]);

    context.strokeStyle = "rgba(0, 0, 0, 0.3)";
    for (let i = 0; i < num_degree; i++) {
        let y = getY(i, num_degree, height - BOTTOM_MARGIN);
        context.beginPath();
        context.moveTo(...transform.apply([LEFT_MARGIN * 2, y]));
        context.lineTo(...transform.apply([width - LEFT_MARGIN * 2, y]));
        context.stroke();
    }
    context.strokeStyle = "black";

    for (let i = 0; i < data.length; i++) {
        drawModule(context, data[i], i * module_width + LEFT_MARGIN, module_width, height, num_degree);
    }
    context.restore(saved);
}
function drawModule(context, data, x, width, height, num_degree) {
    const transform = d3.zoomTransform(window.canvas);
    const offset = 8;
    const mid = Math.round(x + width/2) + 0.5;

    context.fillText(data.graded_dimension.reduce((a, b) => a + b, 0), transform.apply([mid, height])[0], height - BOTTOM_MARGIN * 0.7);

    for (let i = 0; i <= num_degree; i++) {
        const y = getY(i, num_degree, height - BOTTOM_MARGIN);
        const dim = data.graded_dimension[i];
        if (dim == 0) {
            continue;
        }

        for (let j = 0; j < dim; j++) {
            context.beginPath();
            context.arc(...transform.apply([mid + offset * (j - (dim - 1) / 2) , y]), 3, 0, 2 * Math.PI);
            context.stroke();
            context.fill();
        }
    }
    let side = [];
    for (let action of data.actions) {
        const source_dim = data.graded_dimension[action.input_deg];
        const sourceY_ = getY(action.input_deg, num_degree, height - BOTTOM_MARGIN);
        const sourceX_ = mid + offset * (action.input_idx - (source_dim - 1) / 2);

        const [sourceX, sourceY] = transform.apply([sourceX_, sourceY_]);

        const op = action.op[0];
        const target_deg = action.input_deg + op;
        const target_dim = data.graded_dimension[target_deg];
        const targetY_ = getY(target_deg, num_degree, height - BOTTOM_MARGIN);

        if (!side[op]) {
            side[op] = 1;
        }
        for (let j = 0; j < target_dim; j++) {
            if (action.output[j] != 0) {
                const targetX_ = mid + offset * (j - (target_dim - 1) / 2);

                const [targetX, targetY] = transform.apply([targetX_, targetY_]);

                context.beginPath();
                if (op == 1) {
                    context.moveTo(sourceX, sourceY);
                    context.lineTo(targetX, targetY);
                } else {
                    let distance = Math.sqrt((targetX - sourceX)*(targetX - sourceX) + (targetY - sourceY)*(targetY - sourceY));
                    let looseness = 0.4;
                    if (side[op] == 1) {
                        let angle = Math.acos((sourceX - targetX)/distance);
                        context.arc(sourceX - distance * Math.cos(angle - Math.PI/3),
                            sourceY - distance * Math.sin(angle - Math.PI/3),
                            distance,
                            angle - 2 * Math.PI/3,
                            angle - Math.PI/3
                        );
                    } else {
                        let angle = Math.PI - Math.acos((sourceX - targetX)/distance);
                        context.arc(sourceX + distance * Math.cos(angle - Math.PI/3),
                            sourceY - distance * Math.sin(angle - Math.PI/3),
                            distance,
                            4 * Math.PI / 3 - angle,
                            5 * Math.PI / 3 - angle
                        );

                    }
                    side[op] *= -1;
                }
                context.stroke();
            }
        }
    }
}

function getY(index, total, height) {
    return Math.round(height - ((index + 1) / (total + 1) * height));
}
