import { SidebarDisplay } from "./display.js"
import { ExtSseq } from "./sseq.js"
import { msgToDisplay, Panel, StructlinePanel, TabbedPanel, ClassPanel } from "./panels.js"
import { download, renderLaTeX, renderLaTeXP, inflate, deflate } from "./utils.js"
import { IntroJS } from "./intro.js"

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
        this.idx = this.sseqData.length;

        this.sidebar.footer.newGroup();

        this.prevNext = this.sidebar.footer.addButtonRow([
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
        this.tour();
    }

    tour() {
        const DISPLAY_IDX = 3;
        const SELECT_CLASS = [0, 1];

        let updateState = (selectClass, tab, lastPage) => {
            let idx = lastPage ? this.sseqData.length : DISPLAY_IDX;

            if (this.idx != idx) {
                this.idx = idx;
                this.updateStage();
            }

            if (selectClass) {
                this._onClick(SELECT_CLASS);
                this.classPanel.tabs[tab][1].click();
            } else {
                this._onClick([-10, -10]);
                this.generalPanel.tabs[tab][1].click();
            }
        };
        let intro = new IntroJS();
        intro.setOptions({
            steps: [
                {
                    intro: `<p>This is a step-by-step demonstration of the Adams spectral sequence calculation for ${renderLaTeX(this.sseq.moduleName)}. You are currently viewing the final result of the calculation.</p><p>Use arrow keys to navigate between pages and pan and zoom with the mouse.</p><p>Continue with the walkthrough to learn more about the interface, or click anywhere else to skip it.</p>`,
                    width: 400
                },
                {
                    element: this.generalPanel.tabs[1][1],
                    intro: "The history tab on the sidebar lists the steps performed in the calculation of the spectral sequence.",
                    before: () => updateState(false, 1, true),
                    position: "below"
                },
                {
                    element: this.generalPanel.historyTab.container,
                    intro: "<p>In each step, we either mark a class as permanent or add a differential. 'Permanent product' and 'Propagate ...' are annotations that say we want to propagate differentials along these permanent classes or differentials via the Leibniz rule.</p><p>Sometimes, several actions in this list are grouped together in a single step because they can be justified in a similar way.</p>",
                    before: () => updateState(false, 1, true),
                    position: "left"
                },
                {
                    elementFunction: () => this.generalPanel.historyTab.container.firstChild.children[DISPLAY_IDX - 1],
                    intro: "Double click on a step to display the spectral sequence after performing the step.",
                    before: () => updateState(false, 1, false),
                    position: "left"
                },
                {
                    element: this.prevNext[0].parentElement,
                    intro: "You can use the 'Prev' and 'Next' buttons to navigate between steps as well",
                    before: () => updateState(false, 1, false),
                    position: "left"
                },
                {
                    element: this.headerDiv,
                    intro: "The top bar displays the action and a short explanation if available. If multiple actions were performed, the top bar only displays the first.",
                    before: () => updateState(false, 1, false)
                },
                {
                    element: this.sidebar.mainDiv,
                    intro: "The notes tab displays a longer explanation of the step if available",
                    before: () => updateState(false, 0, false),
                    position: "left"
                },
                {
                    element: this.container_DOM,
                    intro: "The classes are color coded.<ul><li>Orange classes are classes involved in the current action.</li><li>Gray classes are classes all of whose differential have been computed.</li><li>Dark red classes are classes involved in inconsistent differentials (this usually means there is some shorter differential yet to be found).</li></ul> Note that a 'class' here means the entire group in the bidegree. All 'dots' in the same bidegree will always have the same color.",
                    width: 450,
                    before: () => updateState(false, 1, false)
                },
                {
                    element: this.sidebar.mainDiv,
                    intro: "The product tab lists the products that have been computed",
                    before: () => updateState(false, 2, false),
                    position: "below"
                },
                {
                    elementFunction: () => this.generalPanel.structlineTab.container.firstChild.children[1],
                    intro: "You can toggle the display of the structlines for the product and configure the display options",
                    before: () => {
                        updateState(false, 2, false);
                        this.generalPanel.structlineTab.container.firstChild.children[1].firstChild.open = true;
                    },
                    position: "left"
                },
                {
                    element: this.container_DOM,
                    intro: "Click on a class to display information about the class. This 'highlights' the class, and highlighted classes are marked in red",
                    before: () => updateState(true, 0, false)
                },
                {
                    element: this.sidebar.mainDiv,
                    intro: "The various panels display information about the classes, such as their names, product decompositions and differentials.",
                    before: () => updateState(true, 0, false),
                    position: "left"
                },
                {
                    intro: "The code used for computing the spectral sequence and generating the display can be found on <a href='https://github.com/hoodmane/rust_ext'>GitHub<a>."
                }
            ]
        });
        intro.onEnd = () => {
            this.classPanel.tabs[0][1].click();
            this.generalPanel.tabs[0][1].click();
            updateState(false, 0, true);
        }
        intro.start();
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
        if (this.idx == this.sseqData.length)
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
        this.sseq.updateFromBinary(this.sseqData[this.idx == this.sseqData.length ? this.sseqData.length - 1 : this.idx]);
        this.setSpecialClasses([]);

        if (this.idx == 0) {
            this.prevNext[0].disabled = true;
            this.prevNext[1].disabled = false;
            this.headerDiv.innerHTML = "Adams Spectral Sequence for " + renderLaTeX(this.sseq.moduleName);
        } else if (this.idx == this.sseqData.length) {
            this.prevNext[0].disabled = false;
            this.prevNext[1].disabled = true;
            this.headerDiv.innerHTML = "Adams Spectral Sequence for " + renderLaTeX(this.sseq.moduleName);
        } else {
            this.prevNext[0].disabled = false;
            this.prevNext[1].disabled = false;
            let history = this.history[this.idx];

            if (history.length > 0) {
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

        this.historyTab = new CHistoryPanel(this.container, this.display);
        this.addTab("Hist", this.historyTab);

        this.structlineTab = new StructlinePanel(this.container, this.display);
        this.addTab("Prod", this.structlineTab);
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

        if (this.display.idx == this.display.sseqData.length) {
            this.currentGroup.innerHTML = "End of calculation";
            return;
        }

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
