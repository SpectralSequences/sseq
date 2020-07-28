self.languagePluginUrl = '/pyodide-build-custom/'
importScripts(`${self.languagePluginUrl}pyodide.js`);

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


function sendMessage(x){
    postMessage(x);
}
self.sendMessage = sendMessage;
self.message_lookup = {};
self.debug_parso_code_lookup = {};

self.asyncCall = function asyncCall(cmd, message){
    Object.assign(message, {cmd});
    self.asyncWorker.postMessage(message);

    // Make an i32 view because Atomics.wait only works on an i32 array.
    let i32_view = new Int32Array(responseBuffer);
    Atomics.wait(i32_view, 0, 0);
    // we stored the byteLength as an i32 as the first four bytes
    let byteLength = Atomics.load(i32_view, 0);
    if(byteLength === 0){
        // No response
        return;
    }
    // The rest is the string, now we know how long it is.
    let u8_view = new Uint8Array(responseBuffer).subarray(4, byteLength + 4);
    // text_decoder doesn't work on SharedArrayBuffer (though the standard says it should as of October 2019,
    // see https://github.com/whatwg/encoding/commit/4716397e04d4f2f9293fb601be8626bbc9e8239c)
    // Instead we copy into a temporary array and decode that.
    let copiedU8Array = new Uint8Array(byteLength);
    copiedU8Array.set(u8_view);
    let result_json = text_decoder.decode(copiedU8Array);
    let result = JSON.parse(result_json);
    return result;
}

async function startup(){
    try {
        await languagePluginLoader;
        await pyodide.loadPackage([
            "pygments", 
            "crappy-python-multitasking",
            "spectralsequence_chart"
        ]);
        await pyodide.runPython(`
            import sys
            sys.path.append("/executor")
            from executor import PyodideExecutor
            executor = PyodideExecutor()
        `);
        self.sendMessage({cmd : "ready"});
    } catch(e){
        self.sendMessage({cmd : "ready", exception : e});
    }
}
let startup_promise = startup();

let text_decoder = new TextDecoder();
self.addEventListener("message", async function(e) { // eslint-disable-line no-unused-vars
    if(e.data.cmd === "service_worker_channel"){
        let {port, responseBuffer} = e.data;
        self.asyncWorker = port;
        self.responseBuffer = responseBuffer;
        port.start();
        return;
    }
    
    await startup_promise;
    // console.log("worker received message", e.data);
    const {uuid, interrupt_buffer} = e.data;
    // delete e.data.interrupt_buffer;
    message_lookup[uuid] = e.data;
    
    if(interrupt_buffer){
        e.data.interrupt_buffer = function(){
            return interrupt_buffer[0]; 
            // return Atomics.load(interrupt_buffer, 0);
        }
    }
    await self.pyodide.runPythonAsync(`executor.handle_message("${uuid}")`);
});
