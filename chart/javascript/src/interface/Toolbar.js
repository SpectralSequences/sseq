import {LitElement, html, css} from 'lit-element';
import { styleMap } from 'lit-html/directives/style-map';


import { sleep, promiseFromDomEvent } from "./utils.js";

export class ToolbarElement extends LitElement {
    
}
customElements.define('sseq-toolbar', ToolbarElement);