import { updatePythonLanguageDefinition } from "./pythonTextCompletion";
import { monaco } from "./monaco";
import { sleep } from './utils';
import { PythonExecutor } from "./pythonExecutor";
import { History } from "./history";
import { promiseFromDomEvent } from "./utils"

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
			scrollBeyondLastLine : true,
			contextmenu : false,
			readOnly : true,
			lineNumbers : "none"
		};
    }
    
    static get styles() {
        return `
            .root {
                height : 100%;
                width : 100%;
            }
        `
	}

	get readOnly(){
		return this._readOnly;
	}
	
	set readOnly(v) {
		this._readOnly = !!v;
		// Monaco has an option called "cursorBlinking" that we tried to set to hidden,
		// but it doesn't seem to do anything. Modifying the dom directly works fine though.
		// Cursor blinking happens by toggling visibility, but we can use display without trouble.
		this.querySelector(".cursor").style.display = v ? "none" : "block";
		// console.log(this.querySelector(".cursor").style.display);
		this.querySelector(".view-lines").style.cursor = v ? "default" : "text";
		this.editor.updateOptions({ renderLineHighlight : v ? "none" : "line"});
	}

	get startOfInputPosition(){
		return new monaco.Position(this.readOnlyLines + 1, 1);
	}

	get endOfInputPosition(){
		return this.editor.getModel().getPositionAt(this.editor.getValue().length);
	}

	get endOfInputSelection(){
		return monaco.Selection.fromPositions(this.endOfInputPosition, this.endOfInputPosition);
	}

	get allOfInputSelection(){
		return monaco.Selection.fromPositions(this.startOfInputPosition, this.endOfInputPosition);
	}

	get value(){
		return this.editor.getModel().getValueInRange(this.allOfInputSelection);
	}

	doesSelectionIntersectReadOnlyRegion(sel){
		return this.editor.getSelection().getStartPosition().isBefore(this.startOfInputPosition);
	}

	doesCurrentSelectionIntersectReadOnlyRegion(){
		return this.doesSelectionIntersectReadOnlyRegion(this.editor.getSelection());
	}

	atStartOfInputRegion(){
		return this.editor.getSelection().isEmpty() && this.editor.getPosition().equals(this.startOfInputPosition);
	}

	setSelectionDirection(sel, dir){
		let {lineNumber : sline, column : scol} = sel.getStartPosition();
		let {lineNumber : eline, column : ecol} = sel.getEndPosition();
		return monaco.Selection.createWithDirection(sline, scol, eline, ecol, dir);
	}

	get lineHeight(){
		return Number(this.querySelector(".view-line").style.height.slice(0,-2));
	}

	get screenHeight(){
		return Number(getComputedStyle(document.querySelector("repl-terminal")).height.slice(0,-2));
	}

	get linesPerScreen(){
		return Math.floor(this.screenHeight / this.lineHeight);
	}

	getScreenLinesInModelLine(lineNumber){
		let bottom = this.editor.getTopForPosition(lineNumber, this.editor.getModel().getLineLength(lineNumber));
		let top = this.editor.getTopForLineNumber(lineNumber);
		return Math.floor((bottom - top)/this.lineHeight + 1);
	}
	
	getLastScreenLineOfModelLine(lineNumber){
		let bottom = this.editor.getTopForPosition(lineNumber, this.editor.getModel().getLineLength(lineNumber));
		let top = 0;
		return Math.floor((bottom - top)/this.lineHeight + 1);
	}

	getScreenLineOfPosition(pos){
		let {lineNumber, column} = pos;
		if(!Number.isInteger(lineNumber) || !Number.isInteger(column)){
			throw Error("Invalid position!");
		}
		let bottom = this.editor.getTopForPosition(lineNumber, column);
		let top = 0;
		return Math.floor((bottom - top)/this.lineHeight + 1);
	}

	getModelLineCount(){
		return this.editor.getModel().getLineCount();
	}

	getScreenLineCount(){
		let totalLines = this.editor.getModel().getLineCount();
		return this.getLastScreenLineOfModelLine(totalLines);
	}

	revealSelection(scrollType = monaco.editor.ScrollType.Immediate){
		this.editor.revealRange(this.editor.getSelection(), scrollType);
	}

	getLengthOfLastModelLine(){
		return this.editor.getModel().getLineLength(this.getModelLineCount());
	}		

	focus(){
		this.editor.focus();
	}

	constructor(options){
		super();
		this._focused = false;
		this._visible = true;
		this.editorOptions = Object.assign(ReplElement.defaultEditorOptions, options);
		this.readOnlyLines = 1;
		this.executor = new PythonExecutor();
		this.jedi_history = this.executor.new_completer();
		this.jedi_value = this.executor.new_completer();
		updatePythonLanguageDefinition(monaco, this);
        this.history = new History();
		this.historyIdx = this.history.length || 0;
		this.historyIndexUndoStack = [];
		this.historyIndexRedoStack = [];
		this.firstLines = {};
		this.outputScreenLines = {};
		this.outputModelLines = {};
		// this.outputModelLines[1] = true;
		// this.outputModelLines[2] = true;
		// this.outputScreenLines[1] = true;
		// this.outputScreenLines[2] = true;
		// this.firstLines[2] = true; 
		this.lineOffsetMutationObserver = new MutationObserver((mutationsList, observer) => {
			this.updateLineOffsets();
		});
		// To display our output lines left of our input lines, we need to do the following:
		//    (1) remove contain:strict; and overflow:hide; from .lines-content
		//    (2) remove overflow:hide; from ".monaco-scrollable-element"
		// 	  (3) attach a mutation observer to .view-lines to make sure left-margin is set appropriately on output line content
		//    (4) attach a mutation observer to .view-overlays to make sure left-margin is set appropriately on output line overlays
		// We are doing these things here to ensure that everything continues to work when "setModel" is called.
		// Not clear whether this is necessary / a good idea...
		this.overflowMutationObserver = new MutationObserver((mutationsList, observer) => {
			for(let mutation of mutationsList){
				if(mutation.target.matches(".view-lines")){
					let elt = mutation.target.parentElement;
					if(!elt.matches(".lines-content")){
						throw Error("This shouldn't happen.");
					}
					elt.style.contain = "";
					elt.style.overflow = "";
					elt = elt.parentElement;
					if(!elt.matches(".monaco-scrollable-element")){
						throw Error("This shouldn't happen.");
					}
					elt.style.overflow = "";
					this.lineOffsetMutationObserver.observe(
						this.querySelector(".view-lines"),
						{"childList" : true, "attributeFilter" : ["style"], "subtree" : true}
					);
					this.lineOffsetMutationObserver.observe(
						this.querySelector(".view-overlays"),
						{"childList" : true, "attributeFilter" : ["style"], "subtree" : true}
					);
					this.updateLineOffsets();
				}
			}
		});

		this.syntaxErrorDecorationMutationObserver = new MutationObserver(async (mutationsList) => {
			await sleep(5);
			for(let m of mutationsList){
				this.updateSyntaxErrorDecorationAttributesOnElement(m.target, this.syntaxErrorDecorationAttributes);
			}
		})
	}

	async connectedCallback(){
        let styles = document.createElement("style");
		styles.innerText = ReplElement.styles;
		this.appendChild(styles);

		this.dummyTextArea = document.createElement("textarea");
		this.dummyTextArea.setAttribute("readonly", "");
		this.dummyTextArea.style.position = "absolute";
		this.dummyTextArea.style.opacity = 0;
		
		this.appendChild(this.dummyTextArea);
		let div = document.createElement("div");
		this.overflowMutationObserver.observe(div, {"childList" : true, "subtree" : true});
        div.className = "root";
		this.appendChild(div);
		
		this._resizeObserver = new ResizeObserver(entries => this.editor.layout());
		this._resizeObserver.observe(this);

		this.editor = monaco.editor.create(
            this.querySelector(".root"),
            this.editorOptions
		);

		await sleep(10);
		this.querySelector(".decorationsOverviewRuler").remove();
		this.readOnly = true;
		this._displayLoadingPrompt();
		try {
			await this.executor.ready();
			this._loaded();
		} catch(e){
			this._loadingFailed(e);
		}
	}

	async _displayLoadingPrompt(){
		let idx = 0;
		let loadingSpinner = ["|",  "/", "—", "\\"];
		while(!this.ready){
			idx ++;
			idx = idx % loadingSpinner.length;
			this.editor.setValue(`Loading... ${loadingSpinner[idx]}`);
			await sleep(50);
		}
	}

	async _displaySillyLoadingPrompt(){
		let idx = -1;
		let animation = ["\n\n    -  Loading  -\n\n", "\n\n   -   Loading   -","\n\n   /   Loading   /","\n\n   |   Loading   |","\n\n   \\   Loading   \\","\n\n   -   Loading   -","\n    |\n   /   Loading   /\n                |","     \\\n    |\n   /   Loading   /\n                |\n               \\","     \\-\n    |\n       Loading\n                |\n              -\\","     \\-\n       /\n       Loading\n             /\n              -\\","      -\n       /\n       L|ading\n             /\n              -","\n       /   \\\n       L|ading\n         \\   /\n","          -\n           \\\n       L|ading\n         \\\n          -","         /-\n           \\\n       Loading\n         \\\n          -/","        |/-\n\n       Loading\n\n          -/|","        |/\n      \\\n       Loading\n              \\\n           /|","        |\n      \\\n    -  Loading  -\n              \\\n            |","\n      \\\n    -  Loading  -\n              \\\n","\n\n    -  Loading  -\n\n","\n\n    -  Loading  -\n\n","\n\n    -  Loading  -\n\n","\n\n    -  Loading  -\n\n","\n\n    -  Loading  -\n\n","\n\n    -  Loading  -\n\n"];

		let observer = new MutationObserver(() => {
			for(let c of document.querySelectorAll(".cigr, .cigra")){
				if(c.style.display !== "none"){
					c.style.display = "none";
				}
			}
		});
		observer.observe(this, {"subtree" : true, "childList" : true, "attributesFilter" : ["style"] });
		while(!this.ready){
			idx ++;
			idx = idx % animation.length;
			this.editor.setValue(animation[idx]);
			if(idx === 0) {
				await sleep(250);
			}
			await sleep(50);
		}
		observer.disconnect();
	}


	async _loaded(){
		this.ready = true;
		this.editor.onKeyDown(this._onkey.bind(this));
		this.editor.onMouseDown(this._onmousedown.bind(this));
		this.editor.onMouseUp(this._onmouseup.bind(this));
		this.editor.onDidChangeCursorSelection(this._onDidChangeCursorSelection.bind(this));
		this.editor.onDidScrollChange(() => this.updateLineOffsets());
		this.editor.onDidChangeModelContent(() => { 
			this.clearSyntaxError();
		});
		// this.editor.onDidChangeModelDecorations(() => {
		// 	console.log("decs:", this.editor.getLineDecorations(this.editor.getPosition().lineNumber));
		// 	if(this.syntaxErrorWidget){
		// 		let elts = [
		// 			document.querySelector(".repl-error-decoration-highlight"),
		// 			document.querySelector(".repl-error-decoration-text"),
		// 			document.querySelector(".repl-error-decoration-underline"),
		// 			document.querySelector(".repl-error-widget"),
		// 		];
		// 		for(let elt of elts){
		// 			if(!elt){
		// 				continue;
		// 			}
		// 			elt.setAttribute("active");
		// 		}
		// 	}
		// })
		this.editor.updateOptions({ "readOnly" : false });
		window.addEventListener("cut", this._oncut.bind(this));
		this.editor.updateOptions({
			lineNumbers : this._getEditorLinePrefix.bind(this)
		});

		this.outputModelLines = { 1 : true};
		this.outputScreenLines = { 1 : true };
		this.firstLines[2] = true;
		this.editor.setValue("\n");
		this.editor.setPosition(this.endOfInputPosition);
		this.focus();
		await sleep(0);
		this.readOnly = false;
	}

	_loadingFailed(e){
		this.ready = true;
		let observer = new MutationObserver(() => {
			for(let c of document.querySelectorAll(".cigr, .cigra")){
				if(
					Number(c.parentElement.style.top.slice(0,-2))/27 >= 2
					&& c.style.left === "0px"
				){
					continue;
				}
				if(c.style.display !== "none"){
					c.style.display = "none";
				}
			}
		});
		observer.observe(this, {"subtree" : true, "childList" : true, "attributesFilter" : ["style"] });
		this.editor.updateOptions({
			lineNumbers : (n) => {
				if(n in this.outputModelLines){
					return "";
				}
				return "   ";
			}
		});		
		this.editor.setValue(`Fatal Error! \n \n ${e.toString().replace(/\n/g,"\n ")}`);
		this.editor.updateOptions({
			rulers : []
		});
	}

	_getEditorLinePrefix(n){
		if(n in this.firstLines){
			return ">>>";
		}
		if(n in this.outputModelLines){
			return "";
		}
		return "...";
	}

	updateCursorOffset(){
		let cursorLineNumber = this.editor.getPosition().lineNumber;
		this.querySelector(".cursor").style.marginLeft = cursorLineNumber in this.outputModelLines ? "-4ch" : "";
	}

	updateLineOffsets(){
		const outputLines = 
			Array.from(document.querySelectorAll(".view-line"))
				.map(e => [e, Number(e.style.top.slice(0, -2))]).sort((a,b) => a[1]-b[1]).map(e => e[0]);
		const topScreenLine = Math.floor(this.editor.getScrollTop() / this.lineHeight) + 1;
		outputLines.forEach((e, idx) => {
			let targetMargin = (topScreenLine + idx) in this.outputScreenLines ? "-4ch" : "";
			if(e.style.marginLeft !== targetMargin){
				e.style.marginLeft = targetMargin;
			}
		});
		const outputOverlays = 
			Array.from(document.querySelector(".view-overlays").children)
				.map(e => [e, Number(e.style.top.slice(0, -2))]).sort((a,b) => a[1]-b[1]).map(e => e[0]);
		outputOverlays.forEach((e, idx) => {
			let targetMargin = (topScreenLine + idx) in this.outputScreenLines ? "-5ch" : "";
			if(e.style.marginLeft !== targetMargin){
				e.style.marginLeft = targetMargin;
			}
		});
	}	

	_onmousedown(){
		// this.mouseDown = true;
		// this.justSteppedHistory = false;
		// this.updateCursorOffset();
	}

	_onmouseup(){
		// this.mouseDown = false;
		// if(this.doesSelectionIntersectReadOnlyRegion() && this.editor.getSelection().isEmpty()){
		// 	this.editor.setPosition(this.endOfInputPosition);
		// 	this.revealSelection();
		// 	this.updateCursorOffset();
		// }
	}	

	async _onDidChangeCursorSelection(e){
		// This means this was a history change event.
		// We use this jank secondarySelection approach to signal a history change event because
		// monaco's edit stack API doesn't let us attach our own undo/redo callback.
		if(e.secondarySelections.length > 0){
			this.editor.setSelection(e.selection);
			// If the source is the keyboard, the user directly changed the history and stepHistory() covered everything.
			if(e.source !== "keyboard"){
				return;
			}
			if(e.reason === monaco.editor.CursorChangeReason.Redo){
				this.history.undoStep();
				return;
			}
			if(e.reason === monaco.editor.CursorChangeReason.Undo){
				this.history.redoStep();
				return;
			}
			throw Error("Unreachable?");
		}
		// Hide line selection if in read only region or if whole editor is read only.
		// This improves feedback by quite a lot.
		if(this.doesCurrentSelectionIntersectReadOnlyRegion()){
			this.editor.updateOptions({ renderLineHighlight : "none" });
			this.querySelector(".cursor").style.display = "none";
			await sleep(0);
			this.querySelector(".cursor").style.display = "none";
		} else {
			this.editor.updateOptions({ renderLineHighlight : "line" });
			this.querySelector(".cursor").style.display = this.readOnly ? "none" : "block";
		}
		// await sleep(0); // Need to sleep(0) to allow this.mouseDown to update.
		// if(!this.mouseDown && e.selection.isEmpty() && this.doesCurrentSelectionIntersectReadOnlyRegion()){
		// 	this.editor.setPosition(this.endOfInputPosition);
		// 	this.revealSelection();
		// }
		// this.updateCursorOffset();
	}


	/** 
	*   e.preventDefault() doesn't work on certain keys, including Backspace, Tab, and Delete.
	* 	As a recourse to prevent these rogue key events, we move the focus out of the input area 
	*  	for just a moment and then move it back.
	*   The .dummy should be a readonly textarea element.
	*/
	async preventKeyEvent(){
		this.dummyTextArea.focus();
		await sleep(0);
		this.focus();
	}


	enforceReadOnlyRegion(e){
		if(e.browserEvent.type !== "keydown"){
			return;
		}
		if(
			(e.browserEvent.ctrlKey || e.browserEvent.altKey || !/^[ -~]$/.test(e.browserEvent.key)) 
			&& !["Backspace", "Tab", "Delete", "Enter"].includes(e.browserEvent.key)
		){
			return;
		}
		this.intersectSelectionWithInputRegion();
	}

	intersectSelectionWithInputRegion(){
		const sel = this.editor.getSelection();
		let newSel = sel.intersectRanges(this.allOfInputSelection);
		if(newSel){
			newSel = this.setSelectionDirection(newSel, sel.getDirection());
			this.editor.setSelection(newSel);
		} else {
			this.editor.setPosition(this.endOfInputPosition);
		}
		this.revealSelection();
		return !newSel || !sel.equalsSelection(newSel);
	}


	_onkey(e){
		// Test if completion suggestion widget is visible.
		// Usually it is hidden by removing the ".visible" class, but in some unusual circumstances
		// it ends up with height 0 but still ".visible". So we also test the height.
		let elt = this.querySelector(".suggest-widget.visible");
		let suggestWidgetVisibleQ = elt && elt.offsetHeight > 0;
		if(suggestWidgetVisibleQ){
			if(
				// This annoying conditional spells out which keys we want the suggestWidget to handle
				(e.browserEvent.key !== "PageUp" || !e.browserEvent.shiftKey)
				&& (!e.browserEvent.ctrlKey || !["x", "Home", "a"].includes(e.browserEvent.key))
				&& ((!e.shiftKey && !e.altKey) || e.browserEvent.key !== "ArrowUp")
			){
				return;
			}
		}
		if(this.maybeStepHistory(e.browserEvent)){
			this.preventKeyEvent();
			return
		}
		this.justSteppedHistory = false;
		// Many Ctrl + <some-key> commands require special handling.
		if(e.browserEvent.ctrlKey){
			if(e.browserEvent.key in ReplElement._ctrlCmdHandlers){
				ReplElement._ctrlCmdHandlers[e.browserEvent.key].call(this, e);
				return
			}
		}
		if(e.browserEvent.key === "PageUp"){
			this._onPageUp(e);
			return;
		}

		// Prevent keys that would edit value if read only
		if(this.readOnly && !e.browserEvent.altKey){
			if(
				!e.browserEvent.ctrlKey && /^[ -~]$/.test(e.browserEvent.key) 
				|| ["Backspace", "Tab", "Delete", "Enter"].includes(e.browserEvent.key)
			){
				this.preventKeyEvent();
				return;
			}
		}
		// Prevent backing up past start of repl input
		if(this.atStartOfInputRegion() && ["ArrowLeft", "Backspace"].includes(e.browserEvent.key)){
			this.preventKeyEvent();
			return;
		}
		this.enforceReadOnlyRegion(e);
		if(e.browserEvent.key === "Escape"){
			if(this.currentExecution){
				this.currentExecution.keyboardInterrupt();
			}
		}

		if(e.browserEvent.key === "Enter") {
			if(this.shouldEnterSubmit(e.browserEvent)){
				this.preventKeyEvent();
				this.submit();
				return;
			}
			if(e.browserEvent.shiftKey){
				return;
			}
			let currentLineContent = this.editor.getModel().getLineContent(this.editor.getPosition().lineNumber);
			// If the current line is empty except for spaces, outdent it.
			// TODO: Good choice or no?
			if(/^ +$/.test(currentLineContent)){
				this.editor.getAction("editor.action.outdentLines").run();
				this.preventKeyEvent();
			}
		}		
	}

	static get _ctrlCmdHandlers(){
		return {
			"c" : ReplElement.prototype._onCtrlC,
			"v" : ReplElement.prototype._onCtrlV,
			"x" : ReplElement.prototype._onCtrlX,
			"a" : ReplElement.prototype._onCtrlA,
			"Home" : ReplElement.prototype._onCtrlHome
		};
	}

	_onCtrlC() {}

	_onCtrlV(e) {
		if(e.browserEvent.altKey){
			// Ctrl+alt+V does nothing by default.
			return;
		}
		if(this.readOnly){
			this.preventKeyEvent();
		}
		if(this.doesCurrentSelectionIntersectReadOnlyRegion()){
			this.preventKeyEvent();
			this.editor.setPosition(this.endOfInputPosition);
			this.revealSelection();
		}
	}

	_onCtrlX() {
		// In these cases, we do special handling in the "window.oncut" event listener see _oncut
		if(
			this.readOnly ||
			this.doesCurrentSelectionIntersectReadOnlyRegion() 
			|| this.value.indexOf("\n") === -1 && this.editor.getSelection().isEmpty()
		){
			this.preventKeyEvent();
		}
		// Otherwise allow default behavior
		return
	}

	_oncut(e) {
		e.preventDefault();
		if(this.editor.getSelection().isEmpty()){
			if(this.value.indexOf("\n") === -1 || this.readOnly){
				event.clipboardData.setData('text/plain', this.value + "\n");
				this.editor.pushUndoStop();
				this.editor.executeEdits(
					"cut", 
					[{
						range : this.allOfInputSelection,
						text : ""
					}],
					[this.endOfInputSelection]
				);
				this.editor.pushUndoStop();
				this.preventKeyEvent();
				return;
			}
			// Else allow default handling
			return;
		}
		if(this.doesCurrentSelectionIntersectReadOnlyRegion() || this.readOnly){ 
			event.clipboardData.setData('text/plain', this.editor.getModel().getValueInRange(this.editor.getSelection()));
			this.preventKeyEvent();
			return;
		}
		// Else allow default handling
	}

	_onCtrlA(){
		if(this.altKey){
			// By default if alt key is pressed nothing happens.
			return;
		}
		if(this.doesCurrentSelectionIntersectReadOnlyRegion()){
			// If the current selection intersects read only region, allow default behavior
			// which selects everything
			return;
		}
		// Otherwise, we only want to select the input region
		this.editor.setSelection(this.allOfInputSelection);
		this.preventKeyEvent();
	}

	_onCtrlHome(e){
		if(e.altKey){
			// By default nothing happens if alt key is pressed.
			return;
		}
		// If we are in the read only region
		if(this.doesCurrentSelectionIntersectReadOnlyRegion()){
			// And the user is holding shift, allow default behavior
			if(e.browserEvent.shiftKey){
				return;
			}
			// If in read only region but user is not holding shift, move to start of input region
			this.editor.setPosition(this.startOfInputPosition);
			this.revealSelection();
			this.preventKeyEvent();
			return;
		}
		// If in input region, select from start of current selection to start of input region
		if(e.browserEvent.shiftKey){
			this.editor.setSelection(this.editor.getSelection().setEndPosition(this.startOfInputPosition));
		} else {
			this.editor.setPosition(this.startOfInputPosition);
		}
		this.revealSelection();
		this.preventKeyEvent();
	}

	_onPageUp(e){
		if(e.browserEvent.altKey){
			// alt key means move screen not cursor. So default handling is always fine.
			return;
		}
		// If we are in the read only region
		if(this.doesCurrentSelectionIntersectReadOnlyRegion()){
			// And the user is holding shift, allow default behavior
			if(e.browserEvent.shiftKey){
				return;
			}
			// If in read only region but user is not holding shift, move to start of input region
			this.editor.setPosition(this.startOfInputPosition);
			this.revealSelection();
			this.preventKeyEvent();
			return;				
		}
		// If we would page up into read only region, select input region up to beginning.
		let targetLine = this.getScreenLineOfPosition(this.editor.getPosition()) - this.linesPerScreen + 1;
		if(targetLine <= this.getLastScreenLineOfModelLine(this.readOnlyLines)){
			if(e.browserEvent.shiftKey){
				this.editor.setSelection(this.editor.getSelection().setEndPosition(this.startOfInputPosition));
			} else {
				this.editor.setPosition(this.startOfInputPosition);
			}
			this.revealSelection();
			this.preventKeyEvent();
		}
		// Otherwise, allow default page up behavior
	}

	async nextHistory(n = 1){
        await this.stepHistory(n);
    }
    
    async previousHistory(n = 1){
        await this.stepHistory(-n);
    }

    async stepHistory(didx) {
		this.history.setTemporaryValue(this.value);
		const changed = this.history.step(didx);
		if(!changed){
			return
		}
		this.editor.pushUndoStop();
		// Use a multi cursor before and after the edit operation to indicate that this is a history item
		// change. In onDidChangeCursorSelection will check for a secondary selection and use the presence 
		// of it to deduce that a history step occurred. We then immediately clear the secondary selection
		// to prevent the user from ever seeing it.
		this.editor.getModel().pushEditOperations(
			[this.editor.getSelection(), new monaco.Selection(1, 1, 1, 1)], 
			[{
				range : this.allOfInputSelection,
				text : (await this.history.value) || ""
			}],
			() => [this.endOfInputSelection, new monaco.Selection(1, 1, 1, 1)]
		);
		this.editor.pushUndoStop();
		// For some reason we end up selecting the input. This is maybe a glitch with pushEditOperations?
		// It doesn't happen with this.editor.executeEdits, but using executeEdits doesn't work because
		// for some reason we can't convince it that the before cursor state is a multi cursor like we need 
		// for this strategy.
		this.editor.setPosition(this.endOfInputPosition);
		this.revealSelection();
		// Record how much the history index changed for undo.
		this.historyIndexRedoStack = [];
		this.historyIndexUndoStack.push(didx);
		this.justSteppedHistory = true;
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
		if(!["ArrowUp", "ArrowDown"].includes(event.key)){
			return false;
		}
		if(this.justSteppedHistory){
			return true;
		}
		if(!this.editor.getSelection().isEmpty()){
			return false;
		}
		let pos = this.editor.getPosition();
        if(event.key === "ArrowUp"){
			return this.getScreenLineOfPosition(pos) === this.getScreenLineOfPosition(this.startOfInputPosition);
        }
        if(event.key === "ArrowDown"){
            return this.getScreenLineOfPosition(pos) === this.getScreenLineOfPosition(this.endOfInputPosition);
        }
        throw Error("Unreachable");
	}

	shouldEnterSubmit(event){
		if(event.shiftKey){
			return false;
		}
		if(event.ctrlKey){
			return true;
		}
		if(this.editor.getPosition().lineNumber !== this.getModelLineCount()){
			return false;
		}
		if(this.getLengthOfLastModelLine() == 0){
			return true;
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
		if(this.syntaxErrorWidget){
			// Don't do anything if there's already a syntax error...
			return;
		}
		const code = this.value;
		if(!code.trim()){
			return;
		}
		// this.editor.setValue(this.editor.getValue().trimEnd());
		// this.printToConsole("\n");
		// await sleep(0);
		// editor.setValue seems to undo changes to the console so readOnly has to be set second
		// and we need to sleep first.
		this.readOnly = true; 
		// execution is a handle we can use to cancel.
		const execution = this.executor.execute(code);
		this.currentExecution = execution;
		execution.onStdout((data) => this.printToConsole(data));
		execution.onStderr((data) => this.printToConsole(data));
		let syntaxCheck = await execution.validate_syntax(code);
		if(!syntaxCheck.valid){
			await sleep(0);
			this.showSyntaxError(syntaxCheck.errors);
			this.currentExecution = undefined;
			this.readOnly = false;
			// await sleep(0);
			// this.editor.setPosition(this.endOfInputPosition);
			return;
		}

		this.editor.setValue(this.editor.getValue().trimEnd());
		this.printToConsole("\n");
		await sleep(0);
		this.history.push(code);
        await sleep(0);
		this.historyIdx = this.history.length;

		let result;
		try {
			result = await execution.result();
			this.addOutput(result);
		} catch(e) {
			this.addOutput(e.traceback);
		}
		this.currentExecution = undefined;
		this.prepareInput();
		await sleep(0);
		this.readOnly = false;
	}

	addOutput(value){
		// If something printed to stdout but didn't end the line, we'll have a nonempty line
		// at the bottom. We don't want our output sharing with that, so insert a newline in 
		// this case.
		if(this.getLengthOfLastModelLine() !== 0){
			this.printToConsole("\n");
		}
		if(value === undefined){
			return;
		}
		this.printToConsole(`${value}\n`);
	}

    printToConsole(value){
		const oldNumScreenLines = this.getScreenLineCount();
		const oldNumModelLines = this.getModelLineCount();
		this.editor.setValue(`${this.editor.getValue()}${value}`);
		const newNumScreenLines = this.getScreenLineCount();
		const newNumModelLines = this.getModelLineCount();
		for(let curLine = oldNumModelLines + 1; curLine <= newNumModelLines; curLine++){
			this.outputModelLines[curLine] = true;
		}
		for(let curLine = oldNumScreenLines + 1; curLine <= newNumScreenLines; curLine++){
			this.outputScreenLines[curLine] = true;
		}
	}

	prepareInput(){
		const numScreenLines = this.getScreenLineCount();
		let numModelLines = this.getModelLineCount();
		if(this.getLengthOfLastModelLine() === 0){
			// Usually there is an empty model line at the bottom, it has been marked as an output line
			// so onmark it.
			delete this.outputModelLines[numModelLines];
			delete this.outputScreenLines[numScreenLines];
		} else {
			// Ocassionally something printed stuff to stdout but didn't terminate the line.
			// We terminate it ourselves in that case to prevent weirdness.
			this.editor.setValue(this.editor.getValue() + "\n");
			numModelLines++;
		}
		this.readOnlyLines = numModelLines - 1;
		this.firstLines[numModelLines] = true;
		// the value of firstLines signals that the "line number" should show as a ">>>" prompt.
		// we need to refresh the editor to get it to show up though so set value to current value.
		this.editor.setValue(this.editor.getValue());		
		
		this.editor.setPosition(this.endOfInputPosition);
		this.revealSelection();
	}

	updateSyntaxErrorDecorationAttributes(state){
		let elts = [
			document.querySelector(".repl-error-decoration-highlight"),
			document.querySelector(".repl-error-decoration-text"),
			document.querySelector(".repl-error-decoration-underline"),
			document.querySelector(".repl-error-widget"),
		];
		for(let elt of elts){
			if(!elt){
				continue;
			}
			this.updateSyntaxErrorDecorationAttributesOnElement(elt, state);
		}
	}	

	updateSyntaxErrorDecorationAttributesOnElement(elt, state){
		for(let [k, v] of Object.entries(state)){
			if(v === false && elt.hasAttribute(k)){
				elt.removeAttribute(k);
			} else if(elt.getAttribute(k) !== v){
				elt.setAttribute(k, v);
			}
		}
	}

	async clearSyntaxError(){
		if(!this.syntaxErrorWidget){
			return;
		}
		this.syntaxErrorDecorationAttributes = { "transition" : "hide", "active" : false };
		this.updateSyntaxErrorDecorationAttributes(this.syntaxErrorDecorationAttributes);
		await promiseFromDomEvent(document.querySelector(".repl-error-widget"), "transitionend");
		this.syntaxErrorDecorationAttributes = { "transition" : false, "active" : false };
		this.updateSyntaxErrorDecorationAttributes(this.syntaxErrorDecorationAttributes);
		this.editor.removeContentWidget(this.syntaxErrorWidget);
		this.editor.deltaDecorations(this.decorationIds || [], []);
		this.decorationIds = [];
		this.syntaxErrorWidget = undefined;
	}
	
	async showSyntaxError(errors){
		// TODO: maybe handle multiple syntax errors case...? Probably not important...
		console.log(errors);
		let error = errors[0];
		let [start_line, start_col] = error.start_pos;
		start_line += this.readOnlyLines;
		let [end_line, end_col] = error.end_pos;
		end_line += this.readOnlyLines;		
		let decorations = [{
			range: new monaco.Range(start_line, 1, end_line, 1),
			options: {
				isWholeLine: true,
				afterContentClassName: 'repl-error repl-error-fade-in repl-error-decoration-underline',
			}
		}];
		// console.log([start_line, start_col, end_line, end_col + 1]);
		if(start_line !== end_line || start_col != end_col){
			// let endCol = this.editor.getModel().getLineMaxColumn(line);
			decorations.push({
				range: new monaco.Range(start_line, start_col, end_line, end_col + 1),
				options: {
					// isWholeLine: true,
					className: 'repl-error repl-error-fade-in repl-error-decoration-highlight',
					inlineClassName : 'repl-error repl-error-decoration-text'
				}
			});
		}
		await sleep(0);
		this.editor.deltaDecorations(this.decorationsIds || [], []);
		await sleep(0);
		this.decorations = decorations;
		this.decorationIds = this.editor.deltaDecorations([], decorations);
		// this.editor.onDidChangeModelDecorations((e) => {
		// 	console.log("decs:", this.editor.getLineDecorations(this.editor.getPosition().lineNumber))
		// });
		this.syntaxErrorWidget = {
			domNode: null,
			getId: function() {
				return 'error.widget';
			},
			getDomNode: function() {
				if (!this.domNode) {
					this.domNode = document.createElement('div');
					this.domNode.innerText = `${error.msg}`; //`${error.type}: ${error.msg}`;
					this.domNode.classList.add("repl-error");
					this.domNode.classList.add("repl-error-fade-in");
					this.domNode.classList.add("repl-error-widget");
				}
				return this.domNode;
			},
			getPosition: function() {
				return {
					position: {
						lineNumber: start_line, // Is this the best line to choose?
						column: 1
					},
					preference: [monaco.editor.ContentWidgetPositionPreference.BELOW]
				};
			}
		};
		this.editor.addContentWidget(this.syntaxErrorWidget);
		await sleep(10);
		document.querySelector(".repl-error-widget")
		// What is this supposed to do??
		let elts = [
			document.querySelector(".repl-error-decoration-highlight"),
			document.querySelector(".repl-error-decoration-text"),
			document.querySelector(".repl-error-decoration-underline"),
		];
		// for(let elt of elts){
		// 	if(!elt){
		// 		continue;
		// 	}
		// 	this.syntaxErrorDecorationMutationObserver.observe(
		// 		elt,
		// 		{ "attributeFilter" : ["transition", "active"] }
		// 	);
		// }


		this.syntaxErrorDecorationAttributes = { "transition" : "show", "active" : "" };
		this.updateSyntaxErrorDecorationAttributes(this.syntaxErrorDecorationAttributes);
		await promiseFromDomEvent(document.querySelector(".repl-error-widget"), "transitionend");
		this.syntaxErrorDecorationAttributes = { "transition" : false, "active" : "" };
		this.updateSyntaxErrorDecorationAttributes(this.syntaxErrorDecorationAttributes);
	}
}
customElements.define('repl-terminal', ReplElement);