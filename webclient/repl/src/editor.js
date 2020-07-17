import { updatePythonLanguageDefinition } from "./pythonTextCompletion";
import { monaco } from "./monaco";
import { sleep } from './utils';
import { PythonExecutor } from "./python_executor";
import { History } from "./history";

// function sleep(ms) {
//     return new Promise(resolve => setTimeout(resolve, ms));
// }

updatePythonLanguageDefinition(monaco);
function isSelectionNonempty(sel){
	return sel.startLineNumber < sel.endLineNumber || sel.startColumn < sel.endColumn;
}

class TerminalElement extends HTMLElement {
	static get defaultEditorOptions(){
		return {
			value: "blah\nblah\nblah",
			language: "python",
			lineNumbers : (n) => n == 1 ? ">>>" : "...",
			folding : false,
			theme : "vs-dark",
			roundedSelection : false,
			fontSize : 20,
			minimap: {
				enabled: false
			},
			wordWrap: 'wordWrapColumn',
			wordWrapColumn: 80,
			overviewRulerLanes : 0,
			scrollbar : {
				horizontal : "hidden",
				vertical : "hidden"
			},
			scrollBeyondLastLine : false,
			contextmenu : false
		};
    }
    
    static get styles() {
        return css`
            #root {
                height : 100%;
                width : 100%;
            }
        `
	}

	get position(){
		return this.editor.getPosition();
	}

	set position(v){
		this.editor.setPosition(v);
	}

	get offset(){
		return this.editor.getModel().getOffsetAt(this.position);
	}

	set offset(v){
		this.position = this.editor.getModel().getPositionAt(v);
	}

	get endPosition(){
		return this.editor.getModule().getPositionAt(this.value.length);
	}


	get value(){
		return this.editor.getValue();
	}

	set value(v){
		this.editor.setValue(v);
	}

	get readOnlyLines(){
		return this._readOnlyLines;
	}

	set readOnlyLines(v){
		this._readOnlyLines = v;
		this._readOnlyOffset = this.editor.getModel().getOffsetAt(
			new monaco.Position(this._readOnlyLines + 1, 1)
		);
	}
	
	get readOnlyOffset(){
		return this._readOnlyOffset;
	}

	
	get atStart(){
		return this.offset === 0;
	}

	get atEnd(){
		return this.offset === this.value.length;
	}

	constructor(options){
		super();
		this._focused = false;
		this._visible = true;
		this.editorOptions = Object.assign(EditorElement.defaultEditorOptions, options);
		this._readOnlyLines = 0;
		this._readOnlyOffset = 0;
	}

	connectedCallback(){
        let styles = document.createElement("style");
        styles.innerText = EditorElement.styles;
        this.appendChild(styles);
        let div = document.createElement("div");
        div.id = "root";
        this.appendChild(div);
		this.editor = monaco.editor.create(
            this.querySelector("div"),
            this.editorOptions
        );
		this.editor.updateOptions({
			"autoIndent": true
		});
		this.querySelector(".decorationsOverviewRuler").remove();
		this.focusElement = this.querySelector("textarea");
        
        // Ensure that selection color behaves according to our definition of focus
		this.focusMutationObserver = new MutationObserver(async (mutationsList, observer) => {
			// await sleep(10);//
			this.updateViewFocus();
		});
        this.focusMutationObserver.observe(this.querySelector(".view-overlays"), { attributes: true });
        // Ensure that cursor behaves according to our definition of focus
		this.setupCursor();
		this.editor.onKeyDown(this._onkey.bind(this));
		this.editor.onKeyUp(this._onkey.bind(this));
		this.editor.onDidChangeCursorSelection((e) => {
			console.log(e);
			if(isSelectionNonempty(e.selection)){
				return;
			}
			if(this.offset < this.readOnlyOffset){
				this.editor.setSelections(e.oldSelections);
			}
		});
	}
	
	async nextHistory(n = 1){
        await this.stepHistory(n);
    }
    
    async previousHistory(n = 1){
        await this.stepHistory(-n);
    }

    async stepHistory(didx) {
        this.historyIdx = Math.min(Math.max(this.historyIdx + didx, 0), this.history.length);
        this.input.value = await this.history[this.historyIdx] || "";
        await sleep(0);
        this.input.offset = this.input.value.length; // Set cursor at end
	}
	
	_onkey(e){
		this.enforceReadOnlyRegion(e);
		if(
			e.browserEvent.key.startsWith("Arrow") 
			|| [
				"Backspace", "Tab", "Delete", 
				"Home", "End", "PageUp", "PageDown",
			].includes(e.browserEvent.key)
		){
			this.dispatchEvent(new KeyboardEvent(e.browserEvent.type, e.browserEvent));
		}
		if(e.browserEvent.key === "Enter") {
			if(this.parentElement.shouldEnterSubmit(e.browserEvent)){
				e.preventDefault();
			}
		}
	}

