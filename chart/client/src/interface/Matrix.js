import {LitElement, html, css} from 'lit-element';
// import {unsafeHTML} from 'lit-html/directives/unsafe-html.js';
// import { KatexExprElement } from "../KatexExprElement.js";


export class MatrixElement extends LitElement {
    static get properties() {
        return { 
            value : { type: Array },
            selectedRows : { type : Array }
        };
    }

    static get styles() {
        return css`
            .matrixbrak {
                outline: none;
            }

            .matrix {
                border-spacing: 0px;
            }

            .row {
                margin-left : -var(--label-width);
            }

            td.lbrak {
                transform: translateX(calc(var(--label-width, 0) + 2.8ex));
                width: 0.8ex;
                font-size: 50%;
                border-top: solid 0.25ex currentColor;
                border-bottom: solid 0.25ex currentColor;
                border-left: solid 0.5ex currentColor;
                border-right: none;
            }

            td.rbrak {
                transform: translateX(-2.8ex);
                width: 0.8ex;
                font-size: 50%;
                border-top: solid 0.25ex currentColor;
                border-bottom: solid 0.25ex currentColor;
                border-right: solid 0.5ex currentColor;
                border-left: none;
            }

            table.matrixbrak td {
                line-height: 1.5;
            }

            td {
                text-align: center;
                line-height: 1.2rem;
                padding : 2px;
                user-select: none;
            }

            td .padding {
                /* background : gray; */
                width: 0.5rem;
                height: 1.2rem;
                /* We want this to be above lbrak and rbrak so we use z-index
                   z-index only applies to positioned elements, so set position to relative. */
                position : relative; 
                z-index : 100;
            }

            td .entry {
                /* background : blue; */
                width: 0.7rem;
            }

            td .label-entry {
                display : flex;
                flex-direction : row-reverse;
                padding-right : 10pt;
            }

            input {
                width : 1.5rem;
                height: 1.5rem;
                padding: 0;
                text-align: center;
                margin: 0;
            }

            input[type="number"]::-webkit-outer-spin-button, input[type="number"]::-webkit-inner-spin-button {
                -webkit-appearance: none;
                margin: 0;
            }
            
            input[type="number"] {
                -moz-appearance: textfield;
            }

            .row:active {
                box-shadow: 0px 2px 2px -1px var(--row-active);
                background-color : var(--row-active) !important;
                color : rgba(var(--text-color), 1);
            }

            .row:hover {
                background-color : var(--row-hover) !important;
                color : rgba(var(--text-color), 1);
            }

            .row:hover:active {
                background-color : var(--row-active) !important;
            }            
            
            .row[selected] {
                background-color : var(--row-selected);
                color : rgba(var(--text-color), 1);
            }

            .row:active[selected] {
                box-shadow: 0px 2px 2px -1px var(--row-active-selected);
                background-color : var(--row-active-selected) !important;
            }

            .row:hover[selected] {
                background-color : var(--row-hover-selected) !important;
            }
            
            
            /*
            .row:active td.padding,  .row:active td.entry {
                box-shadow: 0px 2px 2px -1px var(--row-active);
                background-color : var(--row-active) !important;
            }

            .row:hover td.padding,  .row:hover td.entry {
                background-color : var(--row-hover) !important;
            }
            
            .row[selected] td.padding,  .row[selected] td.entry {
                background-color : var(--row-selected);
            }

            .row:active[selected] td.padding,  .row:active[selected] td.entry {
                box-shadow: 0px 2px 2px -1px var(--row-active-selected);
                background-color : var(--row-active-selected) !important;
            }

            .row:hover[selected]  td.padding,  .row:hover[selected]  td.entry {
                background-color : var(--row-hover-selected) !important;
            }  */
        `
    }

    constructor(){
        super();
        this._handleMouseEvent = this._handleMouseEvent.bind(this);
        this.value = [];
        this.labels = undefined;
        this.lastTargets = [];
        this.selectedRows = [];
    }

