'use strict';

// Unit tests for the panic-catching logic added to the wasm workers.
//
// With `panic=unwind`, wasm-bindgen rethrows Rust panics as JS exceptions out
// of the exported `run()` method. The workers wrap that call in a try/catch
// and forward the failure as an `Error` action message, which the frontend
// renders as a dialog (see `interface/index.js`'s `messageHandler.Error`).
//
// These tests load the (tiny) worker scripts into a sandbox with mocked
// browser/wasm globals so we can make `run()` throw and assert on what gets
// posted back, without needing a browser or a compiled wasm module.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));

// Load a worker script into a sandbox with the globals it expects
// (`self`, `importScripts`, `wasm_bindgen`). Returns handles to drive it:
// `self` (with the installed `onmessage`), the list of `posted` messages, and
// the wasm `instance` whose `run` method the test can override.
async function loadWorker(filename, className) {
    const source = readFileSync(join(here, filename), 'utf8');

    const posted = [];
    const instance = {
        run() {
            throw new Error('run() should be overridden by the test');
        },
    };

    // `wasm_bindgen` is both callable (loads the module, returns a promise)
    // and carries the exported classes as properties.
    function wasm_bindgen() {
        return Promise.resolve();
    }
    wasm_bindgen[className] = {
        new() {
            return instance;
        },
    };

    const self = {
        postMessage: msg => posted.push(msg),
        onmessage: null,
    };

    const sandbox = {
        self,
        wasm_bindgen,
        importScripts() {},
        console,
    };
    vm.createContext(sandbox);
    vm.runInContext(source, sandbox, { filename });

    // The script sets `self.<instance>` asynchronously once the (mocked) wasm
    // module "loads". Flush microtasks so it is ready before we return.
    await new Promise(resolve => setImmediate(resolve));

    return { self, posted, instance };
}

const cases = [
    {
        filename: 'sseq_worker.js',
        className: 'Sseq',
        label: 'Panic in sseq worker:',
    },
    {
        filename: 'resolution_worker.js',
        className: 'Resolution',
        label: 'Panic in resolution worker:',
    },
];

for (const { filename, className, label } of cases) {
    test(`${filename}: a panic in run() is forwarded as an Error message`, async () => {
        const { self, posted, instance } = await loadWorker(filename, className);

        instance.run = () => {
            throw new Error('boom');
        };

        self.onmessage({ data: '{"some": "message"}' });

        assert.equal(posted.length, 1, 'exactly one message should be posted');

        const msg = JSON.parse(posted[0]);
        assert.deepEqual(msg.recipients, []);
        assert.equal(msg.sseq, 'Main');
        assert.ok(msg.action.Error, 'message should be an Error action');
        assert.ok(
            msg.action.Error.message.startsWith(label),
            `error message should be labelled with "${label}"`,
        );
        assert.ok(
            msg.action.Error.message.includes('boom'),
            'error message should include the panic text',
        );
    });

    test(`${filename}: a successful run() posts nothing`, async () => {
        const { self, posted, instance } = await loadWorker(filename, className);

        let received;
        instance.run = data => {
            received = data;
        };

        self.onmessage({ data: 'payload' });

        assert.equal(received, 'payload', 'run() should receive the message data');
        assert.equal(posted.length, 0, 'no error message should be posted');
    });
}
