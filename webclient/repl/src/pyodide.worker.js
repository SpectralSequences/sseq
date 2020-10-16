self.languagePluginUrl = '/pyodide-build-custom/'
importScripts(`${self.languagePluginUrl}pyodide.js`);

import { sleep } from "./utils";

self.sleep = sleep;

self.fetch = fetch.bind(self);

async function is_promise(obj){
    return obj && typeof obj.then == 'function';
}
self.is_promise = is_promise;

/**  
 * NOTE: When pyodide is finished initializing, the original "pyodide" object
 * is stored as "pyodide._module". We don't want to wait for this to happen or worry 
 * about when the move occurs, so we just store it and use that.
 */


import { files_to_install } from "./python_imports";

// The pyodide loader will move what is currently called "pyodide" into pyoide._module.
let pyodide_FS = pyodide.FS;
pyodide_FS.mkdir('/executor');
pyodide_FS.mkdir('/executor/executor');
for(let [k, v] of Object.entries(files_to_install)){
    pyodide_FS.writeFile(`/executor/executor/${k}.py`, v);
}


function sendMessage(message){
    self.postMessage(message);
}
self.sendMessage = sendMessage;
self.messageLookup = {};

async function startup(){
    try {
        await languagePluginLoader;
        await pyodide.loadPackage([
            // "pygments", 
            "crappy-python-multitasking",
            "spectralsequence_chart"
        ]);
        pyodide.runPython(`
            import sys
            sys.path.append("/executor")
            from executor import PyodideExecutor
            from executor.sseq_display import SseqDisplay
            executor = PyodideExecutor()
        `);
        self.postMessage({cmd : "ready"});
    } catch(e){
        self.postMessage({cmd : "ready", exception : e});
    }
}
let startup_promise = startup();

self.subscribers = [];

self.addEventListener("message", async function(e) {
    if(e.data.cmd === "service_worker_channel"){
        registerServiceWorkerPort(e);
        return;
    }

    await startup_promise;
    const {uuid, interrupt_buffer} = e.data;
    messageLookup[uuid] = e.data;
    
    if(interrupt_buffer){
        e.data.interrupt_buffer = function(){
            return interrupt_buffer[0]; 
            // return Atomics.load(interrupt_buffer, 0);
        }
    }
    await self.pyodide.runPythonAsync(`executor.handle_message("${uuid}")`);
});


function registerServiceWorkerPort(e){
    let { port } = e.data;
    self.serviceWorker = port;
    self.serviceWorker.addEventListener("message", handleMessageFromServiceWorker)
    port.start();
}

function handleMessageFromServiceWorker(event) {
    if(event.data.cmd === "subscribe_chart_display"){
        registerNewSubscriber(event);
        return;
    }
    console.error(`Unknown command: ${event.data.cmd}`, event.data, event);
    throw Error(`Unknown command: ${event.data.cmd}`);
}

function registerNewSubscriber(event){
    let { port, chart_name, uuid, client_id } = event.data;
    console.log(`New subscriber to ${chart_name}`, event.data);
    port.addEventListener("message", (e) => handleMessageFromChart(e, port, chart_name, client_id));
    port.start();
    self.subscribers.push(port);
}

function handleMessageFromChart(event, port, chart_name, client_id){
    let message = event.data;
    let { uuid } = JSON.parse(message);
    messageLookup[uuid] = { message, chart_name, port, client_id };
    console.log("message from chart:", message);
    pyodide.runPython(`SseqDisplay.dispatch_message("${uuid}")`);
}