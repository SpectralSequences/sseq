import { Swork, FetchContext } from "swork";
import { Router } from "swork-router";
 
const app = new Swork();
const router = new Router({
    prefix: "/dist"
});
 
const charts = {};
charts["my-chart"] = "This is a chart.";
charts["ko2"] = "A chart of ko2.";

router.get("/chart/get/:id", (context) => {
    let { id } = context.params;
    if(!(id in charts)){
        context.response = new Response(`No chart named ${id}`);
        return
    }
    context.response = new Response(charts[id]);
});


router.get("/chart/status/:id", (context) => {
    let { id } = context.params;
    context.response = new Response(id in charts ? "exists" : "doesnotexist");
});

app.on("message", handleMessage)

app.use(router.routes());
 
app.listen();

self.pyodideWorkers = [];

function handleMessage(event){
    console.log("service message", event.data);
    let message = event.data;
    if(!message.cmd){
        throw Error("Undefined command")
    }
    if(message.cmd !== "pyodide_worker_channel"){
        throw Error("Unknown command.");
    }
    let port = message.port;
    // port.addEventListener("message", handlePyodideMessage);
    port.addEventListener("message", (e) => console.log("port message", e.data));
    self.pyodideWorkers.push(port);
    port.start();
    console.log(message);
    return;
}



