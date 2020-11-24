// This pyodide worker starts the pyodide runtime on a worker thread.
// It talks to pythonExecutor, which is responsible for wrapping communication between the main thread and the pyodide thread.
// 
import { v4 as uuid4 } from "uuid";
import { sleep } from "./utils";


self.languagePluginUrl = 'pyodide-build-custom/'
importScripts(`${self.languagePluginUrl}pyodide.js`);

self.sleep = sleep;

self.fetch = fetch.bind(self);

async function is_promise(obj){
    return obj && typeof obj.then == 'function';
}
self.is_promise = is_promise;


// files_to_install is a map file_name => file_contents for the files in the python directory. 
// It's produced by the webpack prebuild script scripts/bundle_python_sources.py

let outBuffer = [];
let lastStreamFunc = undefined;
function makeOutputStream(streamFunc){
    function writeToStream(charCode){
        // console.log("hi?");
        // console.error("writeToStream", charCode);
        // outBuffer.push(String.fromCharCode(charCode));
        if(lastStreamFunc && lastStreamFunc !== streamFunc){
            // console.error("writing!");
            lastStreamFunc(outBuffer.join(""));
            outBuffer = [];
            lastStreamFunc = undefined;
        }
        if(charCode === 10 || !charCode){
            // console.error("writing!");
            // console.log(outBuffer.join(""));
            streamFunc(outBuffer.join(""));
            outBuffer = [];
            lastStreamFunc = undefined;
        } else {
            // lastStreamFunc = streamFunc;
            outBuffer.push(String.fromCharCode(charCode));
            // console.log("ip::", outBuffer.join(""));
        }
    }
    return writeToStream;
}

import { files_to_install } from "./python_imports";
function initializeFileSystem(){
    /**  
     * NOTE: When pyodide is finished initializing, the original "pyodide" object
     * is stored as "pyodide._module" (so then FS is pyodide._module.FS). 
     * We don't want to wait for this to happen and I'm not sure when the exactly
     * the move occurs, but this code consistently executes before the move.
     */
    let pyodide_FS = pyodide.FS;
    let stdoutStream = makeOutputStream(console.log);
    let stderrStream = makeOutputStream(console.error);

    pyodide_FS.init(() => null, stdoutStream, stderrStream);
    pyodide_FS.mkdir('/repl');
    pyodide_FS.mkdir('/repl/repl');
    for(let [k, v] of Object.entries(files_to_install)){
        pyodide_FS.writeFile(`/repl/repl/${k}.py`, v);
    }
}
initializeFileSystem();


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
            ],
            (msg) => console.log(msg),
            (err) => console.error(msg)
        );
        pyodide.runPython(`
            import sys
            sys.path.append("/repl")
            from repl import PyodideExecutor, get_namespace, SseqDisplay
            namespace = get_namespace()
            import jedi # This is slow but better to do it up front.
            jedi.Interpreter("SseqDisplay", [namespace]).completions() # Maybe this will reduce Jedi initialization time?
            executor = PyodideExecutor(namespace)
        `);
        self.postMessage({cmd : "ready"});
    } catch(e){
        self.postMessage({cmd : "ready", exception : e});
    }
}
let startup_promise = startup();

self.subscribers = [];

let handledCommands = {
    service_worker_channel : registerServiceWorkerPort,
    respondToQuery : handleQueryResponse
}

self.addEventListener("message", async function(e) {
    if(handledCommands[e.data.cmd]){
        handledCommands[e.data.cmd](e);
        return;
    }
    await startup_promise;
    // interrupt_buffer is a single byte SharedArrayBuffer used to signal a keyboard interrupt.
    // If it contains 0, no keyboard interrupt has occurred, on keyboard interrupt is set to 1.
    const {uuid, interrupt_buffer} = e.data;
    // Store data into message lookup. This allows us to use the FFI to convert the arguments.
    messageLookup[uuid] = e.data;
    
    // I was unable to access the data in the SharedArrayBuffer directly accross the pyodide FFI.
    // Best solution I came up with was to pass a wrapper function that indexes the SAB in js
    if(interrupt_buffer){
        e.data.interrupt_buffer = function(){
            return interrupt_buffer[0]; 
            // I think this Atomics call didn't work for some reason -- for one thing Atomics are limited to
            // Int32Buffers for some reason, though that isn't a big deal. Of course it isn't critical that 
            // keyboard interrupts are processed as soon as possible, and the nonatomic read should be sufficient.
            // return Atomics.load(interrupt_buffer, 0); 
        }
    }
    // executor.handle_message will look up e.data in messageLookup using uuid.
    try {
        await self.pyodide.runPythonAsync(`executor.handle_message("${uuid}")`);
    } finally {
        // pyo
    }
});

let responses = {};
function handleQueryResponse(e){
    responses[e.data.uuid].resolve(e.data);
}

function getResponsePromise(){
    let subuuid = uuid4();
    return [subuuid, new Promise((resolve, reject) => 
        responses[subuuid] = { resolve, reject }
    )];
}


async function filePicker(type){
    let [uuid, promise] = getResponsePromise();
    self.postMessage({cmd : "file_picker", uuid, type });
    let response = await promise;
    if(response.handle){
        return response.handle;
    } else {
        throw Error(response.error);
    }
}
self.filePicker = filePicker;

async function requestHandlePermission(handle, mode){
    let [uuid, promise] = getResponsePromise();
    self.postMessage({cmd : "request_handle_permission", handle, mode, uuid});
    let response = await promise;
    return response.status;
}
self.requestHandlePermission = requestHandlePermission;





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
