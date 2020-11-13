import { Swork, FetchContext } from "swork";
import { Router } from "swork-router";
import Mustache from "mustache";
// import nonexistent_chart_html from 'raw-loader!./charts/nonexistent-chart.html';
// import chart_html from 'raw-loader!./charts/chart.html';

console.log("prefix:", self.location.pathname.split("/").slice(0,-1).join("/"));
const app = new Swork();
const router = new Router({
    // /blah/blah/service_worker.bundle.js ==> /blah/blah
    prefix: self.location.pathname.split("/").slice(0,-1).join("/")
});

const repls = {};
const charts = {};

function make_json_response(body, status){
    status.headers = new Headers({
        "Content-Type" : "application/json"
    });
    return new Response(JSON.stringify(body), status);
}

function make_html_response(body, status){
    status.headers = new Headers({
        "Content-Type" : "text/html; charset=UTF-8"
    });
    return new Response(body, status);
}

async function get_owning_client(chart_name){
    if(!(chart_name in charts)){
        return undefined;
    }
    let owningClientId = charts[chart_name];
    let owningClient = await self.clients.get(owningClientId);
    if(!owningClient){
        console.log("Undefiend owning client, deleting chart.");
        if(owningClientId in repls){
            repls[owningClientId].close();
            delete repls[owningClientId];
        }
        delete charts[chart_name];
    }
    return owningClient;
}

router.put("/api/charts/:name", async (context) => {
    let clientId = context.event.clientId;
    let name = context.params.name;
    let owningClient = await get_owning_client(name);
    if(owningClient){
        context.response = make_json_response({ 
                response : `Chart "${name}" already exists.`,
                code : "put-chart::failed::already-exists",
                same_repl_owns_chart : clientId === owningClient.id
            }, 
            { status : 409,  statusText : "Chart already exists" }
        );
        return;
    }
    charts[name] = clientId;
    context.response = make_json_response(
        { response : `Created chart "${name}".`, code : "put-chart::succeeded" },
        { status : 201,   statusText : "Created chart." }
    );
});


router.get("/api/charts/:name", async (context) => {
    let { name } = context.params;
    let owningClient = await get_owning_client(name);
    if(owningClient){
        context.response = make_json_response(
            { clientId : owningClient.id, code : "get-chart::succeeded" },
            { status : 200, statusText : "Found chart" }
        );
        return;
    }
    context.response = make_json_response({ code : "get-chart::failed::not-found" }, {status : 404, statusText : "Chart not found."});
});


router.get("/charts/:name", async (context) => {
    let { name } = context.params;
    console.log(`requested: /charts/${name}`);
    console.log(self.location.pathname);
    if(name.endsWith(".js") || name.endsWith(".wasm")){
        context.response = fetch(`charts/${name}`);
        return;
    }
    let owningClient = await get_owning_client(name);
    if(owningClient){
        let chart_html = await (await fetch("charts/chart.html")).text();
        context.response = make_html_response(
            Mustache.render(chart_html,
                { clientId : owningClient.id, chart_name : name }),
            { status : 200, statusText : "Found chart" }
        );
        return;
    }
    let nonexistent_chart_html = await (await fetch("charts/nonexistent-chart.html")).text();
    context.response = make_html_response(
        Mustache.render(nonexistent_chart_html, { chart_name : name }), 
        {status : 200, statusText : "Chart not found."}
    );
});

app.on("message", handleMessage)
app.use(router.routes());
app.listen();

function handleMessage(event){
    console.log("service_worker:: received message from a client:", event.data, event);
    let message = event.data;
    if(!message.cmd){
        throw Error("Undefined command")
    }
    if(!message.cmd in messageDispatch){
        throw Error("Unknown command.");
    }
    messageDispatch[message.cmd](event);
}

let messageDispatch = {
    pyodide_worker_channel : installPyodideRepl,
    subscribe_chart_display : passChartChannelToPyodide
};

function installPyodideRepl(event){
    let port = event.data.port;
    console.log(`Service worker :: installing pyodide repl :: id : ${event.source.id}`);
    port.addEventListener("message", handlePyodideMessage);
    repls[event.source.id] = port;
    port.start();
}

function handlePyodideMessage(event){
    console.error(`Unexpected message from pyodide repl`, event.data, event);
    throw Error("Unexpected message from pyodide repl:", event.data);
}

async function passChartChannelToPyodide(event){
    let { port, chart_name } = event.data;
    event.data.client_id = event.source.id;
    let owningClient = await get_owning_client(chart_name);
    console.log(`Owning client Id : ${owningClient.id}`);
    let repl_port = repls[owningClient.id];
    repl_port.postMessage(event.data, [port]);
}
