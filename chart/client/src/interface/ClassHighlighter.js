import {LitElement, html, css} from 'lit-element';
import { sleep, promiseFromDomEvent, findAncestorElement } from "./utils.js";

export class ClassHighlighter extends LitElement {
    static get styles() {
        return css`
            span {
                position: absolute; 
                background-color: orange; 
                border-radius: 50%; 
                height: 0px; 
                width: 0px;
                --transition-time : 0.3s;
                transition : none;
            }

            span[transition] {
                transition-timing-function: cubic-bezier(0,.27,1,5);
                transition-property: all;
                transition-duration : var(--transition-time);
            }
        `;
    }

    constructor(){
        super();
        this.numElements = 0;
        this.classMap = {};
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
        for(let elt of Array.from(this.shadowRoot.children).filter(elt => elt.cls)){
            this.setSize(elt);
        }
    }

    setSize(elt, size){
        if(elt.cls === undefined){
            return;
        }
        // In case cls is out of date, look up the current class with same uuid
        // Once in a while this out of dateness is caused by chart.class.update in sseqsocketlistener
        // TODO: can we change design so this isn't needed? I don't understand why identity of cls changes...
        elt.cls = this.chart.sseq.classes[elt.cls.uuid];
        if(elt.cls === undefined){
            elt.style.display = "none";
            return;
        }
        if(elt.cls.isDisplayed()){
            elt.style.display = "";
        } else {
            elt.style.display = "none";
        }
        let x = elt.cls._canvas_x;
        let y = elt.cls._canvas_y;
        if(size !== undefined){
            elt.size = size;
        }
        size = elt.size;
        elt.style.left = `${x - size/2}px`;
        elt.style.top = `${y - size/2}px`;
        elt.style.height = `${size}px`;
        elt.style.width = `${size}px`;;
    }

    clear(){
        let clearedClasses = [];
        for(let elt of this.shadowRoot.children){
            if(elt.cls) {
                clearedClasses.push(elt);
                elt.style.removeProperty("--transition-time");
                elt.removeAttribute("transition");
                this.setSize(elt, 0);
                delete this.classMap[elt.cls.uuid];
                elt.cls = undefined;
            }
        }
        sleep(30).then(() => {
            for(let c of clearedClasses){
                c.busy = false;
            }
        })
        
    }

    async _setupClasses(classes){
        let availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.busy);
        let numElementsNeeded = classes.filter(c => !(c.uuid in this.classMap)).length;
        if(numElementsNeeded > availableElements.length){
            this.numElements += numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.busy);
        }

        for(let c of classes){
            if(c.uuid in this.classMap){
                continue;
            }
            let elt = availableElements.pop();
            this.classMap[c.uuid] = elt;
            elt.cls = c;
            elt.busy = true;   
        }
    }

    async highlight(classes){
        if(classes.constructor != Array){
            classes = [classes];
        }
        if(classes.length === 0){
            return;
        }

        await this._setupClasses(classes);

        for(let c of classes){
            let elt = this.classMap[c.uuid];
            this.setSize(elt, 15);
        }
    }

    async fire(classes){
        if(classes.constructor != Array){
            classes = [classes];
        }
        if(classes.length === 0){
            return;
        }

        await this._setupClasses(classes);

        for(let c of classes){
            let elt = this.classMap[c.uuid];
            elt.removeAttribute("transition");
            this.setSize(elt, 0);
        }
        await sleep(30);

        for(let c of classes){
            let elt = this.classMap[c.uuid];
            elt.style.visibility = "";
            elt.setAttribute("transition", "");
            this.setSize(elt, 15);
            promiseFromDomEvent(elt, "transitionend").then(() => {
                elt.removeAttribute("transition");                    
            });            
        }

    }
}

customElements.define('sseq-class-highlighter', ClassHighlighter);