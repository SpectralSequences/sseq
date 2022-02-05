// This pyodide worker starts the pyodide runtime on a worker thread.
// It talks to pythonExecutor, which is responsible for wrapping communication between the main thread and the pyodide thread.
//
import { v4 as uuid4 } from 'uuid';
import { sleep } from './utils';
import { IndexedDBStorage } from './indexedDB';
import * as Comlink from 'comlink';
self.Comlink = Comlink;

const pyodideBaseURL = 'https://cdn.jsdelivr.net/pyodide/v0.19.0/full/';
importScripts(pyodideBaseURL + 'pyodide.js');

self.sleep = sleep;
self.fetch = fetch.bind(self);

async function is_promise(obj) {
    return obj && typeof obj.then == 'function';
}
self.is_promise = is_promise;

self.store = new IndexedDBStorage('pyodide-config', 2);

self.releaseComlinkProxy = releaseComlinkProxy(proxy){
    proxy[Comlink.releaseProxy]();
}

async function setWorkingDirectory(directoryHandle) {
    await self.store.open();
    await self.store
        .writeTransaction()
        .setItem('working_directory', directoryHandle);
}
self.setWorkingDirectory = setWorkingDirectory;

async function getWorkingDirectory() {
    await self.store.open();
    let result = await self.store
        .readTransaction()
        .getItem('working_directory');
    if (!result) {
        return;
    }
    let permission = await requestHandlePermission(result, 'readwrite');
    if (permission === 'granted') {
        return result;
    }
}
self.getWorkingDirectory = getWorkingDirectory;

let outBuffer = [];
let lastStreamFunc = undefined;
function makeOutputStream(streamFunc) {
    function writeToStream(charCode) {
        if (lastStreamFunc && lastStreamFunc !== streamFunc) {
            lastStreamFunc(outBuffer.join(''));
            outBuffer = [];
            lastStreamFunc = undefined;
        }
        if (charCode === 10 || !charCode) {
            streamFunc(outBuffer.join(''));
            outBuffer = [];
            lastStreamFunc = undefined;
        } else {
            lastStreamFunc = streamFunc;
            outBuffer.push(String.fromCharCode(charCode));
        }
    }
    return writeToStream;
}

let path = self.location.href;
path = path.substring(0, path.lastIndexOf('/'));

const pyodide_promise = loadPyodide({ indexURL: pyodideBaseURL }).then(
    pyodide => (self.pyodide = pyodide),
);

async function fetch_and_unpack(url) {
    const fetch_promise = fetch(url).then(resp => resp.arrayBuffer());
    await pyodide_promise;
    const buffer = await fetch_promise;
    pyodide.unpackArchive(buffer, url.substring(url.lastIndexOf('.') + 1));
}

const chart_wheel_promise = fetch_and_unpack(
    `${path}/spectralsequence_chart-0.0.28-py3-none-any.whl`,
);
const python_tar_promise = fetch_and_unpack('python.tar');
async function startup(loadingMessage, loadingError) {
    self.loadingMessage = loadingMessage;
    self.loadingError = loadingError;

    loadingMessage('Loading Pyodide packages');
    let jedi_promise = pyodide_promise.then(() => pyodide.loadPackage('jedi'));
    await Promise.all([chart_wheel_promise, python_tar_promise, jedi_promise]);
    pyodide.runPython('import importlib; importlib.invalidate_caches()');
    loadingMessage('Initializing Python Executor');
    pyodide.runPython(`
        from initialize_pyodide import *
    `);
    self.completer_mod = pyodide.pyimport('repl.completer');
    self.execution_mod = pyodide.pyimport('repl.execution');
    self.namespace = pyodide.globals.get('namespace');
    loadingMessage[Comlink.releaseProxy]();
    loadingError[Comlink.releaseProxy]();
    pyodide.registerComlink(Comlink);
}

self.subscribers = [];

async function new_executor(code, stdout, stderr, interrupt_buffer) {
    return Comlink.proxy(
        self.execution_mod.Execution.callKwargs(self.namespace, code, {
            stdout,
            stderr,
            interrupt_buffer,
        }),
    );
}

async function new_completer() {
    return Comlink.proxy(completer_mod.Completer(self.namespace));
}

async function handleSubscribeChartDisplay(e) {
    let uuid = e.data;
    messageLookup.set(uuid, e.data);
    await self.pyodide.runPythonAsync(`
        from js_wrappers.messages import get_message
        msg = get_message("${uuid}")
        display = SseqDisplay.displays[msg["chart_name"]]
        await display.add_subscriber(msg["uuid"], msg["port"])
    `);
}

async function filePicker(type) {
    let [uuid, promise] = getResponsePromise();
    self.postMessage({ cmd: 'file_picker', uuid, type });
    let response = await promise;
    if (response.handle) {
        return response.handle;
    } else {
        throw Error(response.error);
    }
}
self.filePicker = filePicker;

async function requestHandlePermission(handle, mode) {
    let [uuid, promise] = getResponsePromise();
    self.postMessage({ cmd: 'request_handle_permission', handle, mode, uuid });
    let response = await promise;
    return response.status;
}
self.requestHandlePermission = requestHandlePermission;

function registerServiceWorkerPort(e) {
    console.log('registerServiceWorkerPort');
    let { port, repl_id } = e.data;
    self.serviceWorker = port;
    self.serviceWorker.addEventListener(
        'message',
        handleMessageFromServiceWorker,
    );
    port.start();
    port.postMessage({ cmd: 'ready', repl_id });
}

function handleMessageFromServiceWorker(event) {
    if (event.data.cmd === 'subscribe_chart_display') {
        registerNewSubscriber(event);
        return;
    }
    console.error(`Unknown command: ${event.data.cmd}`, event.data, event);
    throw Error(`Unknown command: ${event.data.cmd}`);
}

function registerNewSubscriber(event) {
    let { port, chart_name, uuid, client_id } = event.data;
    console.log(`New subscriber to ${chart_name}`, event.data);
    port.addEventListener('message', e =>
        handleMessageFromChart(e, port, chart_name, client_id),
    );
    port.start();
    self.subscribers.push(port);
}

async function handleMessageFromChart(event, port, chart_name, client_id) {
    let message = event.data;
    console.log('handleMessageFromChart', message);
    let { uuid } = JSON.parse(message);
    messageLookup.set(uuid, { message, chart_name, port, client_id });
    console.log('message from chart:', message);
    await pyodide.runPythonAsync(
        `await SseqDisplay.dispatch_message(get_message("${uuid}"))`,
    );
}

Comlink.expose({
    startup,
    new_completer,
    new_executor,
    // service_worker_channel: registerServiceWorkerPort,
    // respondToQuery: handleQueryResponse,
    // subscribe_chart_display: handleSubscribeChartDisplay,
    // handle_message,
});
