import { SidebarDisplay } from "./display.js"
import { ExtSseq } from "./sseq.js"
import { msgToDisplay, Panel, StructlinePanel, TabbedPanel, ClassPanel } from "./panels.js"
import { download, renderLaTeX, renderLaTeXP, inflate, deflate } from "./utils.js"

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

        let lengths_ = new Uint32Array(sseqList, 0, Math.floor(sseqList.byteLength / 4));
        this.lengths = [];
        for (let len of lengths_) {
            if (len == 0) break;;
            this.lengths.push(len);
        }
        this.sseqData = [];

        let x = (this.lengths.length + 1) * 4; // Drop the \n as well
        for (let len of this.lengths) {
            this.sseqData.push(new Uint8Array(sseqList, x, len));
            x += len;
        }

        this.init = this.sseqData.shift();
        this.historyRaw = this.sseqData.shift();
        this.history = inflate(this.historyRaw).split("\n").map(JSON.parse);

        let sseq = ExtSseq.fromBinary(this.init);
        sseq.updateFromBinary(this.sseqData[0]);
        this.setSseq(sseq);
        this.isUnit = this.sseq.isUnit;
        this.idx = this.sseqData.length - 1;

        this.sidebar.footer.newGroup();

        this.sidebar.footer.addButtonRow([
            ["Prev", () => this.prev()],
            ["Next", () => this.next()]
        ]);

        this.sidebar.footer.currentGroup.firstChild.children[0].firstChild.disabled = true;
        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG());

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

    downloadHistoryFile() {
        lines = [this.init, this.historyRaw].concat(this.sseqData);

        let filename = prompt("History file name");
        if (filename === null) return;
        filename = filename.trim();

        let lengths = lines.map(x => x.length);
        lengths.push(0);

        download(filename, [Uint32Array.from(lengths)].concat(lines), "application/octet-stream");
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
        this.sseq.updateFromBinary(this.sseqData[this.idx]);
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

        let history = this.history[this.idx];

        if (history > 0) {
            let results = history.map(a => msgToDisplay(a, this.sseq));
            this.headerDiv.innerHTML = results[0][0];
            if (results.length > 1) {
                this.headerDiv.innerHTML += " etc.";
            }
            if (history[0] && history[0]["short-note"]) {
                this.headerDiv.innerHTML += " &mdash; " + renderLaTeX(history[0]["short-note"]);
            }
            this.setSpecialClasses(results.map(x => x[1]).flat());
        }
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

        let action = this.display.history[this.display.idx][0];
        if (!action) {
            return;
        }
        let note = action["note"];
        if (note) {
            this.currentGroup.innerHTML = renderLaTeXP(note);
        } else {
            let shortNote = action["short-note"];
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
        for (let [idx, histList] of this.display.history.entries()) {
            for (let hist of histList) {
                this.addMessage(idx, hist);
            }
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
        if (i != this.display.idx) {
            d.style.opacity=0.6;
        }
        s.innerHTML = title;

        d.addEventListener("dblclick", () => {
            this.display.idx = i;
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
