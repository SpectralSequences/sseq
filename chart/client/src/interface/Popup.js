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
                overflow : hidden;
                /*-moz-transition: all .3s ease-in-out;
                -webkit-transition: all .3s ease-in-out;
                transition: all .3s ease-in-out;*/
            }

            :host(:not([modal])) {
                pointer-events: none;
            }

            :host([modal]){
                background: rgba(0,0,0,.8);
            }

            :host([modal][open]:not(:focus-within)) {
                opacity: 0.999;
                transition: opacity 0.01s;
            }

            :host([open]:not(:focus-within)) {
                opacity: 0.8;
                transition: opacity 0.01s;
            }            
            
            :host([open]) {
                opacity: 1;
                margin-top: 0px;
                visibility: visible;	
            }
            
            .body-footer {
                overflow: hidden;
            }

            .body-footer[transition=close] {
                transition: all 0.3s ease-in;
            }

            .body-footer[transition=open] {
                transition: all 0.3s ease-out;
            }

            .header {
                display : flex;
                background : rgba(var(--header-background-color));
                --text-color : var(--header-text-color);
                color : rgba(var(--text-color), var(--text-opacity));
            }

            .footer {
                flex-shrink : 1;
                display : flex;
            }

            sseq-button:focus {
                outline-style : solid;
                outline-offset : -2px;
            }

            .header-inner {
                flex-grow : 1;
                padding: 7px 0px 3px 15px;
                user-select: none;
            }

            .body {
                overflow : auto;
                padding: 25px;
            }
            
            .content {
              /* width: 70%; */
                min-width: 250px;
                width: -moz-fit-content;
                width: fit-content;
                background: #FFF;
                background : rgb(var(--body-background-color));
                --text-color : var(--body-text-color);
                --text-opacity : 0.7;                
                /*max-width: 600px;*/
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
                cursor: grab;
            }
            
            .draggable:active {
                cursor: grabbing;
            }
        `
    }

    static get properties() {
        return { 
            minimized : {type : Boolean}
        };
    }

    get modal(){
        return this.hasAttribute("modal");
    }

    get top(){
        return this.getAttribute("top");
    }

    set top(v){
        this.setAttribute("top", v);
        this.requestUpdate();
    }

    get left(){
        return this.getAttribute("left");
    }

    set left(v){
        this.setAttribute("left", v);
        this.requestUpdate();
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

    get height() {
        return this.getAttribute('height');
    }
  
    set height(v) {
        this.setAttribute('height', v);
        this.requestUpdate();
    }

    get width() {
        return this.getAttribute('width');
    }
  
    set width(v) {
        this.setAttribute('width', v);
        this.requestUpdate();
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
        this.minimized = false;
    }

    firstUpdated(changedProperties) {
        if(!this.top){
            this.top = 70;
        }
        if(!this.left){
            this.left = 200;        
        }
        this.setAttribute("tabindex", "-1");
        this.addEventListener("click", () => {
            if(document.activeElement === this && !this.shadowRoot.activeElement){
                this.focus();
            }
        });
        this.addEventListener("focus", () => {
            if(document.activeElement === this && !this.shadowRoot.activeElement){
                this.focus();
            }
        });        
        let onContentResized = async function onContentResized(_entries){
            if(!this.minimized){                
                let body_and_footer = this.shadowRoot.querySelector(".body-footer");
                let body_and_footer_inner = this.shadowRoot.querySelector(".body-footer-inner");
                await sleep(100);
                body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
            }
        }.bind(this);
        this.resizeObserver = new ResizeObserver(onContentResized);
        this.resizeObserver.observe(this.shadowRoot.querySelector(".body-footer-inner"));
        this.addEventListener("interact-cancel", (e) => {
            this.cancel();
        });
        this.addEventListener("interact-toggle", (e) => {
            e.stopPropagation();
            this.submit(e);
        });
        this.addEventListener("interact-submit", (e) => {
            e.stopPropagation();
            this.submit(e);
        });
        this.addEventListener('transitionend', (e) => {
            if(getComputedStyle(this).opacity === "0.999"){
                this.focus();
            }
        });
    }

    updated(changedProperties) {
        let header = this.shadowRoot.querySelector(".header");
        let body = this.shadowRoot.querySelector(".body");
        let footer = this.shadowRoot.querySelector(".footer");
        let test_div = document.createElement("div");
        body.style.width = this.width;
        if(this.height){
            test_div.style.height = this.height;
            document.body.appendChild(test_div);
            let test_height = getComputedStyle(test_div).height;
            console.log("height:",this.height, "test height:", test_height, "header height:",getComputedStyle(header).height);
            body.style.height = `calc(${test_height} - ${getComputedStyle(header).height} - ${getComputedStyle(footer).height})`;
        }
    }


    render(){
        return html`
            <div class="content" style="margin-top:${this.top}px; margin-left:${this.left}px;">
                <div class="header">
                    <div class="header-inner">
                        <slot name="header">
                            <h3>Popup | Modal | PURE CSS</h3>
                        </slot>
                    </div>
                    ${ 
                        !this.modal 
                        ? html`
                            <sseq-button class="close-btn" @click=${this.toggleMinimize}> 
                            ${this.minimized ? "+" : html`&minus;`}
                            </sseq-button>` 
                        : ""
                    }
                    <sseq-button class="close-btn" @click=${this.cancel}>×</sseq-button>
                </div>
                <div class="body-footer">
                    <div class="body-footer-inner" @click=${this.handleBodyClick}>
                        <div class="body">
                            <slot name="body"></slot>
                        </div>
                        <div class="footer">
                            <span @click=${this.focus} style="flex-grow : 1;"></span>
                            <slot name="buttons">
                                <sseq-button class="ok" @click=${this.ok} style="margin-right: 0.75rem; ">OK</sseq-button>
                                <sseq-button class="cancel" @click=${this.cancel}>CANCEL</sseq-button>
                            </slot>
                        </div>
                    </div>
                </div>
            </div>
        `
    }



    startMove(e){
        this.starting_mouse_x = e.pageX - this.left;
        this.starting_mouse_y = e.pageY - this.top;
        this.shadowRoot.querySelector(".header-inner").setPointerCapture(e.pointerId);
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
        this.shadowRoot.querySelector(".header-inner").releasePointerCapture(e.pointerId);
        this.removeEventListener('pointermove', this.move);
    } 

    async show(){
        // Without this sleep(0) the height of the popup behaves inconsistently (window size will change by +/- 3px). Don't remove it!
        // Also important that cancel happens first, because immediately after using show() we are often going to await on the 
        // result of this popup. If we sleep(0) then the cancel here will be picked up as the result of the popup.
        await sleep(0); 
        let okbtn = this.shadowRoot.querySelector(".ok");
        okbtn.saveState = okbtn.enabled;
        let header = this.shadowRoot.querySelector(".header-inner");
        header.classList.toggle("draggable", !this.modal);
        if(this.modal){
            header.removeEventListener("pointerdown", this.startMove);
            header.removeEventListener("pointerup", this.endMove);            
        } else {
            header.addEventListener("pointerdown", this.startMove);
            header.addEventListener("pointerup", this.endMove);
        }
        this.restore();
        this.focus();
        // It's slightly inconsistent about focusing the button for some reason, 
        // so just to be sure, do it again after 100ms.
        sleep(100).then(() => this.focus());        
        this.triggerElement = document.activeElement;
        this.open = true;
        for(let e of this.querySelectorAll(".cancel")){
            e.addEventListener("click", this.cancel);
        }
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
        return this.shadowRoot.querySelector(".ok").enabled;
    }

    set okEnabled(v){
        this.shadowRoot.querySelector(".ok").enabled = v;
    }

    handleBodyClick(){
        // If the clicked object wouldn't otherwise be focused, 
        if(!document.activeElement || !document.activeElement.closest("sseq-popup")){
            this.focus();
        }
    }

    focus(){
        let focusElt = this.querySelector("[focus]");
        if(focusElt){
            let found = true;
            while(focusElt.shadowRoot && found){
                found = false;
                for(let elt of focusElt.shadowRoot.querySelectorAll("[focus]")){
                    if(elt.shadowRoot || elt.tabIndex === 0){
                        focusElt = elt;
                        found = !!elt.shadowRoot;
                        break;
                    }
                }
            }
        }
        focusElt = 
            focusElt
            || this.querySelector(".ok") 
            || this.shadowRoot.querySelector(".ok")
            || this.querySelector(".cancel")
            || this.shadowRoot.querySelector(".cancel");
        if(focusElt.clientWidth === 0){
            focusElt = this.querySelector(".cancel");
        }
        if(focusElt){
            focusElt.focus();
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
        let body_and_footer = this.shadowRoot.querySelector(".body-footer");
        let body_and_footer_inner = this.shadowRoot.querySelector(".body-footer-inner");
        this.minimized = true;
        body_and_footer.setAttribute("transition", "close");
        body_and_footer.style.height = 0;     
        await promiseFromDomEvent(body_and_footer, "transitionend");
        body_and_footer.removeAttribute("transition");
        for(let btn of this.shadowRoot.querySelector(".body-footer").querySelectorAll("sseq-button")){
            btn.saveState = btn.enabled;
            btn.enabled = false;
        }
    }

    async restore(animate = false) {
        let body_and_footer = this.shadowRoot.querySelector(".body-footer");
        let body_and_footer_inner = this.shadowRoot.querySelector(".body-footer-inner");
        this.minimized = false;
        body_and_footer.style.height = `${body_and_footer_inner.clientHeight}px`;
        for(let btn of this.shadowRoot.querySelector(".body-footer").querySelectorAll("sseq-button")){
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
            await this.restore(true);
        } else {
            await this.minimize();
        }
    }
}

customElements.define('sseq-popup', PopupElement);