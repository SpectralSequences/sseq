import { Swork, FetchContext } from 'swork';
import { Router } from 'swork-router';
import Mustache from 'mustache';
import * as Synclink from 'synclink';
// import nonexistent_chart_html from 'raw-loader!./charts/nonexistent-chart.html';
// import chart_html from 'raw-loader!./charts/chart.html';

console.log(
    'prefix:',
    self.location.pathname.split('/').slice(0, -1).join('/'),
);
const app = new Swork();
const router = new Router({
    // /blah/blah/service_worker.bundle.js ==> /blah/blah
    prefix: self.location.pathname.split('/').slice(0, -1).join('/'),
});

const repl_connections = {};
const chart_owning_repls = {};

function make_json_response(body, status) {
    status.headers = new Headers({
        'Content-Type': 'application/json',
    });
    return new Response(JSON.stringify(body), status);
}

function make_html_response(body, status) {
    status.headers = new Headers({
        'Content-Type': 'text/html; charset=UTF-8',
    });
    return new Response(body, status);
}

async function get_owning_repl(chart_name) {
    if (!(chart_name in chart_owning_repls)) {
        return undefined;
    }
    let owningReplId = chart_owning_repls[chart_name];
    let owningRepl = await self.clients.get(owningReplId);
    if (!owningRepl) {
        console.log('Undefined owning repl, deleting chart.');
        if (owningReplId in repl_connections) {
            repl_connections[owningReplId].close();
            delete repl_connections[owningReplId];
        }
        delete chart_owning_repls[chart_name];
    }
    return owningRepl;
}

router.put('/api/charts/:name', async context => {
    let clientId = context.event.clientId;
    let name = context.params.name;
    let owningRepl = await get_owning_repl(name);
    if (owningRepl) {
        context.response = make_json_response(
            {
                response: `Chart "${name}" already exists.`,
                code: 'put-chart::failed::already-exists',
                same_repl_owns_chart: clientId === owningRepl.id,
            },
            { status: 409, statusText: 'Chart already exists' },
        );
        return;
    }
    chart_owning_repls[name] = clientId;
    context.response = make_json_response(
        { response: `Created chart "${name}".`, code: 'put-chart::succeeded' },
        { status: 201, statusText: 'Created chart.' },
    );
});

router.get('/api/charts/:name', async context => {
    let { name } = context.params;
    let owningClient = await get_owning_repl(name);
    if (owningClient) {
        context.response = make_json_response(
            { clientId: owningClient.id, code: 'get-chart::succeeded' },
            { status: 200, statusText: 'Found chart' },
        );
        return;
    }
    context.response = make_json_response(
        { code: 'get-chart::failed::not-found' },
        { status: 404, statusText: 'Chart not found.' },
    );
});

router.get('/charts/:name', async context => {
    let { name } = context.params;
    console.log(`requested: /charts/${name}`);
    console.log(self.location.pathname);
    if (
        name.endsWith('.js') ||
        name.endsWith('.wasm') ||
        name.endsWith('.css')
    ) {
        context.response = fetch(`charts/${name}`);
        return;
    }
    let owningRepl = await get_owning_repl(name);
    if (owningRepl) {
        let chart_html = await (await fetch('charts/chart.html')).text();
        context.response = make_html_response(
            Mustache.render(chart_html, {
                clientId: owningRepl.id,
                chart_name: name,
            }),
            { status: 200, statusText: 'Found chart' },
        );
        return;
    }
    let nonexistent_chart_html = await (
        await fetch('charts/nonexistent-chart.html')
    ).text();
    context.response = make_html_response(
        Mustache.render(nonexistent_chart_html, { chart_name: name }),
        { status: 200, statusText: 'Chart not found.' },
    );
});

app.on('install', () => {
    console.log('skipWaiting');
    self.skipWaiting();
});

app.on('activate', async () => {
    console.log('claim');
    await clients.claim();
});

app.on('message', handleMessage);
app.use(router.routes());
app.listen();

function handleMessage(event) {
    console.log(
        'service_worker:: received message from a client:',
        event.data,
        event,
    );
    let message = event.data;
    if (!message.cmd) {
        throw Error('Undefined command');
    }
    if (!message.cmd in messageDispatch) {
        throw Error('Unknown command.');
    }
    messageDispatch[message.cmd](event);
}

let messageDispatch = {
    subscribe_chart_display: passChartChannelToPyodide,
    chart_display_focus_repl: focusRepl,
};

async function getPyodidePort(target_repl) {
    if (!repl_connections[target_repl.id]) {
        // Not already connected or in the process of connecting
        installPyodideRepl(target_repl);
    }
    return repl_connections[target_repl.id];
}

function installPyodideRepl(target_repl) {
    // The pyodide worker needs to be able to send messages to the service worker, so we make a channel.
    // We keep one and send the other to the pyodide worker.
    let { port1, port2 } = new MessageChannel();
    // It would be great if there were any way to detect and manage failure for this.
    // Not sure how though.
    target_repl.postMessage(
        {
            cmd: 'connect_to_pyodide',
            repl_id: target_repl.id,
            port: port2,
        },
        [port2],
    );
    let promise = {};
    promise.promise = new Promise((resolve, reject) =>
        Object.assign(promise, { resolve, reject }),
    );
    repl_connections[target_repl.id] = Synclink.wrap(port1);
}

async function passChartChannelToPyodide(event) {
    const { port, chart_name } = event.data;
    const source_id = event.source.id;
    chart_owning_repls[source_id] = chart_name;
    const owningRepl = await get_owning_repl(chart_name);
    const repl_port = await getPyodidePort(owningRepl);
    await repl_port.connect_chart(
        chart_name,
        source_id,
        Synclink.transfer(port, port),
    );
}

async function focusRepl(event) {
    const chart_name = chart_owning_repls[event.source.id];
    const owningRepl = await get_owning_repl(chart_name);
    owningRepl.focus();
}
