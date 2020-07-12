import {LitElement, html, css} from 'lit-element';

import { sleep, promiseFromDomEvent } from "./utils.js";

export class AxesElement extends LitElement {
    static get styles() {
        return css`
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
                line-height: 0pt;
            }

            .tick[type=x]{
                transform : translateY(5pt);
            }

            .tick[type=y]{
                transform : translateX(-5pt);
            }

            .tick[transition] {
                transition : opacity ease-out 0.3s;
            }
        `;
    }

    constructor(){
        super();
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
        this.handleCanvasInitialize = this.handleCanvasInitialize.bind(this);
        this.numTickElements = 0;
        this.tickMap = {"x" : {}, "y" : {}};
    }

    firstUpdated(){
        this.parentElement.addEventListener("canvas-initialize", this.handleCanvasInitialize)
        this.parentElement.addEventListener("scale-update", this.handleScaleUpdate);
    }

    handleCanvasInitialize(){
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

    async handleScaleUpdate(event){
        let disp = this.parentElement;

        let d3XTicks = disp.xScale.ticks(disp._canvasWidth / 70);
        let d3YTicks = disp.yScale.ticks(disp._canvasHeight / 70);
        let minXTick = Math.ceil(d3XTicks[0]);
        let maxXTick = Math.floor(d3XTicks[d3XTicks.length - 1]);
        let xTickStep = Math.ceil(d3XTicks[1] - d3XTicks[0]);
        
        
        let minYTick = Math.ceil(d3YTicks[0]);
        let maxYTick = Math.floor(d3YTicks[d3YTicks.length - 1]);
        let yTickStep = Math.ceil(d3YTicks[1] - d3YTicks[0]);

        if(disp.dxScale(+disp.xminFloat - minXTick + xTickStep) < 1.5 ){
            minXTick -= xTickStep;
        }

        if(disp.dxScale(-disp.xmaxFloat + maxXTick - xTickStep) < 1.5 ){
            maxXTick += xTickStep;
        }

        if(disp.dyScale(-disp.yminFloat + minYTick - yTickStep) < 1.5 ){
            minYTick -= yTickStep;
        }
        if(disp.dyScale(-disp.ymaxFloat + maxYTick - yTickStep) < 1.5 ){
            maxYTick += yTickStep;
        }

        let xTicks = [];
        for(let i = minXTick; i <= maxXTick; i += xTickStep) {
            xTicks.push(i);
        }
        
        let yTicks = [];
        for(let i = minYTick; i <= maxYTick; i += yTickStep) {
            yTicks.push(i);
        }

        let numElementsNeeded =
            xTicks.filter(i => this.tickMap["x"][i] === undefined).length 
          + yTicks.filter(i => this.tickMap["y"][i] === undefined).length;
        
        let allElements = Array.from(this.shadowRoot.querySelectorAll(".tick"));
        let availableElements = allElements.filter(e => e.updateId === undefined);
        if(numElementsNeeded > availableElements.length){
            this.numTickElements += numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            allElements = Array.from(this.shadowRoot.querySelectorAll(".tick"));
            availableElements = allElements.filter(e => e.updateId === undefined);
        }

        let xTickTop = disp._clipHeight;
        let curUpdateId = Math.random();
        for (let i of xTicks) {
            let elt = this.tickMap["x"][i];
            if(elt === undefined){
                elt = availableElements.pop();
                elt.setAttribute("type", "x");
                elt.tickType = "x";
                elt.tickValue = i;
                elt.innerText = i;
                this.tickMap["x"][i] = elt;
            }
            elt.updateId = curUpdateId;
            elt.style.opacity = 1;
            let fontSize = parseInt(window.getComputedStyle(elt).fontSize);
            elt.style.top = `${xTickTop + fontSize/2}px`;
            elt.style.left = `${disp.xScale(i) - elt.clientWidth/2}px`;
        }

        let yTickRight = disp._leftMargin;
        for (let i of yTicks) {
            let elt = this.tickMap["y"][i];
            if(elt === undefined){
                elt = availableElements.pop();
                elt.setAttribute("type", "y");
                elt.tickType = "y";
                elt.innerText = i;
                elt.tickValue = i;
                this.tickMap["y"][i] = elt;
                // If the following goes outside the conditional, single digit labels jitter.
                // For some reason we don't see a similar problem on x axis.
                elt.style.left = `${yTickRight - elt.clientWidth}px`;
            }
            elt.updateId = curUpdateId;
            elt.style.opacity = 1;
            elt.style.top = `${disp.yScale(i) - elt.clientHeight/2}px`;
        }
        for(let elt of this.shadowRoot.querySelectorAll(".tick")){
            if(elt.updateId === undefined || elt.updateId === curUpdateId){
                continue;
            }
            if(elt.tickValue === undefined){
                console.error(elt);
            }
            elt.style.opacity = 0;
            if(elt.tickType === "x"){
                elt.style.left = `${disp.xScale(elt.tickValue) - elt.clientWidth/2}px`;
            } else {
                elt.style.top = `${disp.yScale(elt.tickValue) - elt.clientHeight/2}px`;
            }
            let cleanUpElement = () => {
                if(!elt.updateId || elt.style.opacity === "1"){
                    return;
                }
                elt.removeAttribute("transition");
                delete this.tickMap[elt.tickType][elt.tickValue];
                delete elt.updateId;
                delete elt.tickType;
                delete elt.tickValue;
                elt.style.opacity = 0;
            };
            switch(event.detail[0].type){
                case "zoom":
                    cleanUpElement();
                    break;
                    
                case "pan":
                    elt.setAttribute("transition", "");
                    promiseFromDomEvent(elt, "transitionend").then(cleanUpElement);
                    break;
                
                default:
                    throw Error("Invalid scale change event type");
            }
            elt.style.opacity = 0;
        }
    }    

    render(){
        return html `
            <div id="x-axis" class="axis"></div>
            <div id="y-axis" class="axis"></div>
            <div>
                ${Array(this.numTickElements).fill().map(() => html`<span class="tick"></span>`)}
            </div>
        `;
    }


    // _drawAxes(context){
    //     context.save();

    //     // This makes the white square in the bottom left and top right corners which prevents axes labels from appearing to the left
    //     // or below the axes intercept.
    //     context.beginPath();
    //     context.fillStyle = this.background_color;
    //     context.rect(0, this._clipHeight, this._leftMargin, this._bottomMargin);
    //     context.rect(0, 0, this._leftMargin, this._topMargin);
    //     context.fill();
    //     context.fillStyle = "#000";

    //     // Draw the axes.
    //     context.beginPath();
    //     context.moveTo(this._leftMargin, this._topMargin);
    //     context.lineTo(this._leftMargin, this._clipHeight);
    //     context.lineTo(this._canvasWidth - this._rightMargin, this._clipHeight);
    //     context.stroke();

    //     context.restore();
    // }

    
    // _drawTicks(context) {
    //     context.save();

    //     let xTicks = this.xScale.ticks(this._canvasWidth / 70);
    //     let yTicks = this.yScale.ticks(this._canvasHeight / 70);
    //     let minXTick = Math.floor(xTicks[0]);
    //     let maxXTick = xTicks[xTicks.length - 1];
        
    //     let minYTick = Math.floor(yTicks[0]);
    //     let maxYTick = yTicks[yTicks.length - 1];

    //     let xTickStep = Math.ceil(xTicks[1] - xTicks[0]);
    //     let yTickStep = Math.ceil(yTicks[1] - yTicks[0]);

    //     minXTick -= xTickStep;
    //     maxXTick += xTickStep;
    //     minYTick -= yTickStep
    //     maxYTick -= yTickStep
    //     context.textBaseline = "middle";
    //     context.font = "15px Arial";
    //     context.textAlign = "center";
    //     for (let i = minXTick; i <= maxXTick; i += xTickStep) {
    //         context.fillText(i, this.xScale(i), this._clipHeight + 20);
    //     }

    //     context.textAlign = "right";
    //     for (let i = minYTick; i <= maxYTick; i += yTickStep) {
    //         context.fillText(i, this._leftMargin - 10, this.yScale(i));
    //     }
    //     context.restore();
    // }
}
customElements.define('sseq-axes', AxesElement);