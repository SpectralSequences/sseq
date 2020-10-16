var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { LitElement, html, css } from 'lit-element';
import { sleep, promiseFromDomEvent } from "./utils.js";
export class AxesElement extends LitElement {
    static get styles() {
        return css `
            * {
                z-index : 1;
            }

            :host {
                --axes-thickness : 1px;
            }

            #x-axis {
                position: absolute;
                height : var(--axes-thickness);
                background : black;
            }
            
            #y-axis {
                position: absolute;
                width : var(--axes-thickness);
                background : black;
            }            

            .tick {
                position : absolute;
            }

            .tick[transition] {
                transition : opacity ease-in 0.2s;
            }
        `;
    }
    constructor() {
        super();
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
        this.handleCanvasInitialize = this.handleCanvasInitialize.bind(this);
        this.numTickElements = 0;
        this.tickMap = { "x": {}, "y": {} };
    }
    firstUpdated() {
        this.parentElement.addEventListener("canvas-initialize", this.handleCanvasInitialize);
        this.parentElement.addEventListener("scale-update", this.handleScaleUpdate);
    }
    handleCanvasInitialize() {
        let left = this.parentElement._leftMargin;
        {
            let top = this.parentElement._clipHeight;
            let width = this.parentElement._canvasWidth - this.parentElement._rightMargin - this.parentElement._leftMargin;
            this.shadowRoot.querySelector("#x-axis").style.left = `${left}px`;
            this.shadowRoot.querySelector("#x-axis").style.top = `${top}px`;
            this.shadowRoot.querySelector("#x-axis").style.width = `${width}px`;
        }
        {
            let top = this.parentElement._topMargin;
            let height = this.parentElement._clipHeight - top;
            this.shadowRoot.querySelector("#y-axis").style.left = `${left}px`;
            this.shadowRoot.querySelector("#y-axis").style.top = `${top}px`;
            this.shadowRoot.querySelector("#y-axis").style.height = `${height}px`;
        }
    }
    handleScaleUpdate() {
        return __awaiter(this, void 0, void 0, function* () {
            let disp = this.parentElement;
            let xTicks = [];
            let xTickTop = disp._clipHeight;
            let xScale = disp.xScale;
            {
                let d3XTicks = disp.xScale.ticks(disp._canvasWidth / 70);
                let xTickStep = Math.ceil(d3XTicks[1] - d3XTicks[0]);
                for (let i = Math.floor(d3XTicks[0]); i <= d3XTicks[d3XTicks.length - 1]; i += xTickStep) {
                    xTicks.push(i);
                }
            }
            let yTicks = [];
            let yTickRight = this.parentElement._leftMargin;
            let yScale = this.parentElement.yScale;
            {
                let d3YTicks = disp.yScale.ticks(disp._canvasHeight / 70);
                let yTickStep = Math.ceil(d3YTicks[1] - d3YTicks[0]);
                for (let i = Math.floor(d3YTicks[0]); i <= d3YTicks[d3YTicks.length - 1]; i += yTickStep) {
                    yTicks.push(i);
                }
            }
            let numElementsNeeded = xTicks.filter(i => this.tickMap["x"][i] === undefined).length
                + yTicks.filter(i => this.tickMap["y"][i] === undefined).length;
            let allElements = Array.from(this.shadowRoot.querySelectorAll(".tick"));
            let availableElements = allElements.filter(e => e.updateId === undefined);
            if (numElementsNeeded > availableElements.length) {
                this.numTickElements += numElementsNeeded - availableElements.length;
                this.requestUpdate();
                yield sleep(10);
                allElements = Array.from(this.shadowRoot.querySelectorAll(".tick"));
                availableElements = allElements.filter(e => e.updateId === undefined);
            }
            let curUpdateId = Math.random();
            for (let i of xTicks) {
                let elt = this.tickMap["x"][i];
                if (elt === undefined) {
                    elt = availableElements.pop();
                    elt.tickType = "x";
                    elt.tickValue = i;
                    elt.innerText = i;
                    elt.style.top = `${xTickTop}px`;
                    this.tickMap["x"][i] = elt;
                }
                elt.updateId = curUpdateId;
                elt.style.opacity = 1;
                elt.style.left = `${xScale(i) - elt.clientWidth / 2}px`;
            }
            for (let i of yTicks) {
                let elt = this.tickMap["y"][i];
                if (elt === undefined) {
                    elt = availableElements.pop();
                    elt.tickType = "y";
                    elt.tickValue = i;
                    elt.innerText = i;
                    elt.style.right = `${yTickRight}px`;
                    this.tickMap["x"][i] = elt;
                }
                elt.updateId = curUpdateId;
                elt.style.opacity = 1;
                elt.style.top = `${yScale(i)}px`;
            }
            for (let elt of this.shadowRoot.querySelectorAll(".tick")) {
                if (elt.updateId === undefined || elt.updateId === curUpdateId) {
                    continue;
                }
                elt.style.opacity = 0;
                elt.setAttribute("transition", "");
                if (elt.tickType === "x") {
                    elt.style.left = `${xScale(elt.tickValue) - elt.clientWidth / 2}px`;
                }
                else {
                    // elt.style.top = 
                }
                promiseFromDomEvent(elt, "transitionend").then(() => {
                    elt.removeAttribute("transition");
                    if (elt.updateId) {
                        delete elt.updateId;
                        delete this.tickMap[elt.tickType][elt.tickValue];
                        delete elt.tickType;
                        delete elt.tickValue;
                    }
                });
            }
        });
    }
    render() {
        return html `
            <div id="x-axis" class="axis"></div>
            <div id="y-axis" class="axis"></div>
            <div>
                ${Array(this.numTickElements).fill().map(() => html `<span class="tick"></span>`)}
            </div>
        `;
    }
}
customElements.define('sseq-axes', AxesElement);
