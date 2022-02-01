// import Worker from './pyodide.worker.js';
import { v4 as uuid4 } from 'uuid';
import { sleep } from './utils';
import { EventEmitter } from 'eventemitter3';

function createInterruptBuffer() {
    if (window.SharedArrayBuffer) {
        return new Int32Array(new SharedArrayBuffer(4));
    } else {
        return new Int32Array(new ArrayBuffer(4));
    }
}

export class PythonExecutor {
    constructor() {
        this.executions = {};
        this.completers = {};
        window.loadingWidget.addLoadingMessage('Loading Pyodide');
        this.pyodide_worker = new Worker('pyodide_worker.bundle.js');
        this.pyodide_worker.addEventListener(
            'message',
            this._handlePyodideMessage.bind(this),
        );
        // The pyodide worker needs to be able to send messages to the service worker, so we make a channel
        // and send one end to the service worker and the other to the pyodide worker.
        navigator.serviceWorker.addEventListener(
            'message',
            this._handleServiceWorkerMessage.bind(this),
        );

        let _readyPromise = new Promise(
            (resolve, reject) => (this._readyPromise = { resolve, reject }),
        );
        this._readyPromise.promise = _readyPromise;
    }

    _postMessage(cmd, uuid, msg) {
        Object.assign(msg, { cmd, uuid });
        this.pyodide_worker.postMessage(msg);
    }

    _handleServiceWorkerMessage(event) {
        if (event.data.cmd !== 'connect_to_pyodide') {
            throw Error('Unexpected command from service worker!');
        }
        this._handleServiceWorkerConnection(event);
    }

    _handleServiceWorkerConnection(event) {
        console.log('handle service worker connection');
        let msg = event.data;
        let { port, repl_id } = msg;
        this.pyodide_worker.postMessage(
            {
                cmd: 'service_worker_channel',
                port,
                repl_id,
            },
            [port],
        );
    }

    _handlePyodideMessage(event) {
        let message = event.data;
        let message_cmd = message.cmd;
        let subhandlers = {
            execute: '_handleExecutionMessage',
            complete: '_handleCompletionMessage',
            ready: '_handleReadyMessage',
            file_picker: 'file_picker',
            request_handle_permission: '_handleRequestHandlePermission',
            loadingMessage: '_handleLoadingMessage',
            loadingError: '_handleLoadingError',
        };
        let subhandler_name = subhandlers[message_cmd];
        if (!subhandler_name) {
            throw new Error(`Unknown command "${message_cmd}"`);
        }
        this[subhandler_name](message);
    }

    async file_picker(message) {
        let pickerFunction = {
            directory: showDirectoryPicker,
            read: showOpenFilePicker,
            readwrite: showSaveFilePicker,
        }[message.type];
        try {
            let handle = await pickerFunction();
            if (message.type !== 'read') {
                // In case "read", it returns a list.
                // In remaining cases, it returns a single handle.
                // Allow more consistent handling by always giving a list.
                handle = [handle];
            }
            this._postMessage('respondToQuery', message.uuid, { handle });
        } catch (error) {
            this._postMessage('respondToQuery', message.uuid, { error });
        }
    }

    _handleLoadingMessage(message) {
        window.loadingWidget.addLoadingMessage(message.text);
    }

    _handleLoadingError(message) {
        console.error(message);
        throw Error('TODO: handle me!');
    }

    async _handleRequestHandlePermission(message) {
        let { uuid, handle, mode } = message;
        let status = await handle.requestPermission({ mode });
        this._postMessage({
            cmd: 'respondToQuery',
            uuid,
            status,
        });
    }

    _handleReadyMessage(message) {
        if (message.exception) {
            this._readyPromise.reject(message.exception);
            return;
        }
        window.loadingWidget.addLoadingMessage('Pyodide is ready!');
        this._readyPromise.resolve();
    }

    _handleExecutionMessage(message) {
        // execution messages get emitted on the execution object.
        const { uuid, subcmd, last_response } = message;
        const execution = this.executions[uuid];
        if (!execution) {
            throw new Error(`Invalid execution uuid "${uuid}"`);
        }
        // Check if there is a handler for the given command, otherwise fail.
        // All messages are meant to be handled.
        if (execution.listenerCount(subcmd) === 0) {
            throw new Error(`Unexpected command "${subcmd}"`);
        }
        execution.emit(subcmd, message);
        if (last_response) {
            execution._close();
            delete this.executions[uuid];
        }
    }

    _handleCompletionMessage(message) {
        const { uuid, subcmd } = message;
        const completer = this.completers[uuid];
        if (!completer) {
            throw new Error(`Invalid completer uuid "${uuid}"`);
        }
        if (completer.listenerCount(subcmd) === 0) {
            throw new Error(`Unexpected command "${subcmd}"`);
        }
        completer.emit(subcmd, message);
    }

    async ready() {
        return await this._readyPromise.promise;
    }

