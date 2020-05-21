import {LitElement, html, css} from 'lit-element';

import { ButtonElement } from "./Button.js";

import { sleep, promiseFromDomEvent } from "./utils.js";

export class PopupElement extends LitElement {
    static get styles() {
        return css `
            :host {
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                opacity: 0;
                z-index: 9999;	
                overflow: auto;
                position: fixed;
                visibility: hidden;
                margin-top: -250px;

                pointer-events: none;
                overflow : hidden;
                /*-moz-transition: all .3s ease-in-out;
                -webkit-transition: all .3s ease-in-out;
                transition: all .3s ease-in-out;*/
            }

            :host([modal]){
                background: rgba(0,0,0,.8);
                
            }
            
            :host([open]) {
                opacity: 1;
                margin-top: 0px;
                visibility: visible;	
            }
            
            #body-footer {
                overflow: hidden;
            }

            #body-footer[transition=close] {
                transition: all 0.3s ease-in;
            }

            #body-footer[transition=open] {
                transition: all 0.3s ease-out;
            }

            #header {
                display : flex;
                background : var(--primary-2);
                --text-color : var(--primary-2-text);
                color : rgba(var(--text-color), var(--text-opacity));
            }

            #footer {
                flex-shrink : 1;
                display : flex;
            }

            #header-inner {
                flex-grow : 1;
                padding: 7px 0px 3px 15px;
                user-select: none;
            }

            #body {
                padding: 25px;
            }
            
            #content {
                /* width: 70%; */
                min-width: 250px;
                width: -moz-fit-content;
                width: fit-content;
                background: #FFF;
                background : var(--primary-4);
                --text-color : var(--primary-4-text);
                --text-opacity : 0.7;                
                max-width: 600px;
                position: relative;
                /* border-radius: 8px; */
                box-shadow: 0 0 6px rgba(0, 0, 0, 0.2);
                pointer-events: auto;
            }        

            ::slotted(*) {
                color : rgba(var(--text-color), var(--text-opacity));
            }

            .close-btn {
                font-size: var(--close-icon-font-size);
            }


            .draggable {
                cursor: grab; /* W3C standards syntax, all modern browser */
            }
            
            .draggable:active {
                cursor: grabbing;
            }
     
        `
    }

    static get properties() {
        return { 
            top : { type: Number },
            left : { type : Number },
            minimized : {type : Boolean}
        };
    }

    get open() {
        return this.hasAttribute('open');
    }
  
    set open(v) {
        if (v) {
            this.setAttribute('open', '');
        } else {
            this.removeAttribute('open');
        }
    }

    // get minimized() {
    //     return this.hasAttribute('minimized');
    // }
  
    // set minimized(v) {
    //     if (v) {
    //         this.setAttribute('minimized', '');
    //     } else {
    //         this.removeAttribute('minimized');
    //     }
    // }

    constructor(){
        super();
        this.show = this.show.bind(this);
        this.hide = this.hide.bind(this);
        this.startMove = this.startMove.bind(this);
        this.move = this.move.bind(this);
        this.endMove = this.endMove.bind(this);
        this.toggleMinimize = this.toggleMinimize.bind(this);
        this.top = 70;
        this.left = 70;
        this.minimized = false;
    }

    render(){
        return html`
            <div id="content" style="margin-top:${this.top}px; margin-left:${this.left}px;">
                <div id="header">
                    <div id="header-inner" class="draggable" @pointerdown=${this.startMove} @pointerup=${this.endMove}>
                        <slot name="header">
                            <h3>Popup | Modal | PURE CSS</h3>
                        </slot>
                    </div>
                    <sseq-button class="close-btn" @click=${this.toggleMinimize}> ${this.minimized ? "+" : html`&minus;`}</sseq-button>
                    <sseq-button class="close-btn" @click=${this.hide}>×</sseq-button>
                </div>
                <div id="body-footer">
                    <div id="body-footer-inner">
                        <div id="body">
                            <slot name="body"></slot>
                        </div>
                        <div id="footer">
                            <span style="flex-grow : 1;"></span>
                            <sseq-button style="margin-right: 0.75rem;">OK</sseq-button>
                            <sseq-button>CANCEL</sseq-button>
                        </div>
                    </div>
                </div>
            </div>
        `
    }

    firstUpdated(changedProperties) {
        let onContentResized = function onContentResized(_entries){
            if(!this.minimized){                
                let body_and_footer = this.shadowRoot.querySelector("#body-footer");
                let body_and_footer_inner = this.shadowRoot.querySelector("#body-footer-inner");
                body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
            }
        }.bind(this);
        this.resizeObserver = new ResizeObserver(onContentResized);
        this.resizeObserver.observe(this.shadowRoot.querySelector("#body-footer-inner"));
    }

    startMove(e){
        this.starting_mouse_x = e.pageX - this.left;
        this.starting_mouse_y = e.pageY - this.top;
        this.shadowRoot.querySelector("#header-inner").setPointerCapture(e.pointerId);
        this.addEventListener('pointermove', this.move);
    }

    move(e) {
        let boundingRect = this.getBoundingClientRect();
        let pageX = Math.max(Math.min(e.pageX, boundingRect.width), boundingRect.left);
        let pageY = Math.max(Math.min(e.pageY, boundingRect.height), boundingRect.top);
        this.left = pageX - this.starting_mouse_x;
        this.top = pageY - this.starting_mouse_y;
    }

    endMove(e) {
        this.shadowRoot.querySelector("#header-inner").releasePointerCapture(e.pointerId);
        this.removeEventListener('pointermove', this.move);
    } 

    async show(){
        await sleep(100);
        this.open = true;
    }

    hide(){
        this.open = false;
    }

    minimize() {
        this.setAttribute("minimized", true);
        this.minimized = true;
    }

    restore() {
        this.removeAttribute("minimized");
        this.minimized = false;
    }

    async toggleMinimize() {
        let body_and_footer = this.shadowRoot.querySelector("#body-footer");
        let body_and_footer_inner = this.shadowRoot.querySelector("#body-footer-inner");
        if(this.minimized){
            this.minimized = false;
            body_and_footer.setAttribute("transition", "open");
            body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
        } else {
            this.minimized = true;
            body_and_footer.setAttribute("transition", "close");
            body_and_footer.style.height = 0;            
        }
        await promiseFromDomEvent(body_and_footer, "transitionend");
        body_and_footer.removeAttribute("transition");
    }
}

customElements.define('sseq-popup', PopupElement);

// import {unsafeHTML} from 'lit-html/directives/unsafe-html.js';

// <!-- Start Modal -->

// <!-- End Modal -->
 
// <!-- Link in page to show modal on click-->
// <a href="#ft-demo-modal">Open modal</a>