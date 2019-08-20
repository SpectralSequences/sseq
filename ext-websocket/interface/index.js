import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";

let commandQueue = [];
function processCommandQueue() {
    let command = "";
    // If we are resolving, we should wait for it to finish resolving before we
    // can continue. For example, we don't want to add a differential when the
    // corresponding classes have not been generated.
    while (commandQueue.length > 0 && !command.includes('"Resolve"')) {
        command = commandQueue.pop().trim();
        if (command.length == 0)
            continue;
        window.webSocket.send(command);
    }
}

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
            if (a.tagName == "A") {
                a.innerHTML = Interface.renderLaTeX(a.innerHTML);
                a.href = `?module=${a.getAttribute("data")}&degree=50`;
            }
        });
    });
} else {
    let maxDegree = parseInt(params.degree ? params.degree : 50);

    openWebSocket([
        {
            recipients: ["Resolver"],
            sseq : "Main",
            action : {
                "Construct": {
                    algebra_name : "adem",
                    module_name : params.module,
                }
            }
        },
        {
            recipients: ["Resolver"],
            sseq : "Main",
            action : {
                "Resolve": {
                    max_degree : maxDegree
                }
            }
        },
    ]);
}

function openWebSocket(initialData, maxDegree) {
    window.webSocket = new WebSocket(`ws://${window.location.host}/ws`);

    webSocket.onopen = function(e) {
        for (let data of initialData) {
            if (data instanceof Object) {
                webSocket.send(JSON.stringify(data));
            } else {
                webSocket.send(data);
            }
        }
    };

    webSocket.onmessage = function(e) {
        let data = JSON.parse(e.data);
        try {
            let command = Object.keys(data.action)[0];
            if (messageHandler[command]) {
                messageHandler[command](data.action[command], data);
            } else {
                switch (data.sseq) {
                    case "Main":
                        window.mainSseq["process" + command](data.action[command]);
                        break;
                    case "Unit":
                        window.unitSseq["process" + command](data.action[command]);
                        break;
                    default:
                }
            }
        } catch (err) {
            console.log("Unable to process message");
            console.log(data);
            console.log(`Error: ${err}`);
        }
    }
}

function requestHistory() {
    window.webSocket.send(JSON.stringify({
        recipients: ["Server"],
        sseq : "Main",
        action : { "RequestHistory": { } }
    }));
}
window.requestHistory = requestHistory;

function setUnitRange() {
    let maxX = Math.max(unitSseq.maxDegree, mainSseq.maxDegree)
    unitSseq.xRange = [0, maxX];
    unitSseq.yRange = [0, Math.min(unitSseq.maxDegree, Math.ceil(maxX/2 + 1))];
    unitSseq.initialxRange = [0, maxX];
    unitSseq.initialyRange = [0, Math.min(unitSseq.maxDegree, Math.ceil(maxX/2 + 1))];
}

window.setUnitRange = setUnitRange;

let messageHandler = {};
messageHandler.ReturnHistory = (data) => {
    let filename = prompt("Input filename");
    IO.download(filename, data.history.map(JSON.stringify).join("\n"), "text/plain");
}

messageHandler.Resolving = (data, msg) => {
    if (msg.sseq == "Unit") { return; }
    if (!window.mainSseq) {
        window.mainSseq = new ExtSseq("Main", window.webSocket);
        window.unitSseq = new ExtSseq("Unit", window.webSocket);
    }

    window.mainSseq.processResolving(data);

    unitSseq.maxDegree = 9;
    setUnitRange();

    if (!window.display) {
        window.display = new MainDisplay("#main", mainSseq);
        window.unitDisplay = new UnitDisplay("#modal-body", unitSseq);
    }

    display.runningSign.style.removeProperty("display");
}

messageHandler.Complete = function (m) {
    display.runningSign.style.display = "none";
    processCommandQueue();
}

messageHandler.QueryTableResult = function (m) {
    console.log(`Table for (t - s, s) = (${m.t - m.s}, ${m.s}):`);
    console.log(m.string);
}

// Set up upload button
document.getElementById("json-upload").addEventListener("change", function() {
    let maxDegree = parseInt(prompt("Maximum degree", 30).trim());

    let file = document.getElementById("json-upload").files[0];
    let fileReader = new FileReader();
    fileReader.onload = e => {
        openWebSocket([
            {
                recipients: ["Resolver"],
                sseq : "Main",
                action : {
                    "ConstructJson": {
                        algebra_name : "adem",
                        data : e.target.result,
                    }
                }
            },
            {
                recipients: ["Resolver"],
                sseq : "Main",
                action : {
                    "Resolve": {
                        max_degree : maxDegree
                    }
                }
            },
        ]);
    };
    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
document.getElementById("history-upload").addEventListener("change", function() {
    let file = document.getElementById("history-upload").files[0];

    let fileReader = new FileReader();
    fileReader.onload = e => {
        let lines = e.target.result.split("\n");
        let firstBatch = [];

        let i = 0;
        for (let line of lines) {
            i++;
            if (line.includes("Resolve")) {
                break;
            }
        }
        openWebSocket(lines.splice(0, i + 1));

        lines.reverse();
        commandQueue = lines;
    };

    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
