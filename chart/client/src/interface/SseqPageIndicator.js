import {LitElement, html} from 'lit-element';

export class SseqPageIndicator extends LitElement {
    static get properties() {
        return { 
            page_value : { type: String }
        };
    }

    constructor(){
        super(); 
        this.page_value = "";
    }

    firstUpdated(changedProperties) {
        this.page_value = this.parentElement.page;
        this.parentElement.addEventListener("page-change", (e) => {
            this.page_value = e.detail[0];
        });
    }

    render() {
        // if(this.parentElement){
        //     this.page_value = this.parentElement.page;
        // }
        return html`<p>${this.parentElement.getPageDescriptor(this.page_value)}</p>`;
    }
}
customElements.define('sseq-page-indicator', SseqPageIndicator);
