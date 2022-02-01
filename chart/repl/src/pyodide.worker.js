// This pyodide worker starts the pyodide runtime on a worker thread.
// It talks to pythonExecutor, which is responsible for wrapping communication between the main thread and the pyodide thread.
// 
import { v4 as uuid4 } from "uuid";
import { sleep } from "./utils";
import { IndexedDBStorage } from "./indexedDB";

self.loaded = false;

const pyodideBaseURL = 'https://cdn.jsdelivr.net/pyodide/v0.19.0/full/'
importScripts(pyodideBaseURL + "pyodide.js");

self.sleep = sleep;
self.fetch = fetch.bind(self);

async function is_promise(obj){
    return obj && typeof obj.then == 'function';
}
self.is_promise = is_promise;

self.store = new IndexedDBStorage("pyodide-config", 2);

async function setWorkingDirectory(directoryHandle){
    await self.store.open();
    await self.store.writeTransaction().setItem("working_directory", directoryHandle);
}
self.setWorkingDirectory = setWorkingDirectory;

async function getWorkingDirectory(){
    await self.store.open();
    let result = await self.store.readTransaction().getItem("working_directory");
    if(!result){
        return;
    }
    let permission = await requestHandlePermission(result, "readwrite");
    if(permission === "granted"){
        return result
    }    
}
self.getWorkingDirectory = getWorkingDirectory;



function loadingMessage(text){
    postMessage({ cmd : "loadingMessage", text});
}

function loadingError(text){
    postMessage({ cmd : "loadingError", text});
}

// files_to_install is a map file_name => file_contents for the files in the python directory. 
// It's produced by the webpack prebuild script scripts/bundle_python_sources.py

let outBuffer = [];
let lastStreamFunc = undefined;
function makeOutputStream(streamFunc){
    function writeToStream(charCode){
        if(lastStreamFunc && lastStreamFunc !== streamFunc){
            lastStreamFunc(outBuffer.join(""));
            outBuffer = [];
            lastStreamFunc = undefined;
        }
        if(charCode === 10 || !charCode){
            streamFunc(outBuffer.join(""));
            outBuffer = [];
            lastStreamFunc = undefined;
        } else {
            lastStreamFunc = streamFunc;
            outBuffer.push(String.fromCharCode(charCode));
        }
    }
    return writeToStream;
}

// See scripts/bundle_python_sources.py

function sendMessage(message){
    self.postMessage(message);
}
self.sendMessage = sendMessage;
self.messageLookup = new Map();


let path = self.location.href;
path = path.substring(0, path.lastIndexOf("/"))

const pyodide_promise = loadPyodide({indexURL : pyodideBaseURL}).then((pyodide) => self.pyodide = pyodide);

async function fetch_and_unpack(url){
    const fetch_promise = fetch(url).then(resp => resp.arrayBuffer());
    await pyodide_promise;
    const buffer = await fetch_promise;
    pyodide.unpackArchive(buffer, url.substring(url.lastIndexOf(".") + 1));
}

const chart_wheel_promise = fetch_and_unpack(`${path}/spectralsequence_chart-0.0.28-py3-none-any.whl`)
const python_tar_promise = fetch_and_unpack("python.tar");
self.loadingMessage = loadingMessage;
async function startup(){
    try {
        loadingMessage("Loading Pyodide packages");
        let jedi_promise = pyodide_promise.then(() => pyodide.loadPackage("jedi"));
        await Promise.all([chart_wheel_promise, python_tar_promise, jedi_promise]);
        pyodide.runPython("import importlib; importlib.invalidate_caches()");
        loadingMessage("Initializing Python Executor");
        pyodide.runPython(`
            from initialize_pyodide import *
        `);
        self.loaded = true;
        self.postMessage({cmd : "ready"});
    } catch(e){
        self.postMessage({cmd : "ready", exception : e});
    }
}
let startup_promise = startup();

self.subscribers = [];

let handledCommands = {
    service_worker_channel : registerServiceWorkerPort,
    respondToQuery : handleQueryResponse,
    subscribe_chart_display : handleSubscribeChartDisplay,
}

self.addEventListener("message", async function(e) {
    if(handledCommands[e.data.cmd]){
        await handledCommands[e.data.cmd](e);
        return;
    }
    await startup_promise;
    // interrupt_buffer is a single byte SharedArrayBuffer used to signal a keyboard interrupt.
    // If it contains 0, no keyboard interrupt has occurred, on keyboard interrupt is set to 1.
    const {uuid, interrupt_buffer} = e.data;

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
    } else {
        e.data.interrupt_buffer = () => 0;
    }

    // Store data into message lookup. This allows us to use the FFI to convert the arguments.
    messageLookup.set(uuid, e.data);
    // get_message looks up e.data in messageLookup using uuid.
    try {
        await self.pyodide.globals.get("handle_message")(uuid);
    } finally {
        // pyo
    }
});


async function handleSubscribeChartDisplay(e){
    let uuid = e.data;
    messageLookup.set(uuid, e.data); 
    await self.pyodide.runPythonAsync(`
        from js_wrappers.messages import get_message
        msg = get_message("${uuid}")
        display = SseqDisplay.displays[msg["chart_name"]]
        await display.add_subscriber(msg["uuid"], msg["port"])
    `);
}

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
    console.log("registerServiceWorkerPort");
    let { port, repl_id } = e.data;
    self.serviceWorker = port;
    self.serviceWorker.addEventListener("message", handleMessageFromServiceWorker)
    port.start();
    port.postMessage({ cmd : "ready", repl_id });
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

async function handleMessageFromChart(event, port, chart_name, client_id){
    let message = event.data;
    console.log("handleMessageFromChart", message);
    let { uuid } = JSON.parse(message);
    messageLookup.set(uuid, { message, chart_name, port, client_id });
    console.log("message from chart:", message);
    await pyodide.runPythonAsync(`await SseqDisplay.dispatch_message(get_message("${uuid}"))`);
}
