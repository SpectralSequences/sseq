import { SidebarDisplay } from "./display.js"
import { ExtSseq } from "./sseq.js"
import { msgToDisplay, Panel, StructlinePanel, TabbedPanel, ClassPanel } from "./panels.js"
import { renderLaTeX } from "./utils.js"

export class CalculationDisplay extends SidebarDisplay {
    constructor(container, sseqList) {
        super(container);
        this.topMargin = 40;

        this.headerDiv = document.createElement("div");
        this.headerDiv.style.position = "absolute";
        this.headerDiv.style.textOverflow = "ellipsis";
        this.headerDiv.style.overflow = "hidden";
        this.headerDiv.style.whiteSpace = "nowrap";
        this.headerDiv.style.display = "block";
        this.headerDiv.style.width = "100%";
        this.headerDiv.style.left = "20px";
        this.headerDiv.style.top = "7px";
        this.container_DOM.appendChild(this.headerDiv);

        let splitter = sseqList.indexOf("\n");
        let indices = JSON.parse(sseqList.slice(0, splitter));
        this.sseqData = [];

        let x = splitter + 1; // Drop the \n as well
        for (let index of indices) {
            this.sseqData.push(sseqList.slice(x, x + index));
            x += index;
        }

        this.init = JSON.parse(LZString.decompressFromUTF16(this.sseqData.shift()));

        let sseq = ExtSseq.fromJSON(this.init);
        sseq.updateFromJSON(JSON.parse(LZString.decompressFromUTF16(this.sseqData[0])));
        this.setSseq(sseq);
        this.isUnit = this.sseq.isUnit;
        this.idx = 0;

        this.sidebar.footer.newGroup();

        this.sidebar.footer.addButtonRow([
            ["Prev", () => this.prev()],
            ["Next", () => this.next()]
        ]);

        this.sidebar.footer.currentGroup.firstChild.children[0].firstChild.disabled = true;
        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG());

        this.history = [];

        this.generalPanel = new GeneralPanel(this.sidebar.mainDiv, this);
        this.sidebar.addPanel(this.generalPanel);

        this.classPanel = new ClassPanel(this.sidebar.mainDiv, this);
        this.sidebar.addPanel(this.classPanel);

        this.sidebar.currentPanel = this.generalPanel;

        this.on("click", () => {
            if (this.selected)
                this.sidebar.showPanel(this.classPanel);
            else
                this.sidebar.showPanel(this.generalPanel);
        });

        Mousetrap.bind("J", () => this.sidebar.currentPanel.prevTab());
        Mousetrap.bind("K", () => this.sidebar.currentPanel.nextTab());

        this.updateStage();
    }

    next() {
        if (this.idx == this.sseqData.length - 1)
            return;
        this.idx ++;
        this.updateStage();
    }

    prev() {
        if (this.idx == 0)
            return;
        this.idx --;
        this.updateStage();
    }

    updateStage() {
        this.sseq.updateFromJSON(JSON.parse(LZString.decompressFromUTF16(this.sseqData[this.idx])));
        if (this.idx == this.history.length + 1) {
            this.history.push(this.sseq.currentAction);
        }
        // Find a better way to do this.
        if (this.idx == this.sseqData.length - 1) {
            this.sidebar.footer.currentGroup.firstChild.children[1].firstChild.disabled = true;
        } else {
            this.sidebar.footer.currentGroup.firstChild.children[1].firstChild.disabled = false;
        }

        if (this.idx == 0) {
            this.sidebar.footer.currentGroup.firstChild.children[0].firstChild.disabled = true;
        } else {
            this.sidebar.footer.currentGroup.firstChild.children[0].firstChild.disabled = false;
        }

        let result = msgToDisplay(this.sseq.currentAction, this.sseq);
        this.headerDiv.innerHTML = result[0];
        if (this.sseq.currentAction && this.sseq.currentAction["short-note"]) {
            this.headerDiv.innerHTML += " &mdash; " + renderLaTeX(this.sseq.currentAction["short-note"]);
        }
        this.alwaysHighlight = result[1];
        this.clearHighlight();
        this.update();
        this.sidebar.showPanel();
    }
}

export class GeneralPanel extends TabbedPanel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.noteTab = new NotePanel(this.container, this.display);
        this.addTab("Note", this.noteTab);

        this.structlineTab = new StructlinePanel(this.container, this.display);
        this.addTab("Prod", this.structlineTab);

        this.historyTab = new CHistoryPanel(this.container, this.display);
        this.addTab("Hist", this.historyTab);
    }
}

class NotePanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
        this.container.style.fontSize = "90%";
    }

    show() {
        this.container.style.removeProperty("display");
        this.clear();
        this.newGroup();

        let action = this.display.sseq.currentAction;
        if (!action) {
            return;
        }
        let note = this.display.sseq.currentAction["note"];
        if (note) {
            this.currentGroup.innerHTML = renderLaTeX(note);
        } else {
            let shortNote = this.display.sseq.currentAction["short-note"];
            if (shortNote) {
                this.currentGroup.innerHTML = renderLaTeX(shortNote);
            }
        }
    }
}

class CHistoryPanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.clear();
        this.newGroup();
        for (let [i, hist] of this.display.history.entries()) {
            this.addMessage(i, hist);
        }
    }

    addHistoryItem(i, msg, title, highlightClasses, content) {
        let d, s;

        if (content === undefined) {
            d = document.createElement("div");
            s = d;
        } else {
            d = document.createElement("details");
            s = document.createElement("summary");
            d.appendChild(s);
        }
        d.className = "history-item";
        if (i + 1 != this.display.idx) { // History item starts at 1
            d.style.opacity=0.6;
        }
        s.innerHTML = title;

        d.addEventListener("dblclick", () => {
            this.display.idx = i + 1;
            this.display.updateStage();
        });

        if (content !== undefined) {
            let div = document.createElement("div");
            div.className = "text-center py-1";
            div.innerHTML = content;
            d.appendChild(div);
        }

        this.addObject(d);

        d.addEventListener("mouseover", () => {
            d.style.color = "blue";
            for (let pair of highlightClasses) {
                this.display.highlightClass([pair[0], pair[1]]);
            }
            this.display.update();
        });
        d.addEventListener("mouseout", () => {
            d.style.removeProperty("color");
            this.display.clearHighlight();
            this.display.update();
        });

    }

    addMessage(i, data) {
        let result = msgToDisplay(data, this.display.sseq);
        this.addHistoryItem(i, data, ...result);
    }
}
