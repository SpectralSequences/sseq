'use strict';
import css from './katex.min.css';
import Mousetrap from 'mousetrap';
import * as Comlink from 'comlink';

import { SseqChart } from 'chart/SseqChart';
window.SseqChart = SseqChart;

import { renderLatex } from 'display/latex.js';

import { promiseFromDomEvent, sleep } from 'display/utils';
// import ReconnectingWebSocket from 'reconnecting-websocket';

import { AxesElement } from 'display/Axes.js';
import { BidegreeHighlighterElement } from 'display/BidegreeHighlighter';
import { ClassHighlighter } from 'display/ClassHighlighter';
// import { KatexExprElement } from "chart/interface/KatexExpr.js";
import { MatrixElement } from 'display/Matrix.js';
import { PageIndicatorElement } from 'display/PageIndicator.js';
import { PopupElement } from 'display/Popup.js';
import { SidebarElement } from 'display/Sidebar.js';
import { TooltipElement } from 'display/Tooltip.js';
import { UIElement } from 'display/UI.js';
import { v4 as uuid4 } from 'uuid';
import { parse } from 'chart/json_utils';

async function main() {
    await import('display/Chart.ts');
}
let chart_loaded = main().catch(console.error);

class ReplDisplayUI {
    constructor(uiElement, chart_name) {
        let { port1, port2 } = new MessageChannel();
        navigator.serviceWorker.controller.postMessage(
            {
                cmd: 'subscribe_chart_display',
                port: port1,
                chart_name,
                uuid: uuid4(),
            },
            [port1],
        );

        this.uiElement = uiElement;
        this.chartElement = uiElement.querySelector('sseq-chart');
        this.mousetrap = new Mousetrap(this.chartElement);
        this.popup = uiElement.querySelector('sseq-popup');
        this.sidebar = uiElement.querySelector('sseq-sidebar');

        const toExpose = [this.initializeSseq, this.reset, this.appplyMessages];
        const exposedFunctions = {};
        for (const func of toExpose) {
            exposedFunctions[func.name] = func.bind(this);
        }
        this.port = Comlink.expose(exposedFunctions, port2);
    }

    //
    async initializeSseq(sseq) {
        await chart_loaded;
        console.log('initializeSseq', sseq);
        this.chartElement.initializeSseq(parse(sseq));
        this.uiElement.start();
    }

    reset(sseq) {
        this.chartElement.reset(sseq);
    }

    appplyMessages(messages) {
        console.log(messages);
        this.chartElement.appplyMessages(parse(messages));
    }

    async showHelpWindow() {
        this.resizeHelpWindow();
        let help_popup = this.uiElement.querySelector('.help');
        help_popup.show();
        help_popup.focus();
    }

    resizeHelpWindow() {
        let help_popup = this.uiElement.querySelector('.help');
        let display_rect = this.uiElement
            .querySelector('sseq-chart')
            .getBoundingClientRect();
        help_popup.left = 0.2 * display_rect.width;
        help_popup.top = 0.1 * display_rect.height;
        help_popup.width = `${0.6 * display_rect.width}px`;
        help_popup.height = '70vh'; //`${0.6 * display_rect.height}px`;
    }

    setupUIBindings() {
        Mousetrap.bind('left', () => this.chartElement.previousPage());
        Mousetrap.bind('right', () => this.chartElement.nextPage());
        this.uiElement.mousetrap.bind('h', this.showHelpWindow.bind(this));
        this.uiElement
            .querySelector('.help-btn')
            .addEventListener('click', this.showHelpWindow.bind(this));
        this.chartElement.addEventListener('click-class', event => {
            let highlighter = this.uiElement.querySelector(
                'sseq-class-highlighter',
            );
            highlighter.clear();
            let cls = event.detail.cls;
            highlighter.fire([cls]);
            let page = this.chartElement.page;
            let div = this.uiElement
                .querySelector('sseq-sidebar')
                .querySelector('#names');
            div.innerHTML = `Name: ${renderLatex(
                '\\(' + cls.name[page] + '\\)',
            )}`;
        });
    }
}
window.ReplDisplayUI = ReplDisplayUI;
