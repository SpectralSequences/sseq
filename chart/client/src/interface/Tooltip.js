import {LitElement, html, css} from 'lit-element';
import { promiseFromDomEvent } from "./utils.js";

const MARGIN = 10;

export class Tooltip extends LitElement {
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

    static get styles() {
        return css `
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
            width : max-content;
        }
        
        :host([shown]) {
            opacity: 0.9;
        }

        :host([transition=show]) {
            transition : opacity 200ms ease-out;
        }
        
        :host([transition=hide]) {
            transition : opacity 500ms ease-in;
        }
        `;
    }


    constructor() {
        super();
        this._mouseover_class = this._mouseover_class.bind(this);
        this._mouseout_class = this._mouseout_class.bind(this);
        this._handle_redraw = this._handle_redraw.bind(this);
    }

    render(){
        return html`
            <slot></slot>
        `;
    }

    firstUpdated(changedProperties) {
        this.display = this.parentElement;
        this.display.addEventListener("mouseover-class", this._mouseover_class);
        this.display.addEventListener("mouseout-class", this._mouseout_class);
    }

    _mouseover_class(event){
        let [cls, mouseState] = event.detail;
        this.cls = cls;
        let sseq = this.display.sseq;
        let page = this.display.page;
        this.setHTML(sseq.getClassTooltipHTML(cls, page));
        this.show(this.cls._canvas_x, this.cls._canvas_y);
    }

    _mouseout_class(event){
        this.cls = undefined;
        this.hide();
    }

    _handle_redraw(){
        this.position(this.cls._canvas_x, this.cls._canvas_y);
    }

    setHTML(html) {
        this.innerHTML = html;
    }

    async show(x, y) {
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
        this.style.left = "0px";
        this.style.top = "0px";

        this.position(x, y);
        this.display.addEventListener("draw", this._handle_redraw);
        this.setAttribute("shown", "");
        this.setAttribute("transition", "show");
        await promiseFromDomEvent(this, "transitionend");
        this.removeAttribute("transition");
    }

    position(x, y){
        this.rect = this.getBoundingClientRect();
        this.canvasRect = this.display.canvas.getBoundingClientRect();
        this.displayRect = this.display.getBoundingClientRect();

        x = x + this.canvasRect.x;
        y = y + this.canvasRect.y;

        /**
         * By default, show the tooltip to the top and right of (x, y), offset
         * by MARGIN. If this causes the tooltip to leave the window, position
         * it to the bottom/left accordingly.
         */
        if (x + MARGIN + this.rect.width < this.displayRect.width){
            x = x + MARGIN;
        } else {
            x = x - this.rect.width - MARGIN;
        }
        
        if (y - this.rect.height - MARGIN > 0){
            y = y - this.rect.height - MARGIN;
        } else {
            y = y + MARGIN;
        }
        this.style.left = `${x}px`;
        this.style.top = `${y}px`;
    }

    async hide() {
        this.display.removeEventListener("draw", this._handle_redraw);
        this.removeAttribute("shown", "");
        this.setAttribute("transition", "hide");
        await promiseFromDomEvent(this, "transitionend");
        this.removeAttribute("transition");
    }
}

if(!customElements.get('sseq-tooltip')){
    customElements.define('sseq-tooltip', Tooltip);
}