    render(){
        this.max_value = this.getAttribute("max-value") || 1;
        let rows = this.value.length;
        let columns = rows > 0 ? this.value[0].length : 0;
        let selectedEntries = new Array(rows).fill(0).map(() => new Array(columns).fill(false));
        for(let i of this.selectedRows){
            for(let j = 0; j < columns; j++){
                selectedEntries[i][j] = true;
            }
        }
        document.createElement("table");

        return html`
<table class="matrixbrak" tabindex="0"
        @click=${this._handleMouseEvent} @dblclick=${this._handleMouseEvent} @contextmenu=${this._handleMouseEvent} 
        @mousedown=${this._handleMouseEvent} @mouseup=${this._handleMouseEvent}
        @mouseover=${this._handleMouseEvent} @mouseout=${this._handleMouseEvent}
        @mouseenter=${this._handleMouseEvent} @mouseleave=${this._handleMouseEvent}
        @mousemove=${this._handleMouseEvent}
>
<tbody>
    <tr>
        <!-- ${  
            this.labels ? html`
                <td><table class="labels"><tbody>
                    ${this.labels.map((label, ridx) => html`
                        <tr class="label-row" pos="${ridx}"><td class="label-entry" pos="${ridx}"><span style="flex-grow:1;"></span><katex-expr>${label}</katex-expr></td></tr>
                    `)}
                </tbody></table></td>
            ` : ""
        } -->
        <td class="lbrak">&nbsp;</td>
        <td> <table class="matrix"><tbody>
            ${
                this.value.map((r, ridx) => html`
                    <tr class="row" pos="${ridx}" ?selected=${this.selectedRows.includes(ridx)}>
                    ${  this.labels ?
                        html`<td class="label-entry" pos="${ridx}"><katex-expr>${this.labels[ridx]}</katex-expr></td>`
                    :""}
                        <td class="padding" ?selected=${this.selectedRows.includes(ridx)}></td>
                        ${r.map((e, colidx) => 
                            html`<td class="entry" pos="${ridx}-${colidx}" ?selected=${selectedEntries[ridx][colidx]}>${this.wrapEntry(e)}</td>`
                        )}
                        <td class="padding" ?selected=${this.selectedRows.includes(ridx)}></td>
                    </tr>
                `)
            }
        </tbody></table></td>
        <td class="rbrak">&nbsp;</td>
    </tr>
</tbody></table>
        `
    }

    updated(changedProperties) {
        let label_entry = this.shadowRoot.querySelector(".label-entry");
        if(label_entry){
            if(!this.resizeObserver) {
                this.resizeObserver = new ResizeObserver(entries => {
                    // resizeObserver.disconect();
                    this.style.setProperty("--label-width", `${label_entry.offsetWidth}px`);
                });
            }
            this.resizeObserver.observe(label_entry);        
        }
    }

    getEntry(e){
        let path = e.composedPath();
        // if(e.type === "mouseover"){
        //     this.lastTarget = target;
        // }
        if(path[1].className !== "row"){
            return;
        }
        let row_idx = Number.parseInt(path[1].getAttribute("pos"));
        let result = {};
        result.originalEvent = e;
        result.row_idx = row_idx;
        result.row = this.shadowRoot.querySelectorAll(".row")[row_idx];
        if(path[0].className === "entry") {
            let col_idx = Number.parseInt(path[0].getAttribute("pos").split("-")[1]);
            result.col_idx = col_idx;
            result.entry = result.row.querySelectorAll(".entry")[col_idx];
        }
        return result;
    }

    _handleMouseEvent(e){
        let detail = this.getEntry(e);
        if(!detail){
            return;
        }
        this.dispatchEvent(new CustomEvent(`matrix-${e.type}`, { 
            detail: detail,
            bubbles: true, 
            composed: true 
        }));
    }
    
    _inputkeydown(e){
        let path = e.composedPath();
        if(["Backspace", "Delete"].includes(e.code)){
            e.preventDefault();    
        }
        if(!e.code.startsWith("Arrow")){
            return;
        }
        e.preventDefault();
        let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
        let direction = e.code.slice("Arrow".length).toLowerCase();
        let dr = {"up" : -1, "down" : 1, "left" : 0, "right" : 0}[direction];
        let dc = {"up" : 0, "down" : 0, "left" : -1, "right" : 1}[direction];
        let targetRow = row + dr;
        let targetCol = col + dc;
        let target = this.shadowRoot.querySelector(`[pos="${targetRow}-${targetCol}"] input`);
        if(target){
            target.focus();
            target.select();
        }
    }

    _inputkeypress(e){
        e.preventDefault();
        let path = e.composedPath();
        let value = parseInt(e.key);
        if(isNaN(value)){
            return;
        }
        let digit = parseInt(e.key);
        if(digit <= this.max_value){
            path[0].value = digit;
            path[0].select();
            let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
            this.value[row][col] = digit;
        }
    }

    _inputclick(e){
        let path = e.composedPath();
        path[0].select();
    }

    _handleDigit(row, col, e) {

    }

    wrapEntry(e){
        switch(this.getAttribute("type")){
            case "input":
                return html`<input type="number" value="${e}" @click="${this._inputclick}" @keydown="${this._inputkeydown}" @keypress="${this._inputkeypress}"></input>`;
            case "display":
                return e;
            default:
                throw Error("Invalid value for type");
        }
    }
}
customElements.define('sseq-matrix', MatrixElement);