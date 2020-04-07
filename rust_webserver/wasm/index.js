'use strict';

import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";
import { renderLaTeX, download, stringToB64, b64ToString } from "./utils.js";

window.resolutionWorker = new Worker("./resolution_worker.js");
window.sseqWorker = new Worker("./sseq_worker.js");

window.resolutionWorker.addEventListener("message", ev => window.sseqWorker.postMessage(ev.data));

window.sseqWorker.addEventListener("message", (ev) => {
    let data = JSON.parse(ev.data);
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
});

window.commandCounter = 0;
window.commandQueue = [];
window.onComplete = [];
function processCommandQueue() {
    if (commandQueue.length == 0)
        return;

    let commandText = "";
    let block = {
        recipients : ["Resolver", "Sseq"],
        action : { "BlockRefresh" : { block : true } }
    };

    window.mainSseq.send(block);
    if (!window.mainSseq.isUnit) {
        window.unitSseq.send(block);
    }
    // If we are resolving, we should wait for it to finish resolving before we
    // can continue. For example, we don't want to add a differential when the
    // corresponding classes have not been generated.
    while (commandQueue.length > 0 && !commandText.includes('"Resolve"')) {
        commandText = commandQueue.pop();
        if (commandText.trim() == "")
            continue;

        try {
            let command = JSON.parse(commandText);
            if (command.sseq == "Main") {
                window.mainSseq.send(command);
            } else {
                window.unitSseq.send(command);
            }
        } catch (e) {
            console.log("Unable to parse command " + commandText);
            console.log(e);
        }
    }
    block.action.BlockRefresh.block = false;

    window.mainSseq.send(block);
    if (!window.mainSseq.isUnit) {
        window.unitSseq.send(block);
    }
}

let url = new URL(document.location);
let params = {};
for(let [k,v] of url.searchParams.entries()){
    params[k] = v;
}

if (params.module) {
    let maxDegree = parseInt(params.degree ? params.degree : 50);
    window.constructCommand = {
        recipients: ["Resolver"],
        sseq : "Main",
        action : {
            "Construct": {
                algebra_name : "adem",
                module_name : params.module,
            }
        }
    };

    (async () => {
        let response = await fetch(`steenrod_modules/${params.module}.json`);
        let json = await response.json();

        openWebSocket([
            {
                recipients: ["Resolver"],
                sseq : "Main",
                action : {
                    "ConstructJson": {
                        algebra_name : "adem",
                        data : JSON.stringify(json),
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
    })();
} else if (params.data) {
    let data = b64ToString(params.data);
    loadHistory(data);
} else {
    document.querySelector("#home").style.removeProperty("display");

    HTMLCollection.prototype.forEach = Array.prototype.forEach;
    let sections = document.querySelector("#home").getElementsByTagName("section");

    sections.forEach(n => {
        n.children[1].children.forEach(a => {
            if (a.tagName == "A") {
                a.innerHTML = renderLaTeX(a.innerHTML);
                a.href = `?module=${a.getAttribute("data")}&degree=50`;
            }
        });
    });
}

function send(msg) {
    commandCounter += msg.recipients.length;
    if (window.display !== undefined)
        display.runningSign.style.removeProperty("display");

    let str = JSON.stringify(msg);
    for (let recipient of msg.recipients) {
        if (recipient == "Sseq") {
            window.sseqWorker.postMessage(str);
        } else {
            window.resolutionWorker.postMessage(str);
        }
    }
}
window.send = send;

function openWebSocket(initialData, maxDegree) {
    // Keep this for the save button
    for (let data of initialData) {
        send(data);
    }
}

function getHistoryLink() {
    return `${url.origin}${url.pathname}?data=` + stringToB64(generateHistory());
}
window.getHistoryLink = getHistoryLink;

function generateHistory() {
    let list = [window.constructCommand];
    list.push(
        {
            recipients: ["Resolver"],
            sseq : "Main",
            action : {
                "Resolve": {
                    max_degree : mainSseq.maxDegree
                }
            }
        }
    );
    if (!window.display.isUnit && unitSseq.maxDegree > 9) {
        list.push(
            {
                recipients: ["Resolver"],
                sseq : "Unit",
                action : {
                    "Resolve": {
                        max_degree : unitSseq.maxDegree
                    }
                }
            }
        );
    };
    list = list.concat(mainSseq.history);

    return list.map(JSON.stringify).join("\n")
}

function save() {
    let filename = prompt("Input filename");
    download(filename, generateHistory(), "text/plain");
}
window.save = save;

async function loadHistory(hist) {
    let lines = hist.split("\n");
    let firstTwo = lines.splice(0, 2).map(JSON.parse);

    window.constructCommand = Object.assign({}, firstTwo[0]); // Shallow copy is enough since we will replace firstTwo[0].action

    // Make this work well with Construct
    if (firstTwo[0].action["Construct"]) {
        let name = firstTwo[0].action["Construct"].module_name;

        let response = await fetch(`steenrod_modules/${name}.json`);
        let json = await response.json();
        firstTwo[0].action = {
            "ConstructJson": {
                algebra_name : "adem",
                data : JSON.stringify(json)
            }
        }
    }

    // First command is construct and second command is resolve
    openWebSocket(firstTwo);

    // Do reverse loop because we are removing things from the array.
    for (let i = lines.length - 1; i>= 0; i--) {
        if (lines[i].startsWith("//") || lines[i].trim() === "") {
            lines.splice(i, 1);
        }
    }

    lines.reverse();
    commandQueue = lines;
}

let messageHandler = {};
messageHandler.Resolving = (data, msg) => {
    if (msg.sseq == "Unit") {
        window.unitSseq.processResolving(data);
        return;
    }
    if (!window.mainSseq) {
        window.mainSseq = new ExtSseq("Main", data.min_degree);
        window.mainSseq.isUnit = data.is_unit;
        if (data.is_unit) {
            window.unitSseq = window.mainSseq;
        } else {
            window.unitSseq = new ExtSseq("Unit", 0);

            unitSseq.maxDegree = 9;
            Object.defineProperty(unitSseq, "maxX", {
                get() { return Math.max(unitSseq.maxDegree, mainSseq.maxDegree) }
            });
            Object.defineProperty(unitSseq, "maxY", {
                get() { return Math.min(unitSseq.maxDegree, Math.ceil(unitSseq.maxX/2 + 1)); }
            });
        }
    }

    window.mainSseq.processResolving(data);

    if (!window.display) {
        if (data.is_unit) {
            window.display = new MainDisplay("#main", mainSseq, data.is_unit);
        } else {
            window.display = new MainDisplay("#main", mainSseq, data.is_unit);
            window.unitDisplay = new UnitDisplay("#unitsseq-body", unitSseq);
        }
        window.display.runningSign.style.removeProperty("display");
    }
}

messageHandler.Complete = function (m) {
    commandCounter --;
    if (commandCounter == 0) {
        display.runningSign.style.display = "none";
        processCommandQueue();
        let f;
        while (f = window.onComplete.pop()) {
            f();
        }
    }
}

messageHandler.QueryCocycleStringResult = function (m) {
    console.log(`Cocyle string for (t - s, s, idx) = (${m.t - m.s}, ${m.s}, ${m.idx}):`);
    console.log(m.string);
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
        window.constructCommand = {
            recipients: ["Resolver"],
            sseq : "Main",
            action : {
                "ConstructJson": {
                    algebra_name : "adem",
                    data : e.target.result,
                }
            }
        };

        openWebSocket([
            window.constructCommand,
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
    fileReader.onload = (e) => {
        loadHistory(e.target.result);
    };

    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
