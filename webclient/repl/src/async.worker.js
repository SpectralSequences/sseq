import { sleep } from "./utils";

self.addEventListener("message", async function(e) { // eslint-disable-line no-unused-vars
    if(e.data.cmd === "pyodide_worker_channel"){
        let { port, responseBuffer } = e.data;
        self.pyodideWorker = port;
        self.responseBuffer = new Int32Array(responseBuffer);
        self.pyodideWorker.addEventListener("message", handlePyodideMessage); 
        self.pyodideWorker.start();
        return;
    }
    throw Error("Unknown command.");
});


const encoder = new TextEncoder();
async function handlePyodideMessage(event){
    let message = event.data;
    if(!message.cmd){
        throw Error("Undefined command")
    }
    if(!(message.cmd in messageDispatch)){
        throw Error(`Unknown command ${msg.cmd}`);
    }
    let response = await messageDispatch[event.data.cmd](event.data);
    if(!response){
        responseBuffer[0] = 0;
        Atomics.notify(responseBuffer, 0);
        return;
    }
    let response_json = JSON.stringify(response);
    // while(self.responseBuffer[0] !== 0){
    //     Atomics.wait(responseBuffer, 0, self.responseBuffer[0]);
    // }
    // encodeInto still doesn't work on SharedArrayBuffer (though the standard says it should as of October 2019,
    // see https://github.com/whatwg/encoding/commit/4716397e04d4f2f9293fb601be8626bbc9e8239c)
    // So we have to encode into a new Uint8Array and then copy.
    // That there is no str.bytesLength, so to size our temporary buffer we note that a UTF8 character is at most 3 bytes.
    // We will need to reinterpret the ArrayBuffer as a Int32Array in a minute so we round up the size to a multiple of 4.
    let buffer = new ArrayBuffer(Math.ceil(3*response_json.length/4)*4);
    let { written } = encoder.encodeInto(response_json, new Uint8Array(buffer));
    if(written/4 >= responseBuffer.length){
        throw Error(`Response buffer too small. Reponse is ${response_json.length + 1} bytes but responseBuffer only fits ${responseBuffer.length} bytes.`);
    }
    responseBuffer.set(new Int32Array(buffer), 1);
    responseBuffer[0] = written;
    // Atomics.notify only works on a I32Array (for some reason?)
    Atomics.notify(responseBuffer, 0);
}


let messageDispatch = {
    "chart.status" : async (message) => {
        return {cmd : "ping", data : "A response" };
    },
    "sleep" : async (message) => {
        await sleep(message.duration);
    }
}