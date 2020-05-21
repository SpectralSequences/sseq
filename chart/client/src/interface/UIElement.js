// import {LitElement, html, css} from 'lit-element';

import { sleep, promiseFromDomEvent } from "./utils.js";

export class UIElement extends HTMLElement {
    constructor(){
        super();
        this.attachShadow({mode: 'open'});
        this.shadowRoot.innerHTML = `
            <style>
                :host {
                    height: 100vh; 
                    width: 100vw; 
                    display: flex;
                }

                :host([transition]) {
                    transition : 0.5s ease;
                }
                
                #loading {
                    position : absolute;
                    left: 20px;
                    top: 20px;
                    font-size : 16pt;
                }
            </style>            
            <slot name=loading> 
                <div id=loading>
                Loading...
                </div>
            </slot>
        `;
    }

    async start(){
        // return;
        let slot = this.shadowRoot.querySelector("slot");
        slot.innerHTML = "";
        slot.name = "";
        this.style.opacity = 0;
        for(let e of this.children){
            if(e.start){
                e.start();
            }
        }
        await sleep(0);
        this.setAttribute("transition", "");
        this.removeAttribute("style");
        await promiseFromDomEvent(this, "transitionend");
        this.removeAttribute("transition");
    }

}
customElements.define('sseq-ui', UIElement);