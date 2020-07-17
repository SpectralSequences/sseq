import {LitElement, html, css} from 'lit-element';
import { sleep } from './utils';

export class CaretElement extends LitElement {
    static get styles() {
        return css`
            :host {
                background-color: white;
                position : absolute;
                left : calc(1ch * var(--pos-x));
                top : calc(var(--line-height) * var(--pos-y));
                height: var(--line-height);
                width: 1.5px;
            }
            
            :host([submitted]), :host([selection]){
                opacity : 0 !important;
                animation : unset !important;
            }

            :host([state="notblinking"]) {
                opacity : 0.99;
                transition-property : opacity;
                transition-duration : 0.2s;
            }

            :host([state="blinking"]) {
                animation-name: caret;
                animation-duration: 1.2s; 
                animation-timing-function: cubic-bezier(1,0,0,1); 
                animation-delay: 0;
                animation-direction: normal;
                animation-iteration-count: infinite;
                animation-fill-mode: none;
                animation-play-state: running;
            }

            @keyframes caret {
                0%, 49% {
                    background-color: black;
                    opacity : 0;
                }
                50%, 100% {
                    background-color: white;
                    opacity : 1;
                }
            }
        `;
    }

    get state(){
        return this.getAttribute("state");
    }

    set state(v){
        this.setAttribute("state", v);
    }

    get selectionHighlight(){
        return this.getAttribute("selection");
    }

    set selectionHighlight(v){
        this.toggleAttribute('selection', v);
    }

    get submitted(){
        return this.getAttribute("submitted");
    }

    set submitted(v){
        this.toggleAttribute('submitted', v);
    }

    get x(){
        return Number.parseInt(this.style.getPropertyValue("--pos-x"));
    }

    set x(v){
        this.style.setProperty("--pos-x", v);
    }

    get y(){
        return Number.parseInt(this.style.getPropertyValue("--pos-y"));
    }

    set y(v){
        this.style.setProperty("--pos-y", v);
    }

    firstUpdated(){
        this.state = "blinking";
        this.addEventListener("transitionend", () => {
            this.state = "blinking";
        });
    }

    hide(){
        this.submitted = true;
    }

    async setPosition(x, y){
        this.x = x;
        this.y = y;
        this.state = "";
        await sleep(20);
        this.state = "notblinking";
    }

    setLineHeight(height){
        this.style.setProperty("--line-height", height)
    }
}

customElements.define('repl-caret', CaretElement);