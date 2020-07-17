import { css } from "lit-element";
import { updatePythonLanguageDefinition } from "./pythonTextCompletion";
import { monaco } from "./monaco";
import { sleep } from './utils';
import { PythonExecutor } from "./pythonExecutor";
import { History } from "./history";

// function sleep(ms) {
//     return new Promise(resolve => setTimeout(resolve, ms));
// }

updatePythonLanguageDefinition(monaco);
function isSelectionNonempty(sel){
	return sel.startLineNumber < sel.endLineNumber || sel.startColumn < sel.endColumn;
}


class ReplElement extends HTMLElement {
	static get defaultEditorOptions(){
		return {
			value: "",
			language: "python",
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
			// "autoIndent": true
			scrollBeyondLastLine : true,
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

	getPositionAt(offset){
		return this.editor.getModel().getPositionAt(offset);
	}

	getOffsetAt(offset){
		return this.editor.getModel().getOffsetAt(offset);
	}

	get position(){
		return this.editor.getPosition();
	}

	set position(v){
		this.editor.setPosition(v);
	}

	get offset(){
		return this.getOffsetAt(this.position);
	}

	set offset(v){
		this.position = this.getPositionAt(v);
	}

	get endPosition(){
		return this.getPositionAt(this.editor.getValue().length);
	}

	get value(){
		return this.editor.getModel().getValueInRange(this.rangeFromOffsets(this.readOnlyOffset, this.editor.getValue().length));
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

	constructor(options){
		super();
		this._focused = false;
		this._visible = true;
		this.editorOptions = Object.assign(ReplElement.defaultEditorOptions, options);
		this._readOnlyLines = 0;
		this._readOnlyOffset = 0;
		this.readOnly = false;
        // window.addEventListener("pagehide", async (event) => {
        //     localStorage.setItem("pageHide", true);
        //     await this.history.writeToLocalStorage();
        // });
        this.executor = new PythonExecutor();
        this.history = new History();
		this.historyIdx = this.history.length;
		this.firstLines = {};
		this.outputLines = {};
		this.firstLines[1] = true; 
	}

	connectedCallback(){
        let styles = document.createElement("style");
        styles.innerText = ReplElement.styles;
        this.appendChild(styles);
        let div = document.createElement("div");
        div.id = "root";
        this.appendChild(div);
		this.editor = monaco.editor.create(
            this.querySelector("div"),
            this.editorOptions
        );
		this.editor.updateOptions({
			lineNumbers : (n) => {
				if(n in this.firstLines){
					return ">>>";
				}
				if(n in this.outputLines){
					return "";
				}
				return "...";
			}
		});
		sleep(10).then(() => this.fixOutputPosition());
		this._resizeObserver = new ResizeObserver(entries => {
			this.editor.layout();
		});
		this._resizeObserver.observe(this);			
		this.editor.onKeyDown(this._onkey.bind(this));
		this.editor.onDidChangeCursorSelection((e) => {
			if(isSelectionNonempty(e.selection)){
				this.fixCursorOutputPosition();
				return;
			}
			if(this.offset < this.readOnlyOffset){
				this.editor.setPosition(this.editor.getModel().getPositionAt(this.readOnlyOffset));
			}
			this.fixCursorOutputPosition();
		});
		this.errorWidget = {
			domNode: null,
			getId: function() {
				return 'error.widget';
			},
			getDomNode: function() {
				if (!this.domNode) {
					this.domNode = document.createElement('div');
					this.domNode.innerHTML = 'My content widget';
					this.domNode.style.background = 'grey';
				}
				return this.domNode;
			},
			getPosition: function() {
				return {
					position: {
						lineNumber: 11,
						column: 8
					},
					preference: [monaco.editor.ContentWidgetPositionPreference.BELOW]
				};
			}
		};
		this.editor.addContentWidget(this.errorWidget);
		
	}

	fixOutputPosition(){
		this.querySelector(".decorationsOverviewRuler").remove();
		this.querySelector(".monaco-scrollable-element").style.overflow = "";
		this.querySelector(".lines-content").style.overflow = "";
		this.querySelector(".lines-content").style.contain = "";
		this.lineContentMutationObserver = new MutationObserver((mutationsList, observer) => {
			for(let mutation of mutationsList){
				for(let addedNode of mutation.addedNodes){
					let idx = Array.from(addedNode.parentElement.children).indexOf(addedNode);
					if((idx + 1) in this.outputLines){
						addedNode.style.marginLeft = "-4ch";
					}
				}
			}
		});
		this.lineContentMutationObserver.observe(
			this.querySelector(".view-lines"),
			{"childList" : true}
		);
		this.viewOverlaysMutationObserver = new MutationObserver((mutationsList, observer) => {
			for(let mutation of mutationsList){
				for(let addedNode of mutation.addedNodes){
					let idx = Array.from(addedNode.parentElement.children).indexOf(addedNode);
					if((idx + 1) in this.outputLines){
						addedNode.style.marginLeft = "-5ch";
					}
				}
			}
		});
		this.viewOverlaysMutationObserver.observe(
			this.querySelector(".view-overlays"),
			{"childList" : true}
		);
	}


	fixCursorOutputPosition(){
		let cursorLineNumber = this.editor.getPosition().lineNumber;
		this.querySelector(".cursor").style.marginLeft = cursorLineNumber in this.outputLines ? "-4ch" : "";
	}
	
	async nextHistory(n = 1){
        await this.stepHistory(n);
    }
    
    async previousHistory(n = 1){
        await this.stepHistory(-n);
    }

    async stepHistory(didx) {
		return;
        this.historyIdx = Math.min(Math.max(this.historyIdx + didx, 0), this.history.length);
        this.value = await this.history[this.historyIdx] || "";
        await sleep(0);
        this.offset = this.value.length; // Set cursor at end
	}

	
	maybeStepHistory(event){
		let result = this.shouldStepHistory(event);
		if(result){
			let dir = {"ArrowUp" : -1, "ArrowDown" : 1 }[event.key];
			this.stepHistory(dir);
		}
		return result;
	}

    shouldStepHistory(event){
		if(this.justSteppedHistory){
			return true;
		}
        if(event.key === "ArrowUp"){
            return this.position.lineNumber === this.readOnlyLines + 1;
        }
        if(event.key === "ArrowDown"){
            return this.position.lineNumber === this.endPosition.lineNumber;
        }
        return false;
	}
		
	
	_onkey(e){
		if(this.readOnly){
			this.preventKeyEvent();
			return;
		}
		if(this.offset === this.readOnlyOffset && e.browserEvent.key === "Backspace"){
			this.preventKeyEvent();
			return;
		}
		if(this.maybeStepHistory(event)){
			return
		}
		this.enforceReadOnlyRegion(e);
		if(e.browserEvent.key === "Enter") {
			if(this.shouldEnterSubmit(e.browserEvent)){
				this.preventKeyEvent();
				this.submit();
			}
		}
	}

	/** 
	*   e.preventDefault() doesn't work on certain keys, including Backspace, Tab, and Delete.
	* 	As a recourse to prevent these rogue key events, we move the focus out of the input area 
	*  	for just a moment and then move it back.
	*/
	preventKeyEvent(){
		document.querySelector(".dummy").focus();
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
		if(event.ctrlKey){
			return true;
		}
		if(event.shiftKey){
			return false;
		}
		if(this.value.search("\n") >= 0){
			return false;
		}
		if(/(:|\\)\s*$/.test(this.value)){
			return false;
		}
        return true;
	}


	async submit(){
		const code = this.value.trim();
		if(!code){
			return;
		}
		this.readOnly = true;
		let syntaxCheck = await this.executor.validate(code);
		if(!syntaxCheck.validated){
			console.log(syntaxCheck.error);
			this.readOnly = false;
			return;
		}
        this.history.push(code);
        await sleep(0);
        this.historyIdx = this.history.length;
        const data = await this.executor.execute(code);
        if(data.result_repr !== undefined){
            this.addOutput(data.result_repr);
		} else {
			this.editor.setValue(`${this.editor.getValue()}\n`);
		}
		const totalLines = this.editor.getModel().getLineCount();
		this.readOnlyLines = totalLines - 1;
		this.position = new monaco.Position(totalLines, 1);
		this.firstLines[totalLines] = true;
		this.readOnly = false;
	}

    addOutput(value){
		const totalLines = this.editor.getModel().getLineCount();
		this.editor.setValue(`${this.editor.getValue()}\n${value}\n`);
		let outputLines = this.editor.getModel().getLineCount() - totalLines - 1;
		for(let curLine = totalLines + 1; curLine <= totalLines + outputLines; curLine++){
			this.outputLines[curLine] = true;
		}
	}
	
	updateOutputLines(){
		for(let idx in this.outputLines){
			this.querySelectorAll(".view-line")[idx - 1].style.marginLeft = "-4ch";
		}
	}
}
customElements.define('repl-terminal', ReplElement);