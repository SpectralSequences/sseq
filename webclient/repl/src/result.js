import {LitElement, html, css} from 'lit-element';

class ResultElement extends LitElement {
    static get styles() {
        return css`
            pre {
                color : white;
                font-family: monospace;
                font-size: 14pt;
                padding-bottom : 8px;  
                margin : 0;          
            }
        `
    }

    static get properties() {
        return { 
            value : { type: String }
        };
    }

    render(){
        return html `
            <pre>${this.value}</pre>
        `
    }
}
if(!customElements.get("repl-result")){
    customElements.define('repl-result', ResultElement);
}