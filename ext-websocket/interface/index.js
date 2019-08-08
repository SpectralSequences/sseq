import MyDisplay from "./display.js";

// For timer
let t0, t_last;

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

        t0 = performance.now();
        t_last = t0;
    };

    webSocket.onmessage = function(e) {
        let data = JSON.parse(e.data);
        messageHandler[data.command](data);
    }
    window.sseq = new Sseq();
    sseq.xRange = [0, maxDegree];
    sseq.yRange = [0, Math.ceil(maxDegree/3) + 1];
    sseq.initialxRange = [0, maxDegree];
    sseq.initialyRange = [0, Math.ceil(maxDegree/3) + 1];
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
    sseq.yRange = [0, Math.ceil((maxDegree - minDegree)/3) + 1];
}

let max_t = 0;
messageHandler.addClass = function addClass(m) {
    if (m.t > max_t) {
        max_t = m.t;
        if (max_t % 10 == 0) {
            console.log(`Time to compute stems ${max_t - 10} to ${max_t} : ${getTime()}`);
            console.log(`Total time to compute first ${max_t} stems : ${getTotalTime()}`);
        }
    }
    sseq.addClass(m.t - m.s, m.s);
}

window.structlineTypes = new Set();
messageHandler.addStructline = function addStructline(m) {
    let source = sseq.getClassesInDegree(m.source.t - m.source.s, m.source.s)[m.source.idx];
    let target = sseq.getClassesInDegree(m.target.t - m.target.s, m.target.s)[m.target.idx];
    sseq.addStructline(source, target, m.mult);
    if (!structlineTypes.has(m.mult)) {
        self.structlineTypes.add(m.mult);
        display.sidebar.showPanel();
    }
}

messageHandler.complete = function complete(m) {
    display.runningSign.style.display = "none";
    console.log(`Total time : ${getTotalTime()}`);
}


function getTime() {
    let t_cur = performance.now();
    let duration = (t_cur - t_last) / 1000;
    t_last = t_cur;
    return duration;
}

function getTotalTime(){
    let t_cur = performance.now();
    return (t_cur - t0) / 1000;
}
