export class BadMessageError extends TypeError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}

export class UnknownCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}

export class InvalidCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}


export class UnknownDisplayCommandError extends UnknownCommandError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}

export class SocketListener {
    constructor(websocket) {
        this.websocket = websocket;
        this.websocket.onmessage = this.onmessage.bind(this);
        this.websocket.onopen = this.onopen.bind(this);
        this.message_dispatch = {};
        this.debug_mode = false;
    }

    add_message_handler(cmd_filter, handler) {
        this.message_dispatch[cmd_filter] = handler;
    }


    add_message_handlers_from_object(handlers) {
        for(let [cmd_filter, handler] of Object.entries(handlers)) {
            this.add_message_handler(cmd_filter, handler.bind(this));
        }
    }    

    start() {
        console.log("client ready");
        this.client_ready = true;
        if(this.socket_ready) {
            this._start();
        }
    }

    onopen(event) {
        console.log("socket opened");
        this.socket_ready = true;
        if(this.client_ready){
            this._start();
        }
    }

    _start(){
        console.error("send_introduction_message");
        this.handle_message({
            "cmd" : ["start"],
            "args" : [],
            "kwargs" : {},
        }, false);
    }

    onmessage(event) {
        let msg = JSON.parse(event.data);
        this.handle_message(msg, true);
    }

    send(cmd, kwargs) { // args parameter?
        let args = [];
        console.log("send message", cmd, args, kwargs);
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
        let obj = { "cmd" : cmd, "args" : args, "kwargs" : kwargs };
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
    
            if(msg.cmd.constructor != Array){
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
            console.error(error);
            if(report_error_to_server){
                this.report_error_to_server(error, msg);
            }
        }
    }    
}