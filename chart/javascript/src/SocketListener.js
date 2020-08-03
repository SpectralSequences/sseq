import { uuidv4 as uuid4 } from "./interface/utils.js";

function removeErrorConstructorsFromStacktrace(stacktrace){
    let lines = stacktrace.split("\n");
    for(let i = 1; i < lines.length; i++){
        if(!(/^\s*at new \w*(Error|Exception)/.test(lines[i]))){
            break;
        }
        console.log("removed:", lines[i]);
        lines[i] = undefined;
    }
    let result = [];
    lines.forEach((line) => {
        if(line){
            result.push(line);
        }
    });
    result.join("\n");
    return result;
}


function cmd_string_to_filter_list(cmd){
    let result = [cmd];
    let idx;
    while( (idx = cmd.lastIndexOf(".")) >= 0) {
        cmd = cmd.slice(0, idx);
        result.push(cmd);
    }
    result.push("*");
    return result;
}

export class BadMessageError extends TypeError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        // this.stack = removeErrorConstructorsFromStacktrace(this.stack);
    }
}

export class UnknownCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
    }
}

export class InvalidCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
    }
}

export class UnknownDisplayCommandError extends UnknownCommandError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
    }
}

export class StaleResponseError extends TypeError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = (new Error(...args)).stack;
    }
}

export class SocketListener {
    constructor(websocket) {
        this.websocket = websocket;
        this.websocket.onmessage = this.onmessage.bind(this);
        this.message_dispatch = {};
        this.promise_filters = {};
        this.promises = {};
        this.message_extra_data = {};
        this.debug_mode = false;
        if("onopen" in websocket){
            this.websocket.onopen = this.onopen.bind(this);
        } else {
            this.onopen();
        }
        if("postMessage" in websocket){
            this.websocket.send = this.websocket.postMessage;
        }
    }

    add_message_handler(cmd_filter, handler) {
        this.message_dispatch[cmd_filter] = handler;
    }

    add_message_handlers_from_object(handlers) {
        for(let [cmd_filter, handler] of Object.entries(handlers)) {
            this.add_message_handler(cmd_filter, handler.bind(this));
        }
    }    

    add_promise_message_handler(cmd_filter) {
        this.promise_filters[cmd_filter] = true;
        this.add_message_handler(cmd_filter, (cmd, args, kwargs) => {
            if(!(cmd_filter in this.promise_filters)){
                throw Error(`Unexpected promise-handled message.`);
            }
            if(kwargs.uuid === undefined){
                throw Error("Response has no uuid.");
            }
            let {resolve, uuid} = this.promises[cmd_filter];
            if(kwargs.uuid === uuid){
                resolve([cmd, args, kwargs]);
            }
        });
    }

    new_message_promise(cmd_filter){
        if(cmd_filter in this.promises){
            let { reject } = this.promises[cmd_filter];
            reject(new StaleResponseError("Stale Response"));
        }
        let result = {};        
        result.promise = new Promise((resolve, reject) => {
            result.resolve = resolve;
            result.reject = reject;
        });
        result.uuid = uuid4();
        this.promises[cmd_filter] = result;
        return {"promise" : result.promise, "uuid" : result.uuid };
    }

    get_message_promise(cmd_filter){
        if(!this.promises[cmd_filter]){
            return this.new_message_promise(cmd_filter);
        }
        let result = this.promises[cmd_filter];
        return {"promise" : result.promise, "uuid" : result.uuid };
    }

    start() {
        this.console_log_if_debug("client ready");
        this.client_ready = true;
        if(this.socket_ready) {
            this._start();
        }
    }

    onopen(event) {
        this.console_log_if_debug("socket opened");
        this.socket_ready = true;
        if(this.client_ready){
            this._start();
        }
    }

    _start(){
        if("start" in this.websocket){
            this.websocket.start();
        }        
        this.console_log_if_debug("send_introduction_message");
        this.handle_message({
            "cmd" : ["start"],
            "args" : [],
            "kwargs" : {},
        }, false);
    }

    onmessage(event) {
        let msg = event.data;
        if(msg.constructor === String){
            msg = JSON.parse(msg);
        }
        this.handle_message(msg, true);
    }

    send(cmd, kwargs) { // args parameter?
        let args = [];
        this.console_log_if_debug("send message", cmd, args, kwargs);
        if(args === undefined || kwargs === undefined) {
            throw TypeError(`Send with missing arguments.`);
        }
        if(args.constructor !== Array){
            throw TypeError(`Argument "args" expected to have type "Array" not "${args.constructor.name}"`);
        }
        if(kwargs.constructor !== Object){
            throw TypeError(`Argument "kwargs" expected to have type "Object" not "${kwargs.constructor.name}"`);
        }            
        if("cmd" in kwargs) {
            throw ValueError(`Tried to send message with top level "cmd" key`);
        }
        let uuid = uuid4();
        let obj = Object.assign({ 
                cmd, args, kwargs,
                uuid : uuid4()
            },
            this.message_extra_data
        );
        let json_str = JSON.stringify(obj);
        this.websocket.send(json_str);
    }

    console_log_if_debug(msg) {
        if(this.debug_mode) {
            console.log(msg);
        }
    }
    
    debug(type, text, orig_msg) {
        let cmd = "debug";
        if(type !== ""){
            cmd += `.${type}`
        }            
        this.send("debug", {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg
        });
    }

    info(type, text, orig_msg) {
        let cmd = "info";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg
        });
    }

    warning(type, text, orig_msg, stack_trace) {
        let cmd = "warning";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg,
            "stack_trace" : stack_trace
        });
    }

    error(type, msg) {
        let cmd = "error.client";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, msg);
    }

    report_error_to_server(error, orig_msg) {
        // For some reason JSON.stringify(error) drops the "message" field by default.
        // We move it to "msg" to avoid that.
        error.msg = error.message; 
        this.error(error.name, 
            {
                "exception" : error,
                "orig_msg" : orig_msg,
            }
        );
    }

    handle_message(msg, report_error_to_server) {
        this.console_log_if_debug(msg);
        try {
            if(msg.cmd === undefined) {
                throw new UnknownCommandError(`Server sent message missing "cmd" field.`);
            }

            if(msg.cmd.constructor === String){
                msg.cmd = cmd_string_to_filter_list(msg.cmd);
            }
    
            if(msg.cmd.constructor !== Array){
                throw new InvalidCommandError(
                    `"msg.cmd" should have type "Array" not "${msg.cmd.constructor.name}."`
                );
            }
    
            if(msg.args === undefined) {
                throw new InvalidCommandError(
                    `Message is missing the "args" field.`
                );
            }
            
            if(msg.kwargs === undefined) {
                throw new InvalidCommandError(
                    `Message is missing the "kwargs" field.`
                );
            }
    
            let key = undefined;
            for(let partial_cmd of msg.cmd) {
                if(this.message_dispatch[partial_cmd] !== undefined){
                    key = partial_cmd; 
                    break;
                }
            }
            this.console_log_if_debug("cmd", msg.cmd, "key", key);
            this.console_log_if_debug("received message","cmd", msg.cmd, "key", key);
            if(key === undefined) {
                throw new UnknownCommandError(`Server sent unknown command "${msg.cmd[0]}".`);
            }
            this.message_dispatch[key](msg.cmd, msg.args, msg.kwargs);
        } catch(error) {
            this.console_log_if_debug(error);
            console.log(msg);
            console.error(error);
            if(report_error_to_server){
                this.report_error_to_server(error, msg);
            }
        }
    }    
}