    // _onkey(event){
    //     if(this.shouldStepHistory(event)){
    //         this.justSteppedHistory = true;
    //         let dir = {"Up" : -1, "Down" : 1}[event.key.slice("Arrow".length)];
    //         this.stepHistory(dir);
    //         sleep(0).then(() => this.input.focus());
    //         return
    //     }
    //     this.justSteppedHistory = false;
    //     if(event.key === "Enter") {
	// 		if(this.shouldEnterSubmit(event)){
    //             this.submit();
    //             return;
	// 		}
	// 	}
    // }

	/** 
	*   e.preventDefault() doesn't work on certain keys, including Backspace, Tab, and Delete.
	* 	As a recourse to prevent these rogue key events, we move the focus out of the input area 
	*  	for just a moment and then move it back.
	*/
	preventKeyEvent(){
		document.querySelector("repl-terminal").focus();
		sleep(0).then(() => this.focus());
	}
	
	enforceReadOnlyRegion(e){
		if(e.browserEvent.type !== "keydown"){
			return;
		}
		if(!/^[ -~]$/.test(e.browserEvent.key) && !["Backspace", "Tab", "Delete", "Enter"].includes(e.browserEvent.key)){
			return;
		}
		if(this.moveSelectionOutOfReadOnlyRegion() && ["Backspace", "Tab", "Delete"].includes(e.browserEvent.key)){
			this.preventKeyEvent();
		}
	}

	moveSelectionOutOfReadOnlyRegion(){
		const sel = this.editor.getSelection();
		const newSel = sel.intersectRanges(new monaco.Range(this.readOnlyLines + 1, 1, 10000, 10000));
		if(!newSel){
			this.offset = this.value.length;
			return true;
		}
		this.editor.setSelection(newSel);
		return this.getSelectionTopOffset(sel) !== this.getSelectionTopOffset(newSel);

		return a;
		if(this.readOnlyOffset > this.getSelectionBottomOffset(sel)){
			this.offset = this.value.length;
			return true;
		}
		const startOffset = this.getSelectionStartOffset(sel);
		const endOffset = this.getSelectionEndOffset(sel);
		const newStartOffset = Math.max(startOffset, this.readOnlyOffset);
		const newEndOffset = Math.max(endOffset, this.readOnlyOffset);
		this.editor.setSelection(this.selectionFromOffsets(newStartOffset, newEndOffset));
		return newStartOffset !== startOffset || newEndOffset !== endOffset;
	}


	getSelectionStartOffset(sel){
		return this.editor.getModel().getOffsetAt(new monaco.Position(sel.selectionStartLineNumber, sel.selectionStartColumn));
	}

	getSelectionEndOffset(sel){
		return this.editor.getModel().getOffsetAt(new monaco.Position(sel.positionLineNumber, sel.positionColumn));
	}

	getSelectionTopOffset(sel){
		return this.editor.getModel().getOffsetAt(new monaco.Position(sel.startLineNumber, sel.startColumn));
	}

	getSelectionBottomOffset(sel){
		return this.editor.getModel().getOffsetAt(new monaco.Position(sel.endLineNumber, sel.endColumn));
	}

	selectionFromOffsets(startOffset, endOffset){
		const model = this.editor.getModel();
		const {lineNumber : sline, column : scol} = model.getPositionAt(startOffset);
		const {lineNumber : eline, column : ecol} = model.getPositionAt(endOffset);
		return new monaco.Selection(sline, scol, eline, ecol);
	}

	rangeFromOffsets(startOffset, endOffset){
		const model = this.editor.getModel();
		const {lineNumber : sline, column : scol} = model.getPositionAt(startOffset);
		const {lineNumber : eline, column : ecol} = model.getPositionAt(endOffset);
		return new monaco.Range(sline, scol, eline, ecol);
	}	


	focus(){
		this.editor.focus();
	}

	shouldEnterSubmit(event){
        return !event.ctrlKey && (event.shiftKey || this.input.value.search("\n") === -1);
    }

    shouldStepHistory(event){
        if(event.key === "ArrowUp"){
            return this.input.atStart || this.justSteppedHistory;
        }
        if(event.key === "ArrowDown"){
            return this.input.atEnd || this.justSteppedHistory;
        }
        return false;
	}
	

	async submit(){

        this.input.visible = false;
        this.addInputCell();
        let code = this.input.value;
        this.input.value = "";
        this.history.push(code);
        this.history.clearModifiedStrings();
        await sleep(0);
        console.log(this.history.length);
        this.historyIdx = this.history.length;
        // elt.evaluationStarted();
        let data = await this.executor.execute(code);
        if(data.result_repr !== undefined){
            await this.addOutputCell(data.result_repr);
        }
        this.input.visible = true;
        this.input.focus();
        // elt.evaluationCompleted();
	}
	
	addInputCell(){

	}

    addOutputCell(value){

    }
}
customElements.define('sseq-repl', TerminalElement);