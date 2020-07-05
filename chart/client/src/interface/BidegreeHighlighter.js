import {LitElement, html, css} from 'lit-element';
import { sleep, promiseFromDomEvent, findAncestorElement } from "./utils.js";

export class BidegreeHighlighterElement extends LitElement {
    static get styles() {
        return css`
            span {
                position: absolute; 
                background-color: gray;
                border : 30px transparent;
                filter : blur(7px);
                opacity : 0.7;
            }
        `;
    }

    constructor(){
        super();
        this.numElements = 0;
        this.bidegreeMap = {};
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
    }

    firstUpdated(){
        this.disp = this.closest("sseq-display");
        this.chart = this.closest("sseq-chart");
        this.disp.addEventListener("scale-update", this.handleScaleUpdate);
    }

    render() {
        return html`
            ${Array(this.numElements).fill().map(() => html`<span></span>`)}
        `;
    }

    handleScaleUpdate(){
        for(let elt of Array.from(this.shadowRoot.children).filter(elt => elt.bidegree)){
            this.positionElement(elt);
        }
    }

    positionElement(elt){
        let [x, y] = elt.bidegree;
        let left = this.disp.xScale(x - 1/2);
        let top = this.disp.yScale(y + 1/2)
        let width = this.disp.dxScale(1);
        let height = this.disp.dyScale(-1);
        elt.style.left = `${left}px`;
        elt.style.top = `${top}px`;
        elt.style.width = `${width}px`;
        elt.style.height = `${height}px`;
    }

    clear(){
        let clearedElements = [];
        for(let elt of this.shadowRoot.children){
            if(elt.bidegree) {
                clearedElements.push(elt);
                elt.style.display = "none";
                delete this.bidegreeMap[JSON.stringify(elt.bidegree)];
            }
        }
        sleep(30).then(() => {
            for(let c of clearedElements){
                c.busy = false;
            }
        })
        
    }

    async _setupBidegrees(bidegrees){
        let availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.busy);
        let numElementsNeeded = bidegrees.filter(bidegree => !(JSON.stringify(bidegree) in this.bidegreeMap)).length;
        if(numElementsNeeded > availableElements.length){
            this.numElements += numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.busy);
        }

        for(let bidegree of bidegrees){
            let key = JSON.stringify(bidegree);
            if(key in this.bidegreeMap){
                continue;
            }
            let elt = availableElements.pop();
            elt.bidegree = bidegree;
            elt.style.display = "";
            this.bidegreeMap[key] = elt;
            elt.busy = true;
        }
    }

    async highlight(bidegrees){
        if(bidegrees.constructor != Array){
            bidegrees = [bidegrees];
        }
        if(bidegrees.length === 0){
            return;
        }

        await this._setupBidegrees(bidegrees);
        for(let bidegree of bidegrees){
            let elt = this.bidegreeMap[JSON.stringify(bidegree)];
            this.positionElement(elt);
        }
    }

    // async fire(bidegrees){
    //     if(bidegrees.constructor != Array){
    //         bidegrees = [bidegrees];
    //     }
    //     if(classes.length === 0){
    //         return;
    //     }

    //     await this._setupClasses(classes);

    //     for(let c of classes){
    //         let elt = this.classMap[c.uuid];
    //         elt.removeAttribute("transition");
    //         this.setSize(elt, 0);
    //     }
    //     await sleep(30);

    //     for(let c of classes){
    //         let elt = this.classMap[c.uuid];
    //         elt.style.visibility = "";
    //         elt.setAttribute("transition", "");
    //         this.setSize(elt, 15);
    //         promiseFromDomEvent(elt, "transitionend").then(() => {
    //             elt.removeAttribute("transition");                    
    //         });            
    //     }
    // }
}

customElements.define('sseq-bidegree-highlighter', BidegreeHighlighter);