    execute(code) {
        const interrupt_buffer = createInterruptBuffer();
        const uuid = uuid4();
        const execution = new Execution(interrupt_buffer);
        this.executions[uuid] = execution;
        this._postMessage('execute', uuid, { code, interrupt_buffer });
        return execution;
    }

    new_completer() {
        const uuid = uuid4();
        const completer = new Completer(this, uuid);
        this.completers[uuid] = completer;
        this._postMessage('complete', uuid, { subcmd: 'new_completer' });
        return completer;
    }
}

export class Execution extends EventEmitter {
    /* An execution object. This is for attaching handlers / giving out promises for various lifecycle events of the execution.
       The execution object is created and scheduled by PythonExecutor.execute. Other files do not construct these directly.
       The Executor also dispatches messages from the pyodide worker to the appropriate execution.
       See the python file "execution.py" for when python generates the messages this is responding to.
    */
    constructor(interrupt_buffer) {
        super();
        this.interrupt_buffer = interrupt_buffer;
        // Using "once" here helps us throw a useful error if some logic error causes the pyodide worker to send
        // the same event twice.
        this._validate_syntax = new Promise((resolve, reject) => {
            this.once('validate_syntax', resolve);
        });
        this._result = new Promise((resolve, reject) => {
            this.once('result', message => resolve(message.result));
            this.once('exception', message => reject(message));
            this.once('keyboard_interrupt', message => reject(message));
        });
    }

    async validate_syntax() {
        return await this._validate_syntax;
    }

    async result() {
        return await this._result;
    }

    setInterrupt(i) {
        this.interrupt_buffer[0] = i;
        // Atomics.notify(this.interrupt_buffer, 0);
    }

    keyboardInterrupt() {
        this.setInterrupt(2); // SIGINT
    }

    onStdout(handler, context) {
        this.on(
            'stdout',
            function (message) {
                handler.call(this, message.data);
            },
            context,
        );
    }

    ignoreStdout() {
        this.on('stdout', () => undefined);
    }

    onStderr(handler, context) {
        this.on(
            'stderr',
            function (message) {
                handler.call(this, message.data);
            },
            context,
        );
    }

    ignoreStderr() {
        this.on('stderr', () => undefined);
    }

    _close() {}
}

export class Completer extends EventEmitter {
    constructor(executor, uuid) {
        super();
        this.executor = executor;
        this.uuid = uuid;
        this.responses = {};
        for (let cmd of ['signatures', 'completions', 'completion_detail']) {
            this._attachResponseHandler(cmd);
        }
    }

    _attachResponseHandler(cmd) {
        this.on(cmd, msg => {
            let promise_obj = this.responses[msg.subuuid];
            if (!promise_obj) {
                throw Error(`Unknown subuuid ${subuuid}`);
            }
            if (cmd !== promise_obj.cmd) {
                throw new Error(
                    `Wrong command for response subuuid ${subuuid}. Was expecting command to be "${cmd}" but got "${promise_obj.cmd}"`,
                );
            }
            promise_obj.resolve(msg);
        });
    }

    _getResponsePromise(cmd) {
        let subuuid = uuid4();
        return [
            subuuid,
            new Promise(
                (resolve, reject) =>
                    (this.responses[subuuid] = { resolve, reject, cmd }),
            ),
        ];
    }

    _postMessage(subcmd, msg) {
        Object.assign(msg, { subcmd });
        this.executor._postMessage('complete', this.uuid, msg);
    }

    async getSignatures(code, position, cancellation_token) {
        let [subuuid, response_promise] =
            this._getResponsePromise('signatures');
        let { lineNumber, column } = position;
        let interrupt_buffer = createInterruptBuffer();
        cancellation_token.onCancellationRequested(() => {
            interrupt_buffer[0] = 2;
        });
        this._postMessage('signatures', {
            subuuid,
            interrupt_buffer,
            code,
            lineNumber,
            column,
        });
        return await response_promise;
    }

    async getCompletions(code, position, cancellation_token) {
        let [subuuid, response_promise] =
            this._getResponsePromise('completions');
        let interrupt_buffer = createInterruptBuffer();
        cancellation_token.onCancellationRequested(() => {
            interrupt_buffer[0] = 2;
        });
        let { lineNumber, column } = position;
        this._postMessage('completions', {
            subuuid,
            interrupt_buffer,
            code,
            lineNumber,
            column,
        });
        return await response_promise;
    }

    async getCompletionInfo(state_id, idx, cancellation_token) {
        let [subuuid, response_promise] =
            this._getResponsePromise('completion_detail');
        let interrupt_buffer = createInterruptBuffer();
        cancellation_token.onCancellationRequested(() => {
            interrupt_buffer[0] = 2;
        });
        this._postMessage('completion_detail', {
            subuuid,
            interrupt_buffer,
            idx,
            state_id,
        });
        return await response_promise;
    }

    close() {
        delete this.executor.completers[this.uuid];
        delete this.executor;
    }
}
