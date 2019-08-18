import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";

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
    let maxDegree = parseInt(params.degree ? params.degree : 50);

    openWebSocket(
        {
            recipient: "resolver",
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
    };

    webSocket.onmessage = function(e) {
        let data = JSON.parse(e.data);
        try {
            switch (data.recipient) {
                case "main":
                    window.mainSseq["_" + data.command](data);
                    break;
                case "unit":
                    window.unitSseq["_" + data.command](data);
                    break;
                default:
                    messageHandler[data.command](data);
            }
        } catch (err) {
            console.log("Unable to process message");
            console.log(data);
            console.log(`Error: ${err}`);
        }
    }
}

function setUnitRange() {
    let maxX = Math.max(unitSseq.maxDegree, mainSseq.maxDegree)
    unitSseq.xRange = [0, maxX];
    unitSseq.yRange = [0, Math.min(unitSseq.maxDegree, Math.ceil(maxX/2 + 1))];
    unitSseq.initialxRange = [0, maxX];
    unitSseq.initialyRange = [0, Math.min(unitSseq.maxDegree, Math.ceil(maxX/2 + 1))];
}

window.setUnitRange = setUnitRange;

let messageHandler = {};
messageHandler.resolving = (data) => {
    if (!window.mainSseq) {
        window.mainSseq = new ExtSseq("main", window.webSocket);
        window.unitSseq = new ExtSseq("unit", window.webSocket);
    }

    window.mainSseq._resolving(data);

    unitSseq.maxDegree = 9;
    setUnitRange();

    if (!window.display) {
        window.display = new MainDisplay("#main", mainSseq);
        window.unitDisplay = new UnitDisplay("#modal-body", unitSseq);
    }

    display.runningSign.style.removeProperty("display");
}

messageHandler.complete = function (m) {
    display.runningSign.style.display = "none";
}

messageHandler.queryTableResult = function (m) {
    console.log(`Table for (t - s, s) = (${m.t - m.s}, ${m.s}):`);
    console.log(m.string);
}

// Set up upload button
document.getElementById("json-upload").addEventListener("change", function() {
    let maxDegree = parseInt(prompt("Maximum degree", 30).trim());

    let file = document.getElementById("json-upload").files[0];
    let fileReader = new FileReader();
    fileReader.onload = e => {
        openWebSocket(
            {
                recipient : "resolver",
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
