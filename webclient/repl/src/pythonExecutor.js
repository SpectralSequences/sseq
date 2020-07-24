// import Worker from './pyodide.worker.js';
import { v4 as uuid4 } from "uuid";
import { sleep } from "./utils";
import { EventEmitter } from "eventemitter3";

export class PythonExecutor {
    constructor(){
        this.executions = {};
        this.completers = {};
        this.worker = new Worker("worker.bundle.js");
        this.worker.addEventListener("message", this._handleMessage.bind(this));
    }

    _handleMessage(event){
        let message = event.data;
        let message_cmd = message.cmd;
        let subhandler_name = ({"execute" : "_handleExecutionMessage", "complete" : "_handleCompletionMessage"})[message_cmd];
        // console.log("message", message, "subhandler_name", subhandler_name);
        if(!subhandler_name){
            throw new Error(`Unknown command "${message_cmd}"`);
        }
        this[subhandler_name](message);
    }

    _handleExecutionMessage(message){
        const { uuid, subcmd, last_response } = message;
        const execution = this.executions[uuid];
        if(!execution){
            throw new Error(`Invalid execution uuid "${uuid}"`);
        }
        if(execution.listenerCount(subcmd) === 0) {
            throw new Error(`Unexpected command "${subcmd}"`);
        }
        execution.emit(subcmd, message);
        if(last_response){
            execution._close();
            delete this.executions[uuid];
        }
    }

    _handleCompletionMessage(message){
        const { uuid, subcmd } = message;
        const completer = this.completers[uuid];
        if(!completer){
            throw new Error(`Invalid completer uuid "${uuid}"`);
        }
        if(completer.listenerCount(subcmd) === 0) {
            throw new Error(`Unexpected command "${subcmd}"`);
        }
        completer.emit(subcmd, message);
    }

    _postMessage(cmd, uuid, msg){
        Object.assign(msg, {cmd, uuid});
        this.worker.postMessage(msg);
    }


    execute(code){
        const interrupt_buffer = new Int32Array(new SharedArrayBuffer(4));
        const uuid = uuid4();
        const execution = new Execution(interrupt_buffer);
        this.executions[uuid] = execution;
        this._postMessage("execute", uuid, {code, interrupt_buffer});
        return execution;
    }

    new_completer(){
        const uuid = uuid4();
        const completer = new Completer(this, uuid);
        this.completers[uuid] = completer;
        this._postMessage("complete", uuid, {subcmd : "new_completer"});
        return completer;
    }

}

export class Execution extends EventEmitter {
    constructor(interrupt_buffer){
        super();
        this.interrupt_buffer = interrupt_buffer;
        this._validate_syntax = new Promise((resolve, reject) => {
            this.once("validate_syntax", resolve);
        });
        this._result = new Promise((resolve, reject) => {
            this.once("result", (message) => resolve(message.result));
            this.once("exception", (message) => reject(message));
        });
    }
    
    async validate_syntax(){
        return await this._validate_syntax;
    }

    async result(){
        return await this._result;
    }

    setInterrupt(i){
        this.interrupt_buffer[0] = i;
    }

    keyboardInterrupt(){
        this.setInterrupt(2); // SIGINT
    }

    onStdout(handler, context){
        this.on("stdout", function(message) { 
            handler.call(this, message.data); 
        }, context);
    }

    ignoreStdout(){
        this.on("stdout", () => undefined);
    }

    onStderr(handler, context){
        this.on("stderr", function(message) { 
            handler.call(this, message.data); 
        }, context);
    }

    ignoreStderr(){
        this.on("stderr", () => undefined);
    }

    _close(){
        
    }    
}

export class Completer extends EventEmitter {
    constructor(executor, uuid){
        super();
        this.executor = executor;
        this.uuid = uuid;
        this.responses = {};
        this.on("completions", (msg) => {
            this.responses[msg.subuuid].resolve(msg);
        });
    }

    _postMessage(subcmd, msg){
        Object.assign(msg, {subcmd});
        this.executor._postMessage("complete", this.uuid, msg);
    }

    setCode(code){
        this._postMessage("set-code", {code});
    }

    setPosition(line, column){
        this.line = line;
        this.column = column;
    }

    async getCompletions(){
        let subuuid = uuid4();
        let response_promise = new Promise((resolve, reject) => 
            this.responses[subuuid] = {resolve, reject}
        );
        this._postMessage("completions", { subuuid });
        let response = await response_promise;
        return response.completions;
    }

    close(){
        delete this.executor.completers[this.uuid];
        delete this.executor;
    }

}