import { v4 as uuid4 } from "uuid";
self.languagePluginUrl = '/pyodide-build-custom/'
importScripts(`${self.languagePluginUrl}pyodide.js`)

/**  
 * NOTE: When pyodide is finished initializing, the original "pyodide" object
 * is stored as "pyodide._module". We don't want to wait for this to happen or worry 
 * about when the move occurs, so we just store it and use that.
 */
let pyodide_module = pyodide;

import handler_decorator from "./handler_decorator.py";
import traceback from "./traceback.py";
import executor from "./executor.py";
let files_to_install = {
    executor, handler_decorator, traceback,
    __init__ : `from .executor import PyodideExecutor`
};

pyodide_module.FS.mkdir('/executor');
pyodide_module.FS.mkdir('/executor/executor');
for(let [k, v] of Object.entries(files_to_install)){
    pyodide_module.FS.writeFile(`/executor/executor/${k}.py`, v);
}



function sendMessage(x){
    postMessage(x);
}
self.sendMessage = sendMessage;
self.message_lookup = {};
self.debug_parso_code_lookup = {};



async function startup(){
    await languagePluginLoader;
    await pyodide.loadPackage(["micropip", "pygments"]);
    await pyodide.runPython(`
        import sys
        sys.path.append("/executor")
        import pathlib
        from executor import PyodideExecutor
        executor = PyodideExecutor()
    `)
}
let startup_promise = startup();

self.addEventListener("message", async function(e) { // eslint-disable-line no-unused-vars
    await startup_promise;
    console.log("worker received message", e.data);
    const {uuid, interrupt_buffer} = e.data;
    // delete e.data.interrupt_buffer;
    message_lookup[uuid] = e.data;
    
    if(interrupt_buffer){
        e.data.interrupt_buffer = function(){
            return Atomics.load(interrupt_buffer, 0);
        }
    }
    await self.pyodide.runPythonAsync(`executor.handle_message("${uuid}")`);
});