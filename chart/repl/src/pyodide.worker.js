// This pyodide worker starts the pyodide runtime on a worker thread.
// It talks to pythonExecutor, which is responsible for wrapping communication between the main thread and the pyodide thread.
//
import { sleep } from './utils';
import * as Synclink from 'synclink';
import {addNativeFS} from "./nativefs_pyodide_thread";
self.Synclink = Synclink;

const pyodideBaseURL = 'https://cdn.jsdelivr.net/pyodide/v0.19.0/full/';
importScripts(pyodideBaseURL + 'pyodide.js');

self.sleep = sleep;


self.releaseSynclinkProxy = function (proxy) {
    proxy[Synclink.releaseProxy]();
};

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

self.mountNative = function(path){
    const handle = self.openNativeDirectory();
    try {
        pyodide.FS.mount(pyodide.FS.filesystems.NATIVEFS, { handle }, path)
    } catch(e){
        console.warn(e);
        throw e;
    }
}

async function startup(main_thread_interface, mainNativeFSHelpers) {
    self.main_thread_interface = main_thread_interface;
    self.loadingMessage = async (msg) =>  await main_thread_interface.loadingMessage(msg);
    self.loadingError = async (msg) =>  await main_thread_interface.loadingError(msg);

    loadingMessage('Loading Pyodide packages');
    let jedi_promise = pyodide_promise.then(() => pyodide.loadPackage('jedi'));
    await Promise.all([chart_wheel_promise, python_tar_promise, jedi_promise]);
    addNativeFS(pyodide, mainNativeFSHelpers);
    self.openNativeDirectory = () => {
        let result = mainNativeFSHelpers.openWorkingDirectory().syncify();
        if(result){
            return result;
        }
        result = mainNativeFSHelpers.setWorkingDirectory().syncify();
        if(result){
            return result;
        }
        throw new Error("Didn't get a directory");
    };

    pyodide.runPython('import importlib; importlib.invalidate_caches()');
    loadingMessage('Initializing Python Executor');
    pyodide.runPython(`
        from initialize_pyodide import *
    `);
    self.completer_mod = pyodide.pyimport('repl.completer');
    self.execution_mod = pyodide.pyimport('repl.execution');
    self.sseq_display_mod = pyodide.pyimport('sseq_display');
    self.namespace = pyodide.globals.get('namespace');
    pyodide.registerComlink(Synclink);
}

function releaseSynclinkProxy(px) {
    px[Synclink.releaseProxy]();
}
self.releaseSynclinkProxy = releaseSynclinkProxy;

self.subscribers = [];

async function new_executor(code, stdout, stderr, interrupt_buffer) {
    return Synclink.proxy(
        self.execution_mod.Execution.callKwargs(self.namespace, code, {
            stdout,
            stderr,
            interrupt_buffer,
        }),
    );
}

async function new_completer() {
    return Synclink.proxy(completer_mod.Completer(self.namespace));
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

const service_worker_interface = {
    async connect_chart(chart_name, source_id, port) {
        const ui = Synclink.wrap(port);
        const toExpose = ['initializeSseq', 'reset', 'appplyMessages'];
        const ui_wrap = {};
        for (const func of toExpose) {
            ui_wrap[func] = async (...args) => {
                console.log(func, args);
                return await ui[func](...args);
            };
        }
        await self.sseq_display_mod.SseqDisplay.subscribe_ui(
            chart_name,
            source_id,
            ui_wrap,
        );
    },
};

function registerServiceWorkerPort(port) {
    Synclink.expose(service_worker_interface, port);
}

const repl_interface = {
    startup,
    new_completer,
    new_executor,
    registerServiceWorkerPort,
    // service_worker_channel: registerServiceWorkerPort,
    // respondToQuery: handleQueryResponse,
    // subscribe_chart_display: handleSubscribeChartDisplay,
    // handle_message,
};

Synclink.expose(repl_interface);
