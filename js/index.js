import "spectral-sequences";
import MyDisplay from "./display.js";

window.sseq = new Sseq();
window.display = new MyDisplay("#main");

const worker = new Worker("./worker.js");

let structlineTypes = new Set();

function getURLDictionary(){
    let url = new URL(document.location);
    let kv = {};
    for(let [k,v] of url.searchParams.entries()){
        kv[k] = v;
    }
    return kv;
}

worker.addEventListener("message", ev => {
    let m = ev.data;
    switch (m.cmd) {
        case "addClass":
            sseq.addClass(m.x, m.y);
            break;
        case "addStructline": 
            let source = sseq.getClassesInDegree(m.source.x, m.source.y)[m.source.idx];
            let target = sseq.getClassesInDegree(m.target.x, m.target.y)[m.target.idx];
            sseq.addStructline(source, target, m.mult);
            if (!structlineTypes.has(m.mult)) {
                structlineTypes.add(m.mult);
                display.sidebar.showPanel();
            }
            break;
        case "initialized":
            start();
            break;
        case "complete":
            console.log("complete");
            display.runningSign.style.display = "none";
            break;
        default:
            break;
    }
});

function start() {
    let params = getURLDictionary();

    let C2json = '{"name": "$C(2)$", "file_name": "C2", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "sq_actions": [{"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}';

    let maxDegree = parseInt(params.degree ? params.degree : 50);
    sseq.xRange = [0, maxDegree];
    sseq.yRange = [0, Math.floor(maxDegree / 2)];
    sseq.initialxRange = sseq.xRange;
    sseq.initialyRange = sseq.yRange;
    sseq.class_scale = 0.5;
    display.setSseq(sseq);

    worker.postMessage({
        cmd: "resolve",
        module: C2json,
        maxDegree: maxDegree
    });
}
