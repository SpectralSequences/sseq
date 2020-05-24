import {LitElement, html, css} from 'lit-element';
import { styleMap } from 'lit-html/directives/style-map';


import { sleep, promiseFromDomEvent } from "./utils.js";
const RESIZER_WIDTH = 8;

export class Panel extends LitElement {
    static get properties() {
        return { 
            width : { type: Number },
            closed : { type : Boolean },
            displayedChildren : { type : Array }
        };
    }

    get closed() {
        return this.hasAttribute("closed");
    }

    set closed(v) {
        let oldValue = this.closed;
        if(v){
            this.setAttribute("closed", "");
        } else {
            this.removeAttribute("closed");
        }
        this.requestUpdate("closed", oldValue);
    }


    constructor(){
        super();
        this.hide = this.hide.bind(this);
        this.show = this.show.bind(this);
        this.toggle = this.toggle.bind(this);
        this._startResize = this._startResize.bind(this);
        this._resize = this._resize.bind(this);
        this._endResize = this._endResize.bind(this);
    }

    static get styles() {
        return css`
            :host {
                --sidebar-width-collapsed : 28.7px; 
            }

            [hidden] {
                display:none !important;
            }
            #divider {
                height : 100%;
                cursor : ew-resize;
                width : ${RESIZER_WIDTH}px;
                position : absolute;
                display:inline;
                z-index : 10000;
            }
            
            #sidebar {
                height: 100%;
                width : var(--sidebar-width);
                margin-left : ${RESIZER_WIDTH / 2}px;
                border-left: 1px solid #DDD;
                float:left; display:inline;
            }

            #sidebar[transition=open] {
                transition: 0.5s ease-out;
            }

            #sidebar[transition=close] {
                transition: 0.5s ease-in;
            }

            :host([closed]) #sidebar {
                --sidebar-translation : calc(var(--sidebar-width) - var(--sidebar-width-collapsed));
                transform : translateX(var(--sidebar-translation));
                margin-left : calc(-1 * var(--sidebar-translation));
            }

            #content {
                background : rgba(var(--body-background-color), 1); /*var(--body-background-opacity)*/
                color : rgba(--body-text-color, 1); /* Is 1 correct for opacity here? */
            }

            ::slotted(*) {
                --text-color : var(--body-text-color);
                color : rgba(var(--body-text-color), var(--body-text-opacity));
            }

            #header {
                background : rgba(var(--header-background-color), 1);
                color : rgba(var(--header-text-color), 1);
                font-size: 20pt;
                display : flex;
                flex-direction : column;
            }

            #btn-collapse {
                width : 28.7px;
                height: 26.9px;
            }

            #btn-collapse[open] {
                font-size: var(--close-icon-font-size);
            }
            
        `
    }

    async firstUpdated(){
        let slot = this.shadowRoot.querySelector("slot");
        slot.style.display = "none";
        this.width = parseFloat(this.getAttribute("initial-width")) || 240; // px
        this.minWidth = parseFloat(this.getAttribute("min-width")) || 200; // px
        this.maxWidth = parseFloat(this.getAttribute("max-width")) || 100000; // px
        await sleep(100);
        this.hideChildren();
        slot.style.display = "";
    }

    render(){
        let sidebar_styles  = { "--sidebar-width" : `${this.width}px` };
        // if(this.hidden){
        //     let translation = this.width - this.collapsedWidth;
        //     Object.assign(sidebar_styles, {
        //         transform : `translateX(${translation}px)`,
        //         marginLeft : `-${translation}px`
        //     });
        // }
        let content_styles = { width : "100%" };
        for(let key of ["display", "flexDirection", "flexWrap", "flexFlow", "justifyContent"]){
            if(this.style[key]){
                content_styles[key] = this.style[key];
            }
        }
        return html`
            <div id=divider 
                @pointerdown=${this._startResize}
                @pointerup=${this._endResize} 
                ?hidden="${this.closed}"
            ></div>
            <div id=sidebar style="${styleMap(sidebar_styles)}">
                <div style="display:flex; height:100%;">
                    <div id="header">
                        <sseq-button @click=${this.toggle} id="btn-collapse" ?open="${!this.closed}">
                            ${this.closed ? html`&#9776;` : html`&times;` }
                        </sseq-button>
                    </div>
                    <div id=content style="${styleMap(content_styles)}">
                        <slot></slot>
                    </div>
                </div>
            </div>
        `;
    }

    _startResize(e){
        e.preventDefault();
        window.addEventListener('pointermove', this._resize);
        this.shadowRoot.querySelector("#divider").setPointerCapture(e.pointerId);
    }

    _resize(e) {
        // e.preventDefault();
        this.width = Math.min(Math.max(this.getBoundingClientRect().right - e.pageX, this.minWidth), this.maxWidth);
    }

    _endResize(e) {
        window.removeEventListener('pointermove', this._resize);
        // This next line doesn't really seem to do anything. I don't understand setPointerCapture very well...
        this.shadowRoot.querySelector("#divider").releasePointerCapture(e.pointerId);
    }    

    async toggle(){
        let transition_direction = this.closed ? "open" : "close";
        let sidebar = this.shadowRoot.querySelector("#sidebar");
        sidebar.setAttribute("transition", transition_direction);
        this.closed = !this.closed;
        await promiseFromDomEvent(sidebar, "transitionend");
        sidebar.removeAttribute("transition");
    }

    hide(){
        this.closed = true;
    }

    show(){
        this.closed = false;
    }

    hideChildren(){
        for(let child of this.children){
            child.slot = "none";
        }
        this.displayedChildren = [];
        this.requestUpdate();
    }

    displayChildren(element){
        if(element.constructor === String){
            element = document.querySelectorAll(element);
        }
        if(element instanceof HTMLElement) {
            if(!this.displayedChildren.includes(element)){
                this.displayedChildren.push(element);
                element.slot = "";
                this.requestUpdate();
            }
            return
        }
        // try {// Maybe it's an array
            let elements = element;
            for(let element of elements){
                this.displayChildren(element);
            }
        // } catch(e) {
        //     throw TypeError("Expected argument to be an HTML element, a String selector, or a list of elements");
        // }
    }
}
customElements.define('sseq-panel', Panel);