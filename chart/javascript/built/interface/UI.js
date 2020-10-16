// import {LitElement, html, css} from 'lit-element';
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import Mousetrap from "mousetrap";
import { sleep, promiseFromDomEvent } from "./utils.js";
export class UIElement extends HTMLElement {
    constructor() {
        super();
        this.mousetrap = new Mousetrap(this);
        this._stopKeypressCallback = this._stopKeypressCallback.bind(this);
        this._handleKeyEvent = this._handleKeyEvent.bind(this);
        this._setupKeyBindings();
        this.attachShadow({ mode: 'open' });
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
    start() {
        return __awaiter(this, void 0, void 0, function* () {
            // return;
            let slot = this.shadowRoot.querySelector("slot");
            slot.innerHTML = "";
            slot.name = "";
            this.style.opacity = 0;
            for (let e of this.children) {
                if (e.start) {
                    e.start();
                }
            }
            yield sleep(0);
            this.setAttribute("transition", "");
            this.removeAttribute("style");
            yield promiseFromDomEvent(this, "transitionend");
            this.removeAttribute("transition");
            this.focus();
            this.dispatchEvent(new CustomEvent("started"));
        });
    }
    _stopKeypressCallback(e, element, combo) {
        // Find the correct target of the event inside the shadow dom
        while (element.shadowRoot && element.shadowRoot.activeElement) {
            element = element.shadowRoot.activeElement;
        }
        // if the element has the class "mousetrap" then no need to stop
        if (element.matches(".mousetrap")) {
            return false;
        }
        // Is the key printable?
        let keyCode = e.keyCode;
        let printable = (keyCode >= 48 && keyCode < 58) // number keys
            || keyCode == 32 // space
            || (keyCode >= 35 && keyCode < 41) //Home, End, Arrow keys (okay technically not printable but they do things in text boxes)
            || (keyCode >= 65 && keyCode < 91) // letter keys
            || (keyCode >= 96 && keyCode < 112) // numpad keys
            || (keyCode >= 186 && keyCode < 193) // ;=,-./` (in order)
            || (keyCode >= 219 && keyCode < 223) // [\]' (in order
            || ["t", "z", "+", "-"].includes(e.key) // Why does this need to be here?
            || e.code.startsWith("Digit") // Why does this need to be here?
        ;
        // console.log(e.key, e.code, e.keyCode, e);
        // Is the element a text input?
        let in_text_input = element.matches("input, select, textarea") || (element.contentEditable && element.contentEditable == 'true');
        return printable && in_text_input;
    }
    _setupKeyBindings() {
        Mousetrap.prototype.stopCallback = this._stopKeypressCallback;
        let [handleEnterDown, handleEnterUp] = this.getEventHandler("interact-submit");
        let [handleSpaceDown, handleSpaceUp] = this.getEventHandler("interact-toggle");
        let [handleEscapeDown, handleEscapeUp] = this.getEventHandler("interact-cancel");
        this.mousetrap.bind("enter", handleEnterDown, "keydown");
        this.mousetrap.bind("enter", handleEnterUp, "keyup");
        this.mousetrap.bind("space", handleSpaceDown, "keydown");
        this.mousetrap.bind("space", handleSpaceUp, "keyup");
        this.mousetrap.bind("escape", handleEscapeDown, "keydown");
        this.mousetrap.bind("escape", handleEscapeUp, "keyup");
        this.addEventListener("keydown", this._handleKeyEvent);
        this.addEventListener("keyup", this._handleKeyEvent);
        this.addEventListener("keypress", this._handleKeyEvent);
    }
    dispatchEventOnActiveElement(event_name, detail) {
        let elt = document.activeElement;
        while (elt.shadowRoot && elt.shadowRoot.activeElement) {
            elt = elt.shadowRoot.activeElement;
        }
        elt.dispatchEvent(new CustomEvent(event_name, {
            bubbles: true,
            composed: true,
            detail: detail
        }));
    }
    getEventHandler(name) {
        let pressed = false;
        return [
            function handleKeydown(e) {
                if (pressed) {
                    return;
                }
                pressed = true;
                let elt = document.activeElement;
                if (!elt) {
                    return;
                }
                this.dispatchEventOnActiveElement(name, { "originalEvent": e });
            }.bind(this),
            function handleKeyup(e) {
                pressed = false;
            }.bind(this)
        ];
    }
    _handleKeyEvent(e) {
        if (this._stopKeypressCallback(e, e.target || e.srcElement)) {
            return;
        }
        let info = this.getKeyEventInfo(e);
        if (info === undefined) {
            return;
        }
        let [keyType, detail] = info;
        detail.originalEvent = e;
        this.dispatchEventOnActiveElement(`${e.type}-${keyType}`, detail);
    }
    getKeyEventInfo(e) {
        if (e.code.startsWith("Arrow")) {
            let direction = e.code.slice("Arrow".length).toLowerCase();
            let dx = { "up": 0, "down": 0, "left": -1, "right": 1 }[direction];
            let dy = { "up": 1, "down": -1, "left": 0, "right": 0 }[direction];
            return ["arrow", { direction: [dx, dy] }];
        }
        if (e.code.startsWith("Digit")) {
            return ["digit", { digit: parseInt(e.key) }];
        }
        if (["+", "-"].includes(e.key)) {
            let dir = { "+": 1, "-": -1 }[e.key];
            return ["pm", { direction: dir }];
        }
        if (["w", "a", "s", "d"].includes(e.key)) {
            let dx = { "w": 0, "s": 0, "a": -1, "d": 1 }[e.key];
            let dy = { "w": 1, "s": -1, "a": 0, "d": 0 }[e.key];
            return ["wasd", { direction: [dx, dy] }];
        }
    }
}
customElements.define('sseq-ui', UIElement);
