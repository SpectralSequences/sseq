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
    self.maxDegree = parseInt(params.degree ? params.degree : 50);
    self.structlineTypes = new Set();
    worker.addEventListener("message", ev => {
        let m = ev.data;
        if(!(m.cmd in message_handlers)){
            console.error(`Unknown command '${m.cmd}'`);
            return;
        }        
        message_handlers[m.cmd](m);
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
        display.on("click", (node) => {
            if(node === undefined) {
                return;
            }
            let c = node.c;
            worker.postMessage({
                "cmd" : "getCocycle",//'get_eta_map',//"get_eta_map",
                "class" : {
                    "x" : c.x,
                    "y" : c.y,
                    "idx" : c.idx
                }
            });
        })
        worker.postMessage({
            cmd: "resolve",
            p: module.p,
            module: JSON.stringify(module),
            maxDegree: degree
        });
    } catch (e) {
        console.error(e);
        alert(`Failed to load file ${filename}.json`);
    }
}

let message_handlers = {};

message_handlers.addClass = function addClass(m) {
    sseq.addClass(m.x, m.y);
}

message_handlers.addStructline = function addStructline(m) {
    let source = sseq.getClassesInDegree(m.source.x, m.source.y)[m.source.idx];
    let target = sseq.getClassesInDegree(m.target.x, m.target.y)[m.target.idx];
    sseq.addStructline(source, target, m.mult);
    if (!structlineTypes.has(m.mult)) {
        self.structlineTypes.add(m.mult);
        display.sidebar.showPanel();
    }
}
    
message_handlers.initialized = function initialized(m){
    displayFile(params.module, self.maxDegree);
}

message_handlers.complete = function complete(m){
    display.runningSign.style.display = "none";
}

message_handlers.cocycleResult = function cocycleResult(m){
    console.log(`class : (${m.class.x}, ${m.class.y}, ${m.class.idx}), cocycle : ${m.cocycle}`);
}