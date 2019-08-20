import { MainDisplay, UnitDisplay } from "./display.js";
import { ExtSseq } from "./sseq.js";

let commandQueue = [];
function processCommandQueue() {
    let commandText = "";
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
    // Keep this for the save button
    window.constructCommand = initialData[0];

    window.webSocket = new WebSocket(`ws://${window.location.host}/ws`);

    webSocket.onopen = function(e) {
        for (let data of initialData) {
            webSocket.send(JSON.stringify(data));
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
    if (unitSseq.maxDegree > 9) {
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
    IO.download(filename, list.map(JSON.stringify).join("\n"), "text/plain");
}
window.save = save;

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
    // Replace the getter with hand-coded actual values;
    Object.defineProperty(unitSseq, "xRange", {
        get() { return [0, Math.max(unitSseq.maxDegree, mainSseq.maxDegree)] }
    });
    Object.defineProperty(unitSseq, "yRange", {
        get() { return [0, Math.min(unitSseq.maxDegree, Math.ceil(this.xRange[1]/2 + 1))] }
    });
    Object.defineProperty(unitSseq, "initialxRange", {
        get() { return this.xRange; }
    });
    Object.defineProperty(unitSseq, "initialyRange", {
        get() { return this.yRange; }
    });

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

        // First command is construct and second command is resolve
        openWebSocket(lines.splice(0, 2).map(JSON.parse));

        lines.reverse();
        commandQueue = lines;
    };

    fileReader.readAsText(file, "UTF-8");
    document.querySelector("#home").style.display = "none";
});
