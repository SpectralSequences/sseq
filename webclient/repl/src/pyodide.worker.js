self.languagePluginUrl = '/pyodide-build-0.15.0/'
importScripts('/pyodide-build-0.15.0/pyodide.js')
import executor_text from "./executor.py";

async function startup(){
    await languagePluginLoader;
    await Promise.all([
        async function(){
            // await pyodide.loadPackage("micropip");
            await self.pyodide.runPythonAsync(`
                # import micropip
                # micropip.install("spectralsequence_chart")
                FLAGS = 0
                NAMESPACE = {}
            `)
        }(),
        self.pyodide.runPythonAsync(executor_text)
    ]);
}

let startup_promise = startup();

self.addEventListener("message", async function(e) { // eslint-disable-line no-unused-vars
    await startup_promise;
    const data = e.data;
    const cmd = data.cmd;
    if(!MessageHandlers[cmd]){
        self.postMessage({error : `Unknown command ${cmd}`, errorType : "unknown-command", id});
    }
    MessageHandlers[cmd](data);
});

class MessageHandlers {
    static async execute(data){
        const id = data.id;
        try {
            // In order to quote our input string in the most robust possible way,
            // we insert it as a global variable and then read it from globals().
            pyodide.globals[id] = data.python; 
            let [result, result_repr] = await self.pyodide.runPythonAsync(`
                result = eval_code(globals()["${id}"], NAMESPACE, flags=FLAGS)
                globals().pop("${id}")
                result
            `);
            self.postMessage({result_repr, id});
        } catch(err) {
            console.log(err);
            self.postMessage({error : err.message, id});
        }
    }

    static async validate(data){
        const id = data.id;
        try {
            // In order to quote our input string in the most robust possible way,
            // we insert it as a global variable and then read it from globals().
            pyodide.globals[id] = data.python; 
            let error = await self.pyodide.runPythonAsync(`
                result = validate_code(globals()["${id}"], flags=FLAGS)
                globals().pop("${id}")
                result
            `);
            if(error){
                error = {type : error.__class__.__name__, msg : error.msg, lineno : error.lineno, offset : error.offset};
            }
            // console.log(error);
            self.postMessage({validated : !error, error, id});
        } catch(err) {
            console.error(err);
            self.postMessage({error : err.message, id});
        }
    }
}