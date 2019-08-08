import MyDisplay from "./display.js";

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
    let webSocket = new WebSocket("ws://localhost:8080/ws");
    let maxDegree = parseInt(params.degree ? params.degree : 50);

    webSocket.onopen = function(e) {
        let data = {
            command : "resolve",
            algebra : "adem",
            module : params.module,
            maxDegree : maxDegree
        };

        webSocket.send(JSON.stringify(data));
    };

    webSocket.onmessage = function(e) {
        let data = JSON.parse(e.data);
        messageHandler[data.command](data);
    }
    window.sseq = new Sseq();
    sseq.xRange = [0, maxDegree];
    sseq.yRange = [0, Math.ceil(maxDegree/3)];
    sseq.initialxRange = [0, maxDegree];
    sseq.initialyRange = [0, Math.ceil(maxDegree/3)];
    sseq.offset_size = 0.1;
    sseq.class_scale = 0.5;
    document.querySelector("#main").style.display = "block";
    window.display = new MyDisplay("#main", sseq);
}

let messageHandler = {};
messageHandler.resolving = (data) => {
    let minDegree = data.minDegree;
    let maxDegree = data.maxDegree;
    sseq.xRange = [minDegree, maxDegree];
    sseq.yRange = [0, Math.ceil((maxDegree - minDegree)/3)];
}

messageHandler.addClass = function addClass(m) {
    sseq.addClass(m.x, m.y);
}

window.structlineTypes = new Set();
messageHandler.addStructline = function addStructline(m) {
    let source = sseq.getClassesInDegree(m.source.x, m.source.y)[m.source.idx];
    let target = sseq.getClassesInDegree(m.target.x, m.target.y)[m.target.idx];
    sseq.addStructline(source, target, m.mult);
    if (!structlineTypes.has(m.mult)) {
        self.structlineTypes.add(m.mult);
        display.sidebar.showPanel();
    }
}

messageHandler.complete = function complete(m) {
    display.runningSign.style.display = "none";
}
