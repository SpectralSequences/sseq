import Mousetrap from "mousetrap";

import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
import { sleep, promiseFromDomEvent, throttle, animationFrame } from "chart/interface/utils.js";
import { v4 as uuid4 } from "uuid";

window.SpectralSequenceChart = SpectralSequenceChart;

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

import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { StaleResponseError } from "chart/SocketListener.js";
window.StaleResponseError = StaleResponseError;
// import {Mutex, Semaphore, withTimeout} from 'async-mutex';


import registerServiceWorker, {
    ServiceWorkerNoSupportError
} from 'service-worker-loader!../service.worker';
 
registerServiceWorker({ scope: '/dist/' }).then((registration) => {
    console.log("Loaded worker!");
}).catch((err) => {
    if (err instanceof ServiceWorkerNoSupportError) {
        console.error('Service worker is not supported.');
    } else {
        console.error('Error loading service worker!', err);
    }
});


class ReplDisplayUI {
    constructor(uiElement, chart_name){
        let {port1, port2} = new MessageChannel();
        navigator.serviceWorker.controller.postMessage({
            cmd : "subscribe_chart_display", 
            port : port1,
            chart_name,
            uuid : uuid4()
        }, [port1]);
        
        this.uiElement = uiElement;
        this.display = uiElement.querySelector("sseq-display")
        this.popup = uiElement.querySelector("sseq-popup");
        this.sidebar = uiElement.querySelector("sseq-sidebar");    
        this.socket_listener = new SseqSocketListener(port2);
        this.socket_listener.debug_mode = true;
        this.socket_listener.attachDisplay(this.display);
    }

    async start(){
        this.socket_listener.start();
        await promiseFromDomEvent(this.uiElement, "started");
    }
    
}
window.ReplDisplayUI = ReplDisplayUI;
