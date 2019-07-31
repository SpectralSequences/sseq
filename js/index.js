import "spectral-sequences";
import MyDisplay from "./display.js";

// Read URL to see if module is specified.
let url = new URL(document.location);
let params = {};
for(let [k,v] of url.searchParams.entries()){
    params[k] = v;
}

if (!params.module) {
    console.log("Displaying homepage");
    console.log(document.querySelector("#home"));
    document.querySelector("#home").style.removeProperty("display");

    HTMLCollection.prototype.forEach = Array.prototype.forEach;
    let sections = document.querySelector("#home").getElementsByTagName("section");

    sections.forEach(n => {
        n.children[1].children.forEach(a => {
            a.innerHTML = Interface.renderLaTeX(a.innerHTML);
            a.href = `?module=${a.getAttribute("data")}&degree=50`;
        });
    });
} else {
    window.worker = new Worker("./worker.js");

    let maxDegree = parseInt(params.degree ? params.degree : 50);
    let structlineTypes = new Set();

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
                displayFile(params.module, maxDegree);
                break;
            case "complete":
                display.runningSign.style.display = "none";
                break;
            default:
                break;
        }
    });
}

async function displayFile(filename, degree=50) {
    try {
        let module = await IO.loadFromServer(`modules/${filename}.json`);
        let min_degree = Math.min(...Object.values(module.gens));

        window.sseq = new Sseq();
        sseq.xRange = [min_degree, degree];
        sseq.yRange = [0, (degree - min_degree)/3];
        sseq.initialxRange = [min_degree, degree];
        sseq.initialyRange = [0, (degree - min_degree)/3];
        sseq.offset_size = 0.1;

        window.display = new MyDisplay("#main", sseq);

        worker.postMessage({
            cmd: "resolve",
            p: module.p,
            module: JSON.stringify(module),
            maxDegree: degree
        });
    } catch (e) {
        alert(`Failed to load file ${filename}.json`);
    }
}
