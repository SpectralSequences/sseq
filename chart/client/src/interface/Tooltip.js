"use strict"
const MARGIN = 10;

export class Tooltip extends HTMLElement {
    static toTooltipString(obj, page) {
        if (!obj) {
            return false;
        }

        if(obj.constructor === String){
            return obj;
        }

        if(obj.constructor === Array) {
            return obj.map((x) => Tooltip.toTooltipString(x, page)).filter((x) => x).join("\n");
        }

        if(obj.constructor === Map){
            let lastkey;
            for (let k of obj.keys()) {
                if (k > page) {
                    break;
                }
                lastkey = k;
            }
            return Tooltip.toTooltipString(obj.get(lastkey));
        }

        return false;
    }

    constructor() {
        super();
        this._mouseover_class = this._mouseover_class.bind(this);
        this._mouseout_class = this._mouseout_class.bind(this);
        this.attachShadow({mode: 'open'});
        let style = document.createElement("style");
        style.innerText = `
            :host {
                position: absolute;
                z-index: 999999;
                transition: opacity 500ms ease 0s;             
                text-align: center;
                padding: 5px;
                font: 12px sans-serif;
                background: lightsteelblue;
                border: 0px;
                border-radius: 8px;
                pointer-events: none;
                opacity : 0;
            }`;
        this.shadowRoot.appendChild(style);
        this.showTransitionTime = "200ms";
        this.hideTransitionTime = "500ms";
        let slot = document.createElement("slot");
        this.shadowRoot.appendChild(slot);
    }

    connectedCallback(){
        this.display = this.parentElement;
        this.display.addEventListener("mouseover-class", this._mouseover_class);
        this.display.addEventListener("mouseout-class", this._mouseout_class);
    }

    disconnectedCallback(){
        this.display.removeEventListener("_mouseover_class");
        this.display.removeEventListener("_mouseout_class");
    }

    _mouseover_class(event){
        let sseq = this.display.sseq;
        let [cls, mouse_state] = event.detail;
        let page = this.display.page;
        this.setHTML(sseq.getClassTooltipHTML(cls, page));
        this.show(mouse_state.screen_lattice_x, mouse_state.screen_lattice_y);
    }

    _mouseout_class(event){
        this.hide();
    }

    setHTML(html) {
        this.innerHTML = html;
    }

    show(x, y) {
        /**
         * Reset the tooltip position. This prevents a bug that occurs when the
         * previously displayed tooltip is positioned near the edge (but still
         * positioned to the right of the node), and the new tooltip text is
         * longer than the previous tooltip text. This may cause the new
         * (undisplayed) tooltip text to wrap, which gives an incorrect value
         * of rect.width and rect.height. The bug also occurs after resizing,
         * where the location of the previous tooltip is now outside of the
         * window.
         */
        this.style.cssText = this.style.cssText;
        this.style.left = "0px";
        this.style.top = "0px";

        let rect = this.getBoundingClientRect();
        let canvasRect = this.display.canvas.getBoundingClientRect();

        x = x + canvasRect.x;
        y = y + canvasRect.y;

        /**
         * By default, show the tooltip to the top and right of (x, y), offset
         * by MARGIN. If this causes the tooltip to leave the window, position
         * it to the bottom/left accordingly.
         */
        if (x + MARGIN + rect.width < window.innerWidth){
            x = x + MARGIN;
        } else {
            x = x - rect.width - MARGIN;
        }
        
        if (y - rect.height - MARGIN > 0){
            y = y - rect.height - MARGIN;
        } else {
            y = y + MARGIN;
        }
        this.style.left = `${x}px`;
        this.style.top = `${y}px`;

        this.style.transition = `opacity ${this.showTransitionTime}`;
        this.style.opacity = 0.9;
    }

    hide() {
        this.style.transition = `opacity ${this.hideTransitionTime}`;
        this.style.opacity = 0;
    }
}

if(!customElements.get('sseq-tooltip')){
    customElements.define('sseq-tooltip', Tooltip);
}
