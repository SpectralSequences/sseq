// import Worker from './pyodide.worker.js';
import { v4 as uuid4 } from 'uuid';
import { EventEmitter } from 'eventemitter3';
import { nativeFSHelpers } from "./nativefs_main_thread";
import * as Synclink from 'synclink';
window.Synclink = Synclink;

function createInterruptBuffer() {
    if (window.SharedArrayBuffer) {
        return new Int32Array(new SharedArrayBuffer(4));
    } else {
        return new Int32Array(new ArrayBuffer(4));
    }
}


const main_thread_pyodide_interface = {
    loadingMessage(msg){
        window.loadingWidget.addLoadingMessage(msg);
    },
    loadingError(msg){
        console.error(msg);
    },
};

export class PythonExecutor {
    constructor() {
        this.executions = {};
        this.completers = {};
        window.loadingWidget.addLoadingMessage('Loading Pyodide');
        this._raw_pyodide_worker = new Worker('pyodide_worker.bundle.js');
        this.pyodide_worker = Synclink.wrap(
            new Worker('pyodide_worker.bundle.js'),
        );
        window.python_executor = this;
        this._ready = this.pyodide_worker
            .startup(Synclink.proxy(main_thread_pyodide_interface), Synclink.proxy(nativeFSHelpers))
            .then(() =>
                window.loadingWidget.addLoadingMessage('Pyodide is ready!'),
            );
        navigator.serviceWorker.addEventListener(
            'message',
            this._handleServiceWorkerMessage.bind(this),
        );
    }

    async _handleServiceWorkerMessage(event) {
        if (event.data.cmd !== 'connect_to_pyodide') {
            throw Error('Unexpected command from service worker!');
        }
        let msg = event.data;
        let { port } = msg;
        await this.pyodide_worker.registerServiceWorkerPort(
            Synclink.transfer(port, [port]),
        );
    }

    async ready() {
        await this._ready;
    }

    execute(code) {
        return new Execution(this.pyodide_worker, code);
    }

    async new_completer() {
        await this._ready;
        return new Completer(await this.pyodide_worker.new_completer());
    }
}

export class Execution {
    /* An execution object. This is for attaching handlers / giving out promises for various lifecycle events of the execution.
       The execution object is created and scheduled by PythonExecutor.execute. Other files do not construct these directly.
       The Executor also dispatches messages from the pyodide worker to the appropriate execution.
       See the python file "execution.py" for when python generates the messages this is responding to.
    */
    constructor(pyodide_worker, code) {
        const interrupt_buffer = createInterruptBuffer();
        const stdout = Synclink.proxy(x => {
            this._stdout(x);
        });
        const stderr = Synclink.proxy(x => {
            this._stderr(x);
        });
        this.interrupt_buffer = interrupt_buffer;
        this.proxy_promise = pyodide_worker
            .new_executor(code, stdout, stderr, interrupt_buffer)
            .then(res => (this.proxy = res));
    }

    async validate_syntax() {
        await this.proxy_promise;
        let res = await this.proxy.validate_syntax();
        if (!res.valid) {
            this.proxy[Synclink.releaseProxy]();
        }
        return res;
    }

    async result() {
        try {
            return await this.proxy.run();
        } finally {
            this.proxy[Synclink.releaseProxy]();
        }
    }

    setInterrupt(i) {
        this.interrupt_buffer[0] = i;
        // Atomics.notify(this.interrupt_buffer, 0);
    }

    keyboardInterrupt() {
        this.setInterrupt(2); // SIGINT
    }

    onStdout(handler, context) {
        this._stdout = handler;
    }

    ignoreStdout() {
        this._stdout = () => undefined;
    }

    onStderr(handler) {
        this._stderr = handler;
    }

    ignoreStderr() {
        this._stderr = () => undefined;
    }

    _close() {}
}

export class Completer {
    constructor(completer) {
        this.completer = completer;
    }

    async getCompletions(code, position, cancellation_token) {
        const interrupt_buffer = createInterruptBuffer();
        cancellation_token.onCancellationRequested(() => {
            interrupt_buffer[0] = 2;
        });
        const { lineNumber, column } = position;
        return await this.completer.get_completions(
            code,
            lineNumber,
            column,
            interrupt_buffer,
        );
    }

    async getCompletionInfo(state_id, idx, cancellation_token) {
        let interrupt_buffer = createInterruptBuffer();
        cancellation_token.onCancellationRequested(() => {
            interrupt_buffer[0] = 2;
        });

        return await this.completer.get_completion_info(
            state_id,
            idx,
            interrupt_buffer,
        );
    }

    async getSignatures(code, position, cancellation_token) {
        const interrupt_buffer = createInterruptBuffer();
        const { lineNumber, column } = position;
        return await this.completer.get_signatures(
            code,
            lineNumber,
            column,
            interrupt_buffer,
        );
    }

    close() {
        this.completer[Synclink.releaseProxy]();
    }
}
