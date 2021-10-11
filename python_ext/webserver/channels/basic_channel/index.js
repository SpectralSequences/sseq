"use strict"
import Mousetrap from "mousetrap";

import { SseqChart } from "chart/SseqChart";
window.SseqChart = SseqChart;


import { promiseFromDomEvent } from "display/utils"
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
main = main().catch(console.error);
class BasicUI {
    constructor(uiElement, socket_address){
        this.ws = new WebSocket(socket_address);
        this.ws.onclose = function(event){
            document.querySelector("[error=disconnected]").removeAttribute("hidden");
        }
        this.uiElement = uiElement;
        this.chartElement = uiElement.querySelector("sseq-chart")
        this.ws.onmessage = this.onmessage.bind(this);
        this.socket_opened = new Promise((resolve) =>  this.ws.onopen = () => resolve());
        // this.socket_listener = new SseqSocketListener(this.ws);
        // this.socket_listener.attachDisplay(this.display);
    }

    onmessage(event){
        console.log("chart.update", event.data);
        let data = parse(event.data);
        switch(data.cmd[0]){
            case "chart.state.reset":
                console.log("chart.update", data.kwargs.state);
                this.chartElement.setSseq(data.kwargs.state);
                return;
            case "chart.update":
                for(let update of data.kwargs.messages){
                    this.chartElement.sseq.handleMessage(update);
                }
                this.chartElement._updateChart();
                return
        }
        console.log("Unrecognized message:", data);
    }

    async start(){
        await main;
        this.setupUIBindings();
        this.setupSocketMessageBindings();
        await this.socket_opened;
        this.send("initialize.complete", {});  
        this.uiElement.start();
        // this.socket_listener.start();
        // await promiseFromDomEvent(this.uiElement, "started");
        // let display_rect = this.uiElement.querySelector("sseq-display").getBoundingClientRect();
        // this.popup.left = display_rect.width/2 - 250/2;
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
        this.ws.send(json_str);
    }

    setupUIBindings(){
        this.uiElement.mousetrap.bind("t", () => {
            this.send("console.take", {});
        });

        // this.uiElement.mousetrap.bind("h", this.showHelpWindow.bind(this));
        // this.uiElement.querySelector(".help-btn").addEventListener("click", this.showHelpWindow.bind(this))
        // let resizeObserver = new ResizeObserver(entries => {
        //     this.resizeHelpWindow();
        // });
        // resizeObserver.observe(this.uiElement);     
        
        // this.uiElement.addEventListener("keydown-arrow",
        //     throttle(75, { trailing : false })(this.handleArrow.bind(this)));
        // this.uiElement.addEventListener("keypress-wasd", throttle(5)(this.handleWASD.bind(this)));
        // this.uiElement.addEventListener("keypress-pm",
        //     throttle(150, { trailing : false })(this.handlePM.bind(this)));
    }
}
window.BasicUI = BasicUI;