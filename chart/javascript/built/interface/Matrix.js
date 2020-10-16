var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { LitElement, html, css } from 'lit-element';
import { ifDefined } from 'lit-html/directives/if-defined';
// import {unsafeHTML} from 'lit-html/directives/unsafe-html.js';
// import { KatexExprElement } from "../KatexExprElement.js";
import { promiseFromDomEvent } from "./utils.js";
export class MatrixElement extends LitElement {
    static get properties() {
        return {
            value: { type: Array },
            selectedRows: { type: Array },
            enabledRows: { type: Array }
        };
    }
    static get styles() {
        return css `
            :host {
                color : rgba(var(--text-color), var(--text-opacity));
            }

            ::selection {
                background : rgba(var(--selection-background-color), 1);
            }

            /*tr {
                border-top-color: rgba(0,0,0,0);
                border-top-style: double;
                border-top-width: var(--focus-outline-thickness);
            }*/
    
            /*tr:last-child {
                border-bottom-color: rgba(0,0,0,0);
                border-bottom-style: double;
                border-bottom-width: var(--focus-outline-thickness);
            }*/
    
            :focus {
                /*border-color: var(--focus-outline-color);
                border-style: double;
                border-width: 2px;*/
                --test : var(--focus-outline-color);
                border: var(--focus-outline-thickness) solid purple;
                outline : var(--focus-outline-color) solid var(--focus-outline-thickness);
            }

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
                background-color: rgba(var(--input-background-color), 1);
                color : rgba(var(--input-text-color), 1);                
            }

            input[type="number"]::-webkit-outer-spin-button, input[type="number"]::-webkit-inner-spin-button {
                -webkit-appearance: none;
                margin: 0;
            }
            
            input[type="number"] {
                -moz-appearance: textfield;
            }

            :host([type="select-row"]) .row:active, :host([type="select-row"]) .row.active {
                box-shadow: 0px 2px 2px -1px var(--row-active);
                background-color : var(--row-active) !important;
                color : rgba(var(--text-color), 1);
            }

            :host([type="select-row"]) .row[disabled] {
                --text-opacity : var(--disabled-text-opacity);
                color : rgba(var(--text-color), var(--text-opacity));
                pointer-events : none;
            }

            :host([type="select-row"]) .row:hover:not([disabled]) {
                background-color : var(--row-hover) !important;
                color : rgba(var(--text-color), 1);
            }

            :host([type="select-row"]) .row:hover:active, :host([type="select-row"]) .row:hover.active {
                background-color : var(--row-active) !important;
            }            
            
            :host([type="select-row"]) .row[selected] {
                background-color : var(--row-selected);
                color : rgba(var(--text-color), 1);
            }

            :host([type="select-row"]) .row:hover:active[selected], :host([type="select-row"]) .row.active[selected] {
                box-shadow: 0px 2px 2px -1px var(--row-active-selected);
                background-color : var(--row-active-selected) !important;
            }

            :host([type="select-row"]) .row:hover[selected] { 
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
        `;
    }
    constructor() {
        super();
        this._handleMouseEvent = this._handleMouseEvent.bind(this);
        this.value = [];
        this.lastTargets = [];
        this.selectedRows = [];
    }
    firstUpdated(changedProperties) {
        this.addEventListener("matrix-click", (e) => {
            this.toggleRowSelect(e.detail.row_idx);
        });
        this.addEventListener("interact-toggle", (e) => {
            e.stopPropagation();
            this.toggle(e);
        });
    }
    get type() {
        return this.getAttribute("type");
    }
    get max_value() {
        return this.getAttribute("max-value") || 1;
    }
    get labels() {
        return this._labels;
    }
    set labels(v) {
        if (!this._labels) {
            this._labels = v;
            this.requestUpdate();
            return;
        }
        this._labels = v;
        let label_entries = this.shadowRoot.querySelectorAll(".label-entry katex-expr");
        v.forEach((v, idx) => { label_entries[idx].innerText = v; });
    }
    toggle(e) {
        return __awaiter(this, void 0, void 0, function* () {
            let elt = this.shadowRoot.activeElement;
            if (!elt) {
                return;
            }
            if (e.constructor === CustomEvent) {
                e = e.detail.originalEvent;
            }
            if (e.constructor === KeyboardEvent) {
                elt.classList.add("active");
                yield promiseFromDomEvent(window, "keyup", (keyupEvent) => keyupEvent.key === e.key);
                elt.classList.remove("active");
                this.toggleRowSelect(parseInt(elt.getAttribute("pos")));
            }
        });
    }
    toggleRowSelect(row) {
        if (this.type !== "select-row") {
            return;
        }
        if (this.enabledRows) {
            if (!this.enabledRows[row]) {
                return;
            }
        }
        if (this.selectedRows.includes(row)) {
            this.selectedRows = [];
        }
        else {
            this.selectedRows = [row];
        }
        this.dispatchEvent(new CustomEvent("matrix-select", { "detail": this.selectedRows }));
        this.requestUpdate();
    }
    render() {
        let rows = this.value.length;
        let columns = rows > 0 ? this.value[0].length : 0;
        let selectedEntries = new Array(rows).fill(0).map(() => new Array(columns).fill(false));
        for (let i of this.selectedRows) {
            for (let j = 0; j < columns; j++) {
                selectedEntries[i][j] = true;
            }
        }
        let enabledRows = this.enabledRows || Array(rows).fill(true);
        let rowTabIndices = enabledRows.map(e => (this.type === "select-row") && e ? 0 : undefined);
        return html `
<table class="matrixbrak" 
        @click=${this._handleMouseEvent} @dblclick=${this._handleMouseEvent} @contextmenu=${this._handleMouseEvent} 
        @mousedown=${this._handleMouseEvent} @mouseup=${this._handleMouseEvent}
        @mouseover=${this._handleMouseEvent} @mouseout=${this._handleMouseEvent}
        @mouseenter=${this._handleMouseEvent} @mouseleave=${this._handleMouseEvent}
        @mousemove=${this._handleMouseEvent}
>
<tbody>
    <tr>
        <!-- ${this.labels ? html `
                <td><table class="labels"><tbody>
                    ${this.labels.map((label, ridx) => html `
                        <tr class="label-row" pos="${ridx}"><td class="label-entry" pos="${ridx}"><span style="flex-grow:1;"></span><katex-expr>${label}</katex-expr></td></tr>
                    `)}
                </tbody></table></td>
            ` : ""} -->
        <td class="lbrak">&nbsp;</td>
        <td> <table class="matrix"><tbody>
            ${this.value.map((r, ridx) => html `
                    <tr class="row" pos="${ridx}" 
                        tabindex="${ifDefined(rowTabIndices[ridx])}" 
                        ?selected=${this.selectedRows.includes(ridx)} 
                        ?focus=${enabledRows[ridx]} 
                        ?disabled=${!enabledRows[ridx]} 
                    >
                    ${this.labels ?
            html `<td class="label-entry" pos="${ridx}"><div><katex-expr>${this.labels[ridx]}</katex-expr></div></td>`
            : ""}
                        <td class="padding" ?selected=${this.selectedRows.includes(ridx)}></td>
                        ${r.map((e, colidx) => html `<td class="entry" pos="${ridx}-${colidx}" ?selected=${selectedEntries[ridx][colidx]}>${this.wrapEntry(e)}</td>`)}
                        <td class="padding" ?selected=${this.selectedRows.includes(ridx)}></td>
                    </tr>
                `)}
        </tbody></table></td>
        <td class="rbrak">&nbsp;</td>
    </tr>
</tbody></table>
        `;
    }
    updated(changedProperties) {
        // let row_select = this.type === "select-row";
        // if(row_select){
        //     for(let row of this.shadowRoot.querySelectorAll(".row")){
        //         row.tabIndex = 0;
        //     }
        // }
        let label_entry = this.shadowRoot.querySelector(".label-entry");
        if (label_entry) {
            if (!this.resizeObserver) {
                this.resizeObserver = new ResizeObserver(entries => {
                    this.style.setProperty("--label-width", `${label_entry.offsetWidth}px`);
                });
            }
            this.resizeObserver.observe(label_entry);
        }
    }
    getEntry(e) {
        let path = e.composedPath();
        // if(e.type === "mouseover"){
        //     this.lastTarget = target;
        // }
        if (path[1].className !== "row") {
            return;
        }
        let row_idx = Number.parseInt(path[1].getAttribute("pos"));
        let result = {};
        result.originalEvent = e;
        result.row_idx = row_idx;
        result.row = this.shadowRoot.querySelectorAll(".row")[row_idx];
        if (path[0].className === "entry") {
            let col_idx = Number.parseInt(path[0].getAttribute("pos").split("-")[1]);
            result.col_idx = col_idx;
            result.entry = result.row.querySelectorAll(".entry")[col_idx];
        }
        return result;
    }
    _handleMouseEvent(e) {
        let detail = this.getEntry(e);
        if (!detail) {
            return;
        }
        this.dispatchEvent(new CustomEvent(`matrix-${e.type}`, {
            detail: detail,
            bubbles: true,
            composed: true
        }));
    }
    getEntryInput(row, col) {
        return this.shadowRoot.querySelector(`[pos='${row}-${col}'] input`);
    }
    updateEntryInput(row, col) {
        this.getEntryInput(row, col).value = this.value[row][col];
    }
    focusEntryInput(row, col) {
        let target = this.getEntryInput(row, col);
        if (target) {
            target.focus();
        }
    }
    _inputKeydown(e) {
        let path = e.composedPath();
        if (["Backspace", "Delete"].includes(e.code)) {
            e.preventDefault();
        }
        if (!e.code.startsWith("Arrow")) {
            return;
        }
        e.preventDefault();
        let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
        let direction = e.code.slice("Arrow".length).toLowerCase();
        let dr = { "up": -1, "down": 1, "left": 0, "right": 0 }[direction];
        let dc = { "up": 0, "down": 0, "left": -1, "right": 1 }[direction];
        let targetRow = row + dr;
        let targetCol = col + dc;
        if (e.ctrlKey) {
            if (dr === 0 || targetRow < 0 || targetRow >= this.value.length) {
                return;
            }
            let curRow = this.value[row];
            let swapRow = this.value[targetRow];
            this.value[row] = swapRow;
            this.value[targetRow] = curRow;
            for (let i = 0; i < curRow.length; i++) {
                this.updateEntryInput(row, i);
                this.updateEntryInput(targetRow, i);
            }
            this.dispatchEvent(new CustomEvent("change"));
        }
        this.focusEntryInput(targetRow, targetCol);
    }
    _inputKeypress(e) {
        e.preventDefault();
        let path = e.composedPath();
        let value = parseInt(e.key);
        if (isNaN(value)) {
            return;
        }
        let digit = parseInt(e.key);
        if (digit <= this.max_value) {
            path[0].value = digit;
            path[0].select();
            let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
            this.value[row][col] = digit;
            this.dispatchEvent(new CustomEvent("change", { detail: { row: row, col: col } }));
        }
    }
    _inputCopy(e) {
        e.preventDefault();
        let path = e.composedPath();
        let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
        (e.clipboardData || window.clipboardData).setData('text', JSON.stringify(this.value[row]));
    }
    _inputPaste(e) {
        e.preventDefault();
        let path = e.composedPath();
        let [row, col] = path[1].getAttribute("pos").split("-").map((x) => parseInt(x));
        let pastedText = (e.clipboardData || window.clipboardData).getData('text');
        let filterRegex = new RegExp(`^\[?[, 0-${this.max_value}]*\]?$`);
        if (!filterRegex.test(pastedText)) {
            return;
        }
        let values = pastedText.replace(/[, \[\]]/g, "").split("").map((v) => parseInt(v));
        if (values.length === this.value[row].length) {
            for (let i = 0; i < values.length; i++) {
                this.value[row][i] = values[i];
                this.updateEntryInput(row, i);
            }
            this.dispatchEvent(new CustomEvent("change", { detail: { row: row } }));
        }
        else if (values.length + col <= this.value[row].length) {
            for (let i = 0; i < values.length; i++) {
                this.value[row][col + i] = values[i];
                this.updateEntryInput(row, col + i);
            }
            this.dispatchEvent(new CustomEvent("change", { detail: { row: row } }));
        }
    }
    _inputFocus(e) {
        let path = e.composedPath();
        path[0].select();
    }
    wrapEntry(e) {
        switch (this.type) {
            case "input":
                return html `
                    <input type="number" tabindex="0" focus value="${e}"  
                           @keydown="${this._inputKeydown}"
                           @keypress="${this._inputKeypress}"
                           @focus="${(e) => this._inputFocus(e)}"
                           @copy="${(e) => this._inputCopy(e)}"
                           @paste="${(e) => this._inputPaste(e)}"
                    >
                    `;
            case "display":
                return e;
            case "select-row":
                return e;
            case null:
                return e; // throw Error(`Matrix is missing type.`); //?? 
            default:
                throw Error(`Invalid value "${this.type}" for type`);
        }
    }
}
customElements.define('sseq-matrix', MatrixElement);
