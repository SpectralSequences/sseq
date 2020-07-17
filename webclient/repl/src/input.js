import {LitElement, html, css} from 'lit-element';


function stringRows(s, cols){
    let numRows = 0;
    let lastIdx = 0;
    for(let i = 0; i < s.length; i++){
        if(s.charAt(i) === "\n"){
            numRows += Math.max(1, Math.ceil((i - lastIdx)/cols));
            lastIdx = i;
        }
    }
    numRows += Math.max(1, Math.ceil((s.length - lastIdx)/cols));
    return numRows;
}
window.stringRows = stringRows;

function getStringXYCoords(str, idx, cols) {
    let y = 0;
    let lineStartIdx = 0;
    for(let i=0; i < idx; i++){
        if(str.charAt(i) === "\n"){
            lineStartIdx = i + 1;
            y++;
        }
        if(i - lineStartIdx === cols){
            lineStartIdx = i;
            y++;
        }
    }
    return [idx - lineStartIdx, y];
}

const State = Object.freeze({
    "active" : "active",
    "evaluating" : "evaluating",
    "cancelled" : "cancelled",
    "completed" : "completed",
    "error" : "error"
});

class InputElement extends LitElement {
    static get styles() {
        return css`
            :host, .textarea {
                color : white;
                font-family: monospace;
                font-size: 16pt;                
            }

            .flex-row {
                display : flex;
                flex-direction : row;
            }

            .margin {
                display : inline-block;
                margin-right : 1ch;
                user-select : none;
            }
            
            .textarea-wrap {
                position: relative;
                height : auto;
            }

            .textarea {
                resize: none;
                border: none;
                padding : 0px;
                background-color: black;
                outline : none !important;
                caret-color: black; /* disable default caret */
            }

        `;
    }

    get active() {
        return this.getAttribute('state') === State.active;
    }
  
    set state(v) {
        this.setAttribute('state', v);
    }

    get value(){
        return this.textarea.value;
    }

    set value(v){
        this.textarea.value = v;
        this.updateRows();
    }

    get hasHighlight(){
        return this.textarea.selectionStart < this.textarea.selectionEnd;
    }

    get atStart(){
        return this.selectionStart === 0 && !this.hasHighlight;
    }

    get atEnd(){
        return this.selectionStart === this.value.length && !this.hasHighlight;
    }

    setCursorPosition(pos){
        if(pos < 0){
            pos = this.value.length + pos + 1;
        }
        this.textarea.setSelectionRange(pos, pos);
    }

    constructor(){
        super();
        this.cols = 80;
    }

    render(){
        return html `
            <div class="flex-row">
                <div class="margin"> >>> </div>
                <div class="textarea-wrap">
                    <repl-caret class="caret"></repl-caret>
                    <textarea class="textarea" style="width:${this.cols}ch;" ?contenteditable =${this.active} spellcheck="false"></textarea>
                </div>
            </div>
            
        `;
    }

    firstUpdated(){
        this.textarea = this.shadowRoot.querySelector(".textarea");
        this.caret = this.shadowRoot.querySelector(".caret");
        
        this.textarea.addEventListener("input", this._oninput.bind(this));
        this.textarea.addEventListener("keypress", this._onkeypress.bind(this));
        // this.addEventListener("focusout", this._onblur.bind(this));
        this.addEventListener("focus", this._onfocus.bind(this));
        document.addEventListener("selectionchange", this.updateCaret.bind(this));
        // document.addEventListener("input", this.updateCaret.bind(this));

        this.textarea.setAttribute("rows", 1);
        this.caret.setLineHeight(getComputedStyle(this.textarea).height);
        this.updateRows();
    }

    _oninput(evt){
        this.updateRows();
        this.updateCaret();
    }

    async _onkeypress(evt) {
        if(evt.code === "Enter" && evt.ctrlKey || evt.code === "NumpadEnter"){
            this.state = State.evaluating;
            this.requestUpdate();
            this.closest("repl-terminal").execute(this, this.value);
            evt.preventDefault();
        }
    }

    _onfocus(){
        
    }


    focus(){
        if(this.savedSelection){
            // if(this.savedSelection[0] < this.savedSelection[1]){

            // }
            this.textarea.setSelectionRange(this.selectionStart, this.selectionEnd);
        }
        this.textarea.focus();
    }

    updateCaret(){
        if(document.activeElement === this){
            this.selectionStart = this.textarea.selectionStart;
            this.selectionEnd = this.textarea.selectionEnd;
            this.caret.setPosition(...getStringXYCoords(this.value, this.selectionEnd, this.cols));
            this.caret.selectionHighlight = this.selectionStart < this.selectionEnd;
        }
    }

    updateRows(){
        let innerText = this.value;
        // if(innerText.endsWith("\n\n")){
        //     innerText = innerText.slice(0,-1);
        // }
        let numRows = stringRows(innerText, this.cols);
        this.shadowRoot.querySelector(".textarea").setAttribute("rows", numRows);
        this.shadowRoot.querySelector(".margin").innerText = `>>>${"\n...".repeat(numRows - 1)}`;
    }

    // evaluationStarted(){
    //     this.caret.hide();
    //     this.state = "evaluating";
    // }
    
    // evaluationCompleted(){
    //     this.state = "completed";
    //     this.textarea.setAttribute("readonly", "");
    // }

    // evaluationError(){
    //     this.state = "error";
    //     this.textarea.setAttribute("readonly", "");
    // }

    // test(){
    //     this.editor.changeViewZones(acc => acc.)
    // }
}

customElements.define('repl-input', InputElement);