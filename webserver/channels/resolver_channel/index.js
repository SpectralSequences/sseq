"use strict"
import Mousetrap from "mousetrap";

import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;

// import ReconnectingWebSocket from 'reconnecting-websocket';

import { UIElement } from "chart/interface/UIElement.js";
import { Display } from "chart/interface/Display.js";
import { AxesElement } from "chart/interface/Axes.js";
import { GridElement } from "chart/interface/GridElement.js";
import { ChartElement } from "chart/interface/ChartElement.js";
import { ClassHighlighter } from "chart/interface/ClassHighlighter";
import { BidegreeHighlighter } from "chart/interface/BidegreeHighlighter";
import { SseqPageIndicator } from "chart/interface/SseqPageIndicator.js";
import { Tooltip } from "chart/interface/Tooltip.js";

import {Mutex, Semaphore, withTimeout} from 'async-mutex';

import { Sidebar } from "chart/interface/Sidebar.js";
import { Matrix } from "chart/interface/Matrix.js";
import { KatexExprElement } from "chart/interface/KatexExprElement.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { StaleResponseError } from "chart/SocketListener.js";
import { Popup } from "chart/interface/Popup.js";
import { sleep, promiseFromDomEvent, throttle, animationFrame, uuid4 } from "chart/interface/utils.js";

window.SseqSocketListener = SseqSocketListener;
window.UIElement = UIElement;

class ResolverUI {
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

    async showHelpWindow() {
        this.resizeHelpWindow();
        let help_popup = this.uiElement.querySelector(".help");        
        help_popup.show();
        help_popup.focus();
    }

    resizeHelpWindow(){
        let help_popup = this.uiElement.querySelector(".help");
        let display_rect = this.uiElement.querySelector("sseq-display").getBoundingClientRect();
        help_popup.left = 0.2  * display_rect.width;
        help_popup.top = 0.1 * display_rect.height;
        help_popup.width = `${0.6 * display_rect.width}px`;
        help_popup.height = "70vh";//`${0.6 * display_rect.height}px`;
    }


    setupUIBindings(){
        this.uiElement.mousetrap.bind("t", () => {
            this.socket_listener.send("console.take", {});
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
        if(!this.selected_bidegree){
            return;
        }
        let [x, y] = this.selected_bidegree;
        let [dx, dy] = e.detail.direction;
        x += dx;
        y += dy;
        let [minX, maxX] = display.xRange;
        let [minY, maxY] = display.yRange;
        x = Math.min(Math.max(x, minX), maxX);
        y = Math.min(Math.max(y, minY), maxY);

        this.select_bidegree(x, y);
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


window.ResolverUI = ResolverUI;
