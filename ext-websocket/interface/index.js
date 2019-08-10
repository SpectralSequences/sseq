import MyDisplay from "./display.js";

// For timer
let t0, t_last;
let t_prev = 0;

let callbacks = {};
callbacks.resolveFurther = () => {
    let newmax = parseInt(prompt("New maximum degree", window.maxDegree + 10).trim());
    if (newmax <= window.maxDegree) {
        return;
    }
    window.maxDegree = newmax;
    t0 = performance.now();
    t_last = t0;
    webSocket.send(JSON.stringify({
            command : "resolve_further",
            maxDegree : maxDegree
        }));
    sseq.xRange = [window.minDegree, window.maxDegree];
    sseq.yRange = [0, Math.ceil((window.maxDegree - window.minDegree)/3) + 1];
};

let url = new URL(document.location);
let params = {};
for(let [k,v] of url.searchParams.entries()){
    params[k] = v;
}

if (!params.module) {
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
    window.maxDegree = parseInt(params.degree ? params.degree : 50);
    openWebSocket(
        {
            command : "resolve",
            algebra : "adem",
            module : params.module,
            maxDegree : maxDegree
        });
}

function openWebSocket(initialData, maxDegree) {
    window.webSocket = new WebSocket(`ws://${window.location.host}/ws`);

    webSocket.onopen = function(e) {
        webSocket.send(JSON.stringify(initialData));

        t0 = performance.now();
        t_last = t0;
    };

    webSocket.onmessage = function(e) {
        let data = JSON.parse(e.data);
        messageHandler[data.command](data);
    }
    window.sseq = new Sseq();
    sseq.offset_size = 0.1;
    sseq.class_scale = 0.5;

    var maxDegree = initialData.maxDegree;
    if (maxDegree) {
        sseq.xRange = [0, maxDegree];
        sseq.yRange = [0, Math.ceil(maxDegree/3) + 1];
        sseq.initialxRange = [0, maxDegree];
        sseq.initialyRange = [0, Math.ceil(maxDegree/3) + 1];
    }
    window.display = new MyDisplay("#main", sseq, callbacks);
}
let messageHandler = {};
messageHandler.resolving = (data) => {
    window.minDegree = data.minDegree;
    window.maxDegree = data.maxDegree;
    sseq.xRange = [window.minDegree, window.maxDegree];
    sseq.yRange = [0, Math.ceil((window.maxDegree - window.minDegree)/3) + 1];
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
    t_prev = getTotalTime() * 1000;
}

function getTime() {
    let t_cur = performance.now();
    let duration = (t_cur - t_last) / 1000;
    t_last = t_cur;
    return duration;
}

function getTotalTime(){
    let t_cur = performance.now();
    return (t_cur - t0 + t_prev) / 1000;
}

// Set up upload button
document.getElementById("json-upload").addEventListener("change", function() {
    let maxDegree = parseInt(prompt("Maximum degree", 30).trim());

    let file = document.getElementById("json-upload").files[0];
    let fileReader = new FileReader();
    fileReader.onload = e => {
        openWebSocket(
            {
                command : "resolve_json",
                algebra : "adem",
                data : e.target.result,
                maxDegree : maxDegree
            }
        );
    };
    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
