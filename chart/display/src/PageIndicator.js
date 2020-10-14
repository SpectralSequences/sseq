import {LitElement, html} from 'lit-element';
import { INFINITY } from "./chart/infinity";

export class PageIndicatorElement extends LitElement {
    constructor(){
        super(); 
        this.pageRange = undefined;
    }

    firstUpdated(changedProperties) {
        let elt = this.closest("sseq-display");
        if(elt === undefined){
            throw Error("sseq-class-highlighter must be a descendant of sseq-display.");
        }
        this.disp = elt;
        this.pageRange = this.disp.page;
        this.disp.addEventListener("page-change", (e) => {
            this.pageRange = e.detail.pageRange;
            this.requestUpdate();
        });
    }

    getPageDescriptor(pageRange) {
        if (!this.pageRange) {
            return;  
        }

        if (pageRange[0] === INFINITY) {
            return "Page ∞";
        }

        if(pageRange[1] === INFINITY){
            return `Page ${pageRange[0]} with all differentials`;
        }

        if(pageRange[1] < pageRange[0]){
            return `Page ${pageRange[0]} with no differentials`;
        }

        if(pageRange[0] === pageRange[1]){
            return `Page ${pageRange[0]}`;
        }

        return `Page ${pageRange[0]} with differentials of length ${pageRange[0]} – ${pageRange[1]}`;
    }

    render() {
        return html`<p>${this.getPageDescriptor(this.pageRange)}</p>`;
    }
}
customElements.define('sseq-page-indicator', PageIndicatorElement);
