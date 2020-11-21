"use strict";
import Mousetrap from "mousetrap";

import { SseqChart } from "chart/SseqChart";
window.SseqChart = SseqChart;


import { promiseFromDomEvent, sleep } from "display/utils"
// import ReconnectingWebSocket from 'reconnecting-websocket';

import { AxesElement } from "display/Axes.js";
import { BidegreeHighlighterElement } from "display/BidegreeHighlighter";
import { ClassHighlighter } from "display/ClassHighlighter";
// import { KatexExprElement } from "chart/interface/KatexExpr.js";
import { MatrixElement } from "display/Matrix.js";
import { PageIndicatorElement } from "display/PageIndicator.js";
import { PopupElement } from "display/Popup.js";
import { SidebarElement } from "display/Sidebar.js";
import { TooltipElement } from "display/Tooltip.js";
import { UIElement } from "display/UI.js";
import { v4 as uuid4 } from "uuid";
import { parse } from "chart/json_utils";

window.parseChart = parse;



async function main(){
    await import("display/Chart.ts");
}
let chart_loaded = main().catch(console.error);



// import registerServiceWorker, {
//     ServiceWorkerNoSupportError
// } from 'service-worker-loader!../service.worker';
 
// let service_worker_loaded = registerServiceWorker({ scope: '/dist/' }).then((registration) => {
//     console.log("Loaded worker!");
// }).catch((err) => {
//     if (err instanceof ServiceWorkerNoSupportError) {
//         console.error('Service worker is not supported.');
//     } else {
//         console.error('Error loading service worker!', err);
//     }
// });


class ReplDisplayUI {
    constructor(uiElement, chart_name){
        let {port1, port2} = new MessageChannel();
        navigator.serviceWorker.controller.postMessage({
            cmd : "subscribe_chart_display", 
            port : port1,
            chart_name,
            uuid : uuid4()
        }, [port1]);
        
        // this.ws.onclose = function(event){
        //     document.querySelector("[error=disconnected]").removeAttribute("hidden");
        // }
        // this.socket_opened = new Promise((resolve) =>  this.ws.onopen = () => resolve());

        this.uiElement = uiElement;
        this.chartElement = uiElement.querySelector("sseq-chart")
        this.mousetrap = new Mousetrap(this.chartElement);
        this.popup = uiElement.querySelector("sseq-popup");
        this.sidebar = uiElement.querySelector("sseq-sidebar");    
        this.port = port2;
        this.port.onmessage = this.onmessage.bind(this);
    }

    async start(){
        await chart_loaded;
        await navigator.serviceWorker.ready;
        this.setupUIBindings();
        this.setupSocketMessageBindings();
        this.send("initialize.complete", {});  // Who reads this message?
        await this.uiElement.start();
        this.send("new_user", {});
    }



    onmessage(event){
        let data = parse(event.data);
        if(data.cmd.startsWith("chart.")){
            this.chartElement.handleMessage(data);
            return;
        }
        console.error("Message with unrecognized command:", message);
        throw Error(`Unrecognized command ${message.cmd}`);
    }

    setupSocketMessageBindings(){}

    send(cmd, kwargs) { // args parameter?
        let args = [];
        // this.console_log_if_debug("send message", cmd, args, kwargs);
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
        let obj = Object.assign({ 
                cmd, args, kwargs,
                uuid : uuid4()
            },
            this.message_extra_data
        );
        let json_str = JSON.stringify(obj);
        this.port.postMessage(json_str);
    }

    setupUIBindings(){
        Mousetrap.bind("left", () => this.chartElement.previousPage());
        Mousetrap.bind("right", () => this.chartElement.nextPage());
    }
    
}
window.ReplDisplayUI = ReplDisplayUI;


