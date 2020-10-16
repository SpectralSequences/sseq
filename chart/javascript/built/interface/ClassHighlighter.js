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
export class ClassHighlighterElement extends LitElement {
    static get styles() {
        return css `
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
    constructor() {
        super();
        this.numElements = 0;
        this.classMap = {};
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
    }
    firstUpdated() {
        this.disp = this.closest("sseq-display");
        this.chart = this.closest("sseq-chart");
        this.disp.addEventListener("scale-update", this.handleScaleUpdate);
    }
    render() {
        return html `
            ${Array(this.numElements).fill().map(() => html `<span></span>`)}
        `;
    }
    handleScaleUpdate() {
        for (let elt of Array.from(this.shadowRoot.children).filter(elt => elt.cls)) {
            this.setSize(elt);
        }
    }
    setSize(elt, size) {
        if (elt.cls === undefined) {
            return;
        }
        // In case cls is out of date, look up the current class with same uuid
        // Once in a while this out of dateness is caused by chart.class.update in sseqsocketlistener
        // TODO: can we change design so this isn't needed? I don't understand why identity of cls changes...
        elt.cls = this.chart.sseq.classes[elt.cls.uuid];
        if (elt.cls === undefined) {
            elt.style.display = "none";
            return;
        }
        if (elt.cls.isDisplayed()) {
            elt.style.display = "";
        }
        else {
            elt.style.display = "none";
        }
        let x = elt.cls._canvas_x;
        let y = elt.cls._canvas_y;
        if (size !== undefined) {
            elt.size = size;
        }
        size = elt.size;
        elt.style.left = `${x - size / 2}px`;
        elt.style.top = `${y - size / 2}px`;
        elt.style.height = `${size}px`;
        elt.style.width = `${size}px`;
        ;
    }
    clearClass(cls) {
        return __awaiter(this, void 0, void 0, function* () {
            if (!cls) {
                return;
            }
            let elt = this.classMap[cls.uuid];
            if (elt === undefined) {
                return;
            }
            elt.style.removeProperty("--transition-time");
            elt.removeAttribute("transition");
            this.setSize(elt, 0);
            delete this.classMap[elt.cls.uuid];
            elt.cls = undefined;
            yield sleep(30);
            delete elt.fireID;
        });
    }
    clear() {
        return __awaiter(this, void 0, void 0, function* () {
            let promises = [];
            for (let elt of this.shadowRoot.children) {
                if (elt.cls) {
                    promises.push(this.clearClass(elt.cls));
                }
            }
            yield Promise.all(promises);
        });
    }
    clearClasses(classes) {
        return __awaiter(this, void 0, void 0, function* () {
            let promises = [];
            for (let cls of classes) {
                promises.push(this.clearClass(cls));
            }
            yield Promise.all(promises);
        });
    }
    allocateClasses(fireID, classes) {
        return __awaiter(this, void 0, void 0, function* () {
            let availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.fireID);
            let numElementsNeeded = classes.filter(c => !(c.uuid in this.classMap)).length;
            if (numElementsNeeded > availableElements.length) {
                this.numElements += numElementsNeeded - availableElements.length;
                this.requestUpdate();
                yield sleep(10);
                availableElements = Array.from(this.shadowRoot.children).filter((elt) => !elt.fireID);
            }
            for (let c of classes) {
                if (c.uuid in this.classMap) {
                    this.classMap[c.uuid].fireID = fireID;
                    continue;
                }
                let elt = availableElements.pop();
                this.classMap[c.uuid] = elt;
                elt.cls = c;
                elt.fireID = fireID;
            }
        });
    }
    highlight(classes) {
        return __awaiter(this, void 0, void 0, function* () {
            if (classes.constructor != Array) {
                classes = [classes];
            }
            if (classes.length === 0) {
                return;
            }
            let fireID = Math.random();
            yield this.allocateClasses(fireID, classes);
            yield this.prepareElements(fireID, classes, 15, 0.7);
            // await this.transitionClasses(fireID, classes, "none", 15, 0.7);
        });
    }
    fire(classes) {
        return __awaiter(this, void 0, void 0, function* () {
            if (classes.constructor != Array) {
                classes = [classes];
            }
            if (classes.length === 0) {
                return;
            }
            let opacity = 0.7;
            let fireID = Math.random();
            // console.log("fire")
            // console.log("   allocate");
            yield this.allocateClasses(fireID, classes);
            // console.log("   prepare");
            yield this.prepareElements(fireID, classes, 0, opacity);
            yield sleep(30);
            // console.log("   transition");
            yield this.transitionClasses(fireID, classes, "fire", 15, opacity);
            // console.log("fire completed");
        });
    }
    hideClasses(classes) {
        return __awaiter(this, void 0, void 0, function* () {
            let fireID = Math.random();
            // console.log("hide")
            // console.log("   allocate");        
            yield this.allocateClasses(fireID, classes);
            // console.log("   transition");
            yield this.transitionClasses(fireID, classes, "hide", 15, 0);
            // console.log("   clear");
            yield this.clearClasses(classes);
            // console.log("hide completed")
        });
    }
    prepareElements(fireID, classes, size, opacity) {
        return __awaiter(this, void 0, void 0, function* () {
            for (let c of classes) {
                let elt = this.classMap[c.uuid];
                if (!elt || elt.fireID !== fireID) {
                    continue;
                }
                elt.removeAttribute("transition");
                this.setSize(elt, size);
                elt.style.opacity = opacity;
            }
            yield sleep(0);
        });
    }
    transitionClasses(fireID, classes, transitionType, size, opacity) {
        return __awaiter(this, void 0, void 0, function* () {
            let promises = [];
            for (let c of classes) {
                let elt = this.classMap[c.uuid];
                if (!elt || elt.fireID !== fireID) {
                    continue;
                }
                elt.style.visibility = "";
                elt.setAttribute("transition", transitionType);
                this.setSize(elt, size);
                elt.style.opacity = opacity;
                if (c.isDisplayed()) {
                    promises.push(promiseFromDomEvent(elt, "transitionend"));
                }
            }
            yield Promise.all(promises);
            for (let c of classes) {
                let elt = this.classMap[c.uuid];
                if (!elt || elt.fireID !== fireID) {
                    continue;
                }
                elt.removeAttribute("transition");
            }
        });
    }
}
customElements.define('sseq-class-highlighter', ClassHighlighterElement);
