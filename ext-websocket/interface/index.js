'use strict';

import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";
import { renderLaTeX, download } from "./utils.js";

window.commandCounter = 0;
let commandQueue = [];
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

if (!params.module) {
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
} else {
    let maxDegree = parseInt(params.degree ? params.degree : 50);
    let algebra = params.algebra ? params.algebra : "adem";

    openWebSocket([
        {
            recipients: ["Resolver"],
            sseq : "Main",
            action : {
                "Construct": {
                    algebra_name : algebra,
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

function send(msg) {
    commandCounter += msg.recipients.length;
    if (window.display !== undefined)
        display.runningSign.style.removeProperty("display");

    window.webSocket.send(JSON.stringify(msg));
}
window.send = send;

function openWebSocket(initialData, maxDegree) {
    // Keep this for the save button
    window.constructCommand = initialData[0];

    window.webSocket = new WebSocket(`ws://${window.location.host}/ws`);

    webSocket.onopen = function(e) {
        for (let data of initialData) {
            window.send(data);
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

function save() {
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
    let filename = prompt("Input filename");
    download(filename, list.map(JSON.stringify).join("\n"), "text/plain");
}
window.save = save;

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
        // Do reverse loop because we are removing things from the array.
        for (let i = lines.length - 1; i>= 0; i--) {
            if (lines[i].startsWith("//") || lines[i].trim() === "") {
                lines.splice(i, 1);
            }
        }
        let firstBatch = [];

        // First command is construct and second command is resolve
        openWebSocket(lines.splice(0, 2).map(JSON.parse));

        lines.reverse();
        commandQueue = lines;
    };

    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
