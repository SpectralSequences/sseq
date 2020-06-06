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

            span[transition=fire] {
                transition-timing-function: cubic-bezier(0,.27,1,5);
                transition-property: all;
                transition-duration : var(--transition-time);
            }

            span[transition=show] {
                transition-timing-function: ease;
                transition-property: all;
                transition-duration : var(--transition-time);
            }

            span[transition=hide] {
                transition-timing-function: ease;
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

    async clearClass(cls){
        if(!cls){
            return;
        }
        let elt = this.classMap[cls.uuid];
        if(elt === undefined){
            return;
        }
        elt.style.removeProperty("--transition-time");
        elt.removeAttribute("transition");
        this.setSize(elt, 0);
        delete this.classMap[elt.cls.uuid];
        elt.cls = undefined;
        await sleep(30); 
        delete elt.fireID;
    }

    async clear(){
        let promises = [];
        for(let elt of this.shadowRoot.children){
            if(elt.cls) {
                promises.push(this.clearClass(elt.cls));
            }
        }
        await Promise.all(promises);
    }

    async clearClasses(classes){
        let promises = [];
        for(let cls of classes){
            promises.push(this.clearClass(cls));
        }
        await Promise.all(promises);
    }    

    async allocateClasses(fireID, classes){
        let availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.fireID);
        let numElementsNeeded = classes.filter(c => !(c.uuid in this.classMap)).length;
        if(numElementsNeeded > availableElements.length){
            this.numElements += numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.fireID);
        }

        for(let c of classes){
            if(c.uuid in this.classMap){
                this.classMap[c.uuid].fireID = fireID;
                continue;
            }
            let elt = availableElements.pop();
            this.classMap[c.uuid] = elt;
            elt.cls = c;
            elt.fireID = fireID;
        }
    }

    async highlight(classes){
        if(classes.constructor != Array){
            classes = [classes];
        }
        if(classes.length === 0){
            return;
        }

        let fireID = Math.random();
        await this.allocateClasses(fireID, classes);
        await this.prepareElements(fireID, classes, 15, 0.7);
        // await this.transitionClasses(fireID, classes, "none", 15, 0.7);
    }

    async fire(classes){
        if(classes.constructor != Array){
            classes = [classes];
        }
        if(classes.length === 0){
            return;
        }
        let opacity = 0.7;
        
        let fireID = Math.random();
        // console.log("fire")
        // console.log("   allocate");
        await this.allocateClasses(fireID, classes);
        // console.log("   prepare");
        await this.prepareElements(fireID, classes, 0, opacity);
        await sleep(30);
        // console.log("   transition");
        await this.transitionClasses(fireID, classes, "fire", 15, opacity);
        // console.log("fire completed");
    }

    async hideClasses(classes){
        let fireID = Math.random();
        // console.log("hide")
        // console.log("   allocate");        
        await this.allocateClasses(fireID, classes);
        // console.log("   transition");
        await this.transitionClasses(fireID, classes, "hide", 15, 0);
        // console.log("   clear");
        await this.clearClasses(classes);
        // console.log("hide completed")
    }

    async prepareElements(fireID, classes, size, opacity){
        for(let c of classes){
            let elt = this.classMap[c.uuid];
            if(!elt || elt.fireID !== fireID){
                continue;
            }            
            elt.removeAttribute("transition");
            this.setSize(elt, size);
            elt.style.opacity = opacity;
        }
        await sleep(0);
    }

    async transitionClasses(fireID, classes, transitionType, size, opacity){
        let promises = [];
        for(let c of classes){
            let elt = this.classMap[c.uuid];
            if(!elt || elt.fireID !== fireID){
                continue;
            }
            elt.style.visibility = "";
            elt.setAttribute("transition", transitionType);
            this.setSize(elt, size);
            elt.style.opacity = opacity;
            if(c.isDisplayed()){
                promises.push(promiseFromDomEvent(elt, "transitionend"));
            }
        }
        await Promise.all(promises);
        for(let c of classes){
            let elt = this.classMap[c.uuid];
            if(!elt ||elt.fireID !== fireID){
                continue;
            }
            elt.removeAttribute("transition");
        }
    }
}

customElements.define('sseq-class-highlighter', ClassHighlighter);