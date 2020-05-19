import {LitElement, html, css} from 'lit-element';


export class ButtonElement extends LitElement {
    static get styles() {
        return css `
        :host {
            font-variant: all-small-caps;
            font: 400 14pt Arial;
            text-align : center;
            box-sizing: border-box;
            padding: 1px 6px;
            outline: none;
            color : rgba(var(--button-text-color), var(--button-text-opacity));
            background : var(--button-background-color);
            user-select: none;
            display: flex;
            justify-content: center;
            align-items: center;
        }

        :host(:not([disabled])) {
            cursor: pointer;
        }

        :host([disabled]) {
            opacity : 0.5;
            cursor: default;
        }

        
        :host(:not([disabled]):hover) {
            box-shadow: inset 0px 0px 5px #CCC;
        }

        :host(:not([disabled]):active) {
            box-shadow: 0px 0px 8px #CCC;
            background-color: rgb(224, 224, 224, 0.5);
        }       
        `;
    }

    render(){
        return html`
        <!-- <button class="btn"> -->
            <slot></slot>
        <!-- </button> -->
        `;
    }
}

customElements.define('sseq-button', ButtonElement);
