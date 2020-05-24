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
                background : rgba(var(--header-background-color));
                --text-color : var(--header-text-color);
                color : rgba(var(--text-color), var(--text-opacity));
            }

            #footer {
                flex-shrink : 1;
                display : flex;
            }

            sseq-button:focus {
                outline-style : solid;
                outline-offset : -2px;
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
                background : rgb(var(--body-background-color));
                --text-color : var(--body-text-color);
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
        this.focus = this.focus.bind(this);
        this.handleBodyClick = this.handleBodyClick.bind(this);
        this.show = this.show.bind(this);
        this.hide = this.hide.bind(this);
        this.startMove = this.startMove.bind(this);
        this.move = this.move.bind(this);
        this.endMove = this.endMove.bind(this);
        this.toggleMinimize = this.toggleMinimize.bind(this);
        this.ok = this.ok.bind(this);
        this.cancel = this.cancel.bind(this);
        this.top = 70;
        this.left = 70;
        this.minimized = false;
    }

    firstUpdated(changedProperties) {
        let onContentResized = async function onContentResized(_entries){
            if(!this.minimized){                
                let body_and_footer = this.shadowRoot.querySelector("#body-footer");
                let body_and_footer_inner = this.shadowRoot.querySelector("#body-footer-inner");
                await sleep(100);
                body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
            }
        }.bind(this);
        this.resizeObserver = new ResizeObserver(onContentResized);
        this.resizeObserver.observe(this.shadowRoot.querySelector("#body-footer-inner"));
        this.addEventListener("keydown", (e) => {
            if(e.key === "Escape"){
                this.cancel();
            }
        });
        this.addEventListener("interact-toggle", (e) => {
            e.stopPropagation();
            this.submit(e);
        });
        this.addEventListener("interact-submit", (e) => {
            e.stopPropagation();
            this.submit(e);
        });
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
                    <sseq-button class="close-btn" @click=${this.cancel}>×</sseq-button>
                </div>
                <div id="body-footer">
                    <div id="body-footer-inner" @click=${this.handleBodyClick}>
                        <div id="body">
                            <slot name="body"></slot>
                        </div>
                        <div id="footer">
                            <span @click=${this.focus} style="flex-grow : 1;"></span>
                            <sseq-button id=ok @click=${this.ok} style="margin-right: 0.75rem; ">OK</sseq-button>
                            <sseq-button id=cancel @click=${this.cancel}>CANCEL</sseq-button>
                        </div>
                    </div>
                </div>
            </div>
        `
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
        for(let elt of document.querySelectorAll("sseq-popup")){
            elt.cancel(elt !== this);
        }
        // Without this sleep(0) the height of the popup behaves inconsistently (window size will change by +/- 3px). Don't remove it!
        // Also important that cancel happens first, because immediately after using show() we are often going to await on the 
        // result of this popup. If we sleep(0) then the cancel here will be picked up as the result of the popup.
        await sleep(0); 
        let okbtn = this.shadowRoot.querySelector("#ok");
        okbtn.saveState = okbtn.enabled;
        this.restore();
        this.focus();
        // It's slightly inconsistent about focusing the button for some reason, 
        // so just to be sure, do it again after 100ms.
        sleep(100).then(() => this.focus());        
        this.triggerElement = document.activeElement;
        this.open = true;
        return this;
    }

    hide(){
        if(this.triggerElement){
            this.triggerElement.focus();
            delete this.triggerElement;
        }
        this.open = false;
        return this;
    }

    get okEnabled(){
        return this.shadowRoot.querySelector("#ok").enabled;
    }

    set okEnabled(v){
        this.shadowRoot.querySelector("#ok").enabled = v;
    }

    handleBodyClick(){
        // If the clicked object wouldn't otherwise be focused, focus the okay button if possible.
        if(document.activeElement === document.body){
            this.focus();
        }
    }

    focus(){
        let focusElt = this.querySelector("[focus]");
        if(focusElt){
            focusElt.focus();
        } else if(this.okEnabled){
            this.shadowRoot.querySelector("#ok").focus();
        } else {
            this.shadowRoot.querySelector("#cancel").focus();
        }
        return this;
    }

    ok(){
        if(!this.okEnabled){
            return;
        }
        this.dispatchEvent(new CustomEvent("submit", { detail : true }));
        this._submitPromise = null;
        this.hide();
    }

    cancel(hide = true){
        this.dispatchEvent(new CustomEvent("submit", { detail : false }));
        this._submitPromise = null;
        if(hide){
            this.hide();
        }
    }

    submit(e){
        let elt = this.shadowRoot.activeElement;
        if(elt && elt.submit){
            elt.submit(e);
        } else if(elt && elt.nodeName.toLowerCase() === "input"){
            this.ok();
        } else if(elt){
            elt.click();
        } else {
            this.ok();
        }
    }

    submited(){
        if(!this._submitPromise){
            this._submitPromise = promiseFromDomEvent(this, "submit").then((e) => {
                return e.detail;
            });
        }
        return this._submitPromise;
    }

    async minimize() {
        let body_and_footer = this.shadowRoot.querySelector("#body-footer");
        let body_and_footer_inner = this.shadowRoot.querySelector("#body-footer-inner");
        this.minimized = true;
        body_and_footer.setAttribute("transition", "close");
        body_and_footer.style.height = 0;     
        await promiseFromDomEvent(body_and_footer, "transitionend");
        body_and_footer.removeAttribute("transition");
        for(let btn of this.shadowRoot.querySelector("#body-footer").querySelectorAll("sseq-button")){
            btn.saveState = btn.enabled;
            btn.enabled = false;
        }
    }

    async restore(animate = false) {
        let body_and_footer = this.shadowRoot.querySelector("#body-footer");
        let body_and_footer_inner = this.shadowRoot.querySelector("#body-footer-inner");
        this.minimized = false;
        body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
        for(let btn of this.shadowRoot.querySelector("#body-footer").querySelectorAll("sseq-button")){
            if(btn.saveState !== undefined){
                btn.enabled = btn.saveState;
            } else {
                btn.enabled = true;
            }            
        }    
        if(animate){
            body_and_footer.setAttribute("transition", "open");
            await promiseFromDomEvent(body_and_footer, "transitionend");
            body_and_footer.removeAttribute("transition");
        }
    }

    async toggleMinimize() {
        if(this.minimized){
            await this.restore();
        } else {
            await this.minimize();
        }
    }
}

customElements.define('sseq-popup', PopupElement);

// import {unsafeHTML} from 'lit-html/directives/unsafe-html.js';

// <!-- Start Modal -->

// <!-- End Modal -->
 
// <!-- Link in page to show modal on click-->
// <a href="#ft-demo-modal">Open modal</a>