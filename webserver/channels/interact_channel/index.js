"use strict"
import Mousetrap from "mousetrap";

import { SseqChart } from "chart/sseq/SseqChart";
window.SseqChart = SseqChart;

// import ReconnectingWebSocket from 'reconnecting-websocket';

import { AxesElement } from "chart/interface/Axes.js";
import { BidegreeHighlighterElement } from "chart/interface/BidegreeHighlighter";
import { ChartElement } from "chart/interface/Chart.js";
import { ClassHighlighter } from "chart/interface/ClassHighlighter";
import { DisplayElement } from "chart/interface/Display.js";
import { GridElement } from "chart/interface/Grid.js";
import { KatexExprElement } from "chart/interface/KatexExpr.js";
import { MatrixElement } from "chart/interface/Matrix.js";
import { PageIndicatorElement } from "chart/interface/PageIndicator.js";
import { PopupElement } from "chart/interface/Popup.js";
import { SidebarElement } from "chart/interface/Sidebar.js";
import { TooltipElement } from "chart/interface/Tooltip.js";
import { UIElement } from "chart/interface/UI.js";

import {Mutex, Semaphore, withTimeout} from 'async-mutex';

import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { StaleResponseError } from "chart/SocketListener.js";
import { Popup } from "chart/interface/Popup.js";
import { sleep, promiseFromDomEvent, throttle, animationFrame, uuid4 } from "chart/interface/utils.js";

class InteractUI {
    constructor(uiElement, socket_address){
        this.ws = new WebSocket(socket_address);
        this.ws.onclose = function(event){
            document.querySelector("[error=disconnected]").removeAttribute("hidden");
        }
        this.uiElement = uiElement;
        this.display = uiElement.querySelector("sseq-display")
        this.socket_listener = new SseqSocketListener(this.ws);
        this.socket_listener.attachDisplay(this.display);
        this.uiElement.addEventListener("keypress",(e) => {
            if(e.code.startsWith("Bracket")){
                this.handleBracketPress(e);
            }
        });
    }

    async start(){
        this.setupUIBindings();
        this.setupSocketMessageBindings();        
        this.socket_listener.start();
        await promiseFromDomEvent(this.uiElement, "started");
        // let display_rect = this.uiElement.querySelector("sseq-display").getBoundingClientRect();
        // this.popup.left = display_rect.width/2 - 250/2;
    }

    setupSocketMessageBindings(){}

    setupUIBindings(){
        this.uiElement.mousetrap.bind("t", () => {
            this.socket_listener.send("console.take", {});
        });

        this.uiElement.mousetrap.bind("n", () => {
            this.socket_listener.send("demo.next", {});
        });

        // this.uiElement.mousetrap.bind("h", this.showHelpWindow.bind(this));
        // this.uiElement.querySelector(".help-btn").addEventListener("click", this.showHelpWindow.bind(this))
        // let resizeObserver = new ResizeObserver(entries => {
        //     this.resizeHelpWindow();
        // });
        // resizeObserver.observe(this.uiElement);     
        
        this.uiElement.addEventListener("keydown-arrow",
            throttle(75, { trailing : false })(this.handleArrow.bind(this)));
        this.uiElement.addEventListener("keypress-wasd", throttle(5)(this.handleWASD.bind(this)));
        this.uiElement.addEventListener("keypress-pm",
            throttle(150, { trailing : false })(this.handlePM.bind(this)));
    }

    handleArrow(e){
        let [dx, dy] = e.detail.direction;
        this.uiElement.querySelector("sseq-chart").changePage(dx);
    }
    
    async handleWASD(e){
        await animationFrame();
        let [dx, dy] = e.detail.direction;
        let s = 20;
        display.translateBy( - dx * s, dy * s);
    }

    getZoomCenter(){
        if(this.selected_bidegree){
            let [x, y] = this.selected_bidegree;
            return [display.xScale(x), display.yScale(y)];
        }
        return undefined;
    }

    handlePM(e){
        let d = e.detail.direction;
        let zoomCenter = this.getZoomCenter();
        display.scaleBy(d, zoomCenter);
    }

    handleBracketPress(e){
        let d = e.code.endsWith("Right") ? 1 : -1;
        let zoomCenter = this.getZoomCenter();
        if(e.shiftKey){
            this.display.scaleYBy(d, zoomCenter);
        } else {
            this.display.scaleXBy(d, zoomCenter);
        }
    }


    sortedClasses(){
        return Object.values(
            this.uiElement.querySelector("sseq-chart").sseq.classes
        )
        .sort((a,b) => (a.x - b.x)*10 + Math.sign(a.y - b.y));
    }
}
window.InteractUI = InteractUI;
