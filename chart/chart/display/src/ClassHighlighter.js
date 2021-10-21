import {LitElement, html, css} from 'lit-element';
import { sleep, promiseFromDomEvent, findAncestorElement } from "./utils.js";

export class ClassHighlighterElement extends LitElement {
    static get styles() {
        return css`
            .overflow-hider {
                overflow : hidden;
                position : absolute;
                z-index : -5;
            }

            span {
                position: absolute; 
                background-color: orange; 
                border-radius: 50%; 
                height: 0px; 
                width: 0px;
                --transition-time : 0.3s;
                transition : none;
                transition-property: height,width,margin-left, margin-top; /* Don't do left or top */
                z-index : -100;
            }

            span[transition=fire] {
                transition-timing-function: cubic-bezier(0,.27,1,5);
                transition-duration : var(--transition-time);
            }

            span[transition=show] {
                transition-timing-function: ease;
                transition-duration : var(--transition-time);
            }

            span[transition=hide] {
                transition-timing-function: ease;
                transition-duration : var(--transition-time);
            }
        `;
    }

    constructor(){
        super();
        this.numElements = 0;
        this.classMap = {};
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
        this.handleMarginUpdate = this.handleMarginUpdate.bind(this);
    }

    firstUpdated(){
        this.disp = this.closest("sseq-chart");
        this.disp.addEventListener("scale-update", this.handleScaleUpdate);
        this.disp.addEventListener("margin-update", this.handleMarginUpdate)
        this.handleMarginUpdate();
    }

    render() {
        return html`
            <div class=overflow-hider>
            ${Array(this.numElements).fill().map(() => html`<span></span>`)}
            </div>
        `;
    }

    handleMarginUpdate(){
        let overflowHider = this.shadowRoot.querySelector(".overflow-hider");
        overflowHider.style.left = `${this.disp.leftMargin}px`;
        overflowHider.style.right = `${this.disp.rightMargin}px`;
        overflowHider.style.top = `${this.disp.topMargin}px`;
        overflowHider.style.bottom = `${this.disp.bottomMargin}px`;
    }

    highlighterElements(){
        return this.shadowRoot.querySelectorAll("span");
    }

    filterHighlighterElements(callback){
        return Array.from(this.highlighterElements()).filter(callback);
    }


    handleScaleUpdate(){
        for(let elt of this.filterHighlighterElements(elt => elt.cls)){
            if(elt.hasAttribute("transition")){
                this.updatePosition(elt)
            } else {
                this.setSize(elt);
            }
        }
    }

    updatePosition(elt){
        let [x, y] = this.disp.getClassPosition(elt.cls);
        elt.style.left = `${x - this.disp.leftMargin}px`;
        elt.style.top = `${y - this.disp.topMargin}px`;
    }

    setSize(elt, size){
        if(elt.cls === undefined){
            return;
        }
        // In case cls is out of date, look up the current class with same uuid
        // Once in a while this out of dateness is caused by chart.class.update in sseqsocketlistener
        // TODO: can we change design so this isn't needed? I don't understand why identity of cls changes...
        // elt.cls = this.disp.sseq.classes[elt.cls.uuid];
        if(elt.cls === undefined){
            elt.style.display = "none";
            return;
        }
        if(this.disp.isClassVisible(elt.cls)){
            elt.style.display = "";
        } else {
            elt.style.display = "none";
        }
        
        if(size !== undefined){
            elt.size = size;
        }
        size = elt.size;
        size = this.disp.classOuterRadius(elt.cls) * 2 * size;
        elt.style.marginLeft = `${- size/2}px`
        elt.style.marginTop = `${- size/2}px`
        elt.style.height = `${size}px`;
        elt.style.width = `${size}px`;
        this.updatePosition(elt);
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

    async clearClasses(classes){
        let promises = [];
        for(let cls of classes){
            promises.push(this.clearClass(cls));
        }
        await Promise.all(promises);
    }

    async clear(){
        let promises = [];
        for(let elt of this.highlighterElements()){
            if(elt.cls) {
                promises.push(this.clearClass(elt.cls));
            }
        }
        await Promise.all(promises);
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
        await this.prepareElements(fireID, classes, 1.5, 0.7);
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
        await this.allocateClasses(fireID, classes);
        await this.prepareElements(fireID, classes, 0, opacity);
        await sleep(30);
        await this.transitionClasses(fireID, classes, "fire", 1.5, opacity);
    }

    async hideClasses(classes){
        let fireID = Math.random();
        await this.allocateClasses(fireID, classes);
        await this.transitionClasses(fireID, classes, "hide", 1.5, 0);
        await this.clearClasses(classes);
    }

    async allocateClasses(fireID, classes){
        let availableElements = this.filterHighlighterElements((elt) => !elt.fireID);
        let numElementsNeeded = classes.filter(c => !(c.uuid in this.classMap)).length;
        if(numElementsNeeded > availableElements.length){
            this.numElements += numElementsNeeded - availableElements.length;
            this.requestUpdate();
            await sleep(10);
            availableElements = this.filterHighlighterElements((elt) => !elt.fireID);
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
            if(this.disp.isClassVisible(c)){
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
        await sleep(0);
        for(let c of classes){
            let elt = this.classMap[c.uuid];
            if(!elt ||elt.fireID !== fireID){
                continue;
            }
            this.setSize(elt, size);
        }        
    }
}

customElements.define('sseq-class-highlighter', ClassHighlighterElement);