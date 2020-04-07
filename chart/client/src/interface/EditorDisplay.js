"use strict"

let SidebarDisplay = require("./SidebarDisplay.js").SidebarDisplay;
let Panel = require("./Panel.js");
let Tooltip = require("./Tooltip.js").Tooltip;
let Interface = require("./Interface.js");
let Mousetrap = require("mousetrap");

const STATE_ADD_DIFFERENTIAL = 1;
const STATE_RM_DIFFERENTIAL = 2;
const STATE_ADD_STRUCTLINE = 3;
const STATE_RM_STRUCTLINE = 4;
const STATE_RM_EDGE = 5;
const STATE_ADD_CLASS = 6;

class EditorDisplay extends SidebarDisplay {
    constructor(container, sseq) {
        super(container);

        this.differentialColors = {};

        // Footer
        this.sidebar.footer.newGroup();
        this.sidebar.footer.addButtonRow([
            ["Undo", () => this.sseq.undo.undo()],
            ["Redo", () => this.sseq.undo.redo()]
        ]);

        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG("sseq.svg"));
        this.sidebar.footer.addButton("Save", () => this.sseq.download("sseq.json"));

        // General Panel
        this.generalPanel = new Panel.Panel(this.sidebar.main_div, this);
        this.generalPanel.newGroup();
        this.pageLabel = document.createElement("span");
        this.on("page-change", (r) => {
            this.pageLabel.innerHTML = this.getPageDescriptor(r);
            this._unselect();
        });
        this.generalPanel.addObject(this.pageLabel);

        this.generalPanel.newGroup();
        this.generalPanel.addButton("Add class", () => this.state = STATE_ADD_CLASS, { shortcuts: ["n"] });

        this.generalPanel.newGroup();
        this.generalPanel.addLinkedInput("Min X", "sseq.minX", "number");
        this.generalPanel.addLinkedInput("Max X", "sseq.maxX", "number");
        this.generalPanel.addLinkedInput("Min Y", "sseq.minY", "number");
        this.generalPanel.addLinkedInput("Max Y", "sseq.maxY", "number");
        this.sidebar.addPanel(this.generalPanel);

        // Class panel
        this.classPanel = new Panel.TabbedPanel(this.sidebar.main_div, this);
        this.sidebar.addPanel(this.classPanel);

        // Node tab
        this.nodeTab = new Panel.Panel(this.classPanel.container, this);
        this.nodeTab.newGroup();

        this.title_text = document.createElement("span");
        this.nodeTab.addObject(this.title_text);

        this.title_edit_link = document.createElement("a");
        this.title_edit_link.className = "card-link-body";
        this.title_edit_link.href = "#";
        this.title_edit_link.style.float = "right";
        this.title_edit_link.innerHTML = "Edit";
        this.title_edit_link.addEventListener("click", () => {
            let c = this.selected.c;
            if (this.title_edit_link.innerHTML == "OK") {
                let old_name = c.name;
                c.name = this.title_edit_input.value;
                this.sseq.undo.addValueChange(c, "name", old_name, c.name, () => this.sidebar.showPanel());
                this.sseq.emit("update");
                this.nodeTab.show();
            } else {
                this.title_edit_link.innerHTML = "OK";
                if (c.name) this.title_edit_input.value = c.name;
                this.title_edit_input.style.removeProperty("display");
            }
        });
        this.nodeTab.addObject(this.title_edit_link);

        this.title_edit_input = document.createElement("input");
        this.title_edit_input.className = "form-control mt-2";
        this.title_edit_input.type = "text";
        this.title_edit_input.placeholder = "Enter class name";
        this.nodeTab.addObject(this.title_edit_input);

        this.nodeTab.on("show", () => {
            this.title_edit_input.style.display = "none";
            this.title_edit_input.value = "";
            this.title_edit_link.innerHTML = "Edit";
            let c = this.selected.c;
            if (c.name) {
                this.title_text.innerHTML = Interface.renderLaTeX(Interface.ensureMath(c.name)) + ` - (${c.x}, ${c.y})`;
            } else {
                this.title_text.innerHTML = `<span style='color: gray'>unnamed</span> - (${c.x}, ${c.y})`;
            }
        });

        this.nodeTab.newGroup();
        this.nodeTab.addLinkedInput("Color", "selected.color", "text", "selected.c");
        this.nodeTab.addLinkedInput("Size", "selected.size", "number", "selected.c");
        this.nodeTab.addButton("Delete class", () => {
            this.sseq.startMutationTracking();
            this.sseq.deleteClass(this.selected.c);
            this.sseq.addMutationsToUndoStack();
            this.sidebar.showPanel(this.generalPanel)
        }, { style: "danger" });
        this.classPanel.addTab("Node", this.nodeTab);

        // Differentials tab
        this.differentialTab = new Panel.DifferentialPanel(this.classPanel.container, this);
        Mousetrap.bind('d', () => this.state = STATE_ADD_DIFFERENTIAL);
        Mousetrap.bind('r', () => this.state = STATE_RM_EDGE);
        this.classPanel.addTab("Diff", this.differentialTab);

        // Structline tab
        this.structlineTab = new Panel.StructlinePanel(this.classPanel.container, this);
        Mousetrap.bind('s', () => this.state = STATE_ADD_STRUCTLINE);
        this.classPanel.addTab("Struct", this.structlineTab);

        this.sidebar.showPanel(this.generalPanel);

        this.tooltip = new Tooltip(this);
        this.on("mouseover", (node) => {
            this.tooltip.setHTML(`(${node.c.x}, ${node.c.y})`);
            this.tooltip.show(node.canvas_x, node.canvas_y);
        });
        this.on("mouseout", this._onMouseout.bind(this));
        this.on("click", this.__onClick.bind(this)); // Display already has an _onClick

        this._onDifferentialAdded = this._onDifferentialAdded.bind(this);

        Mousetrap.bind('left',  this.previousPage);
        Mousetrap.bind('right', this.nextPage);
        Mousetrap.bind('x', () => { if(this.selected){ console.log(this.selected.c); } });

        if (sseq) this.setSseq(sseq);
    }

    setDifferentialColor(page, color) {
        this.differentialColors[page] = color;
    }

    setSseq(sseq) {
        if (this.sseq)
            this.sseq.removeListener("differential-added", this._onDifferentialAdded);

        super.setSseq(sseq)

        this.sidebar.showPanel(this.generalPanel);

        this.sseq.on("differential-added", this._onDifferentialAdded);
    }

    _onMouseout() {
        if (this.selected) this.selected.highlight = true;
        this.tooltip.hide();
    }

    _unselect() {
        if (!this.selected) return;

        this.selected.highlight = false;
        this.selected = null;
        this.state = null;

        this.sidebar.showPanel(this.generalPanel);

        this._drawSseq(this.context);
    }

    __onClick(node, e) {
        if (this.state == STATE_ADD_CLASS) {
            let x = Math.round(this.xScale.invert(e.clientX));
            let y = Math.round(this.yScale.invert(e.clientY));
            this.sseq.undo.startMutationTracking();
            this.sseq.addClass(x, y);
            this.sseq.undo.addMutationsToUndoStack();
            this.state = null;
            return;
        }

        if (!node) {
            this._unselect();
            return;
        }

        if (!this.selected) {
            this._unselect();
            this.selected = node;
            this.sidebar.showPanel(this.classPanel);
            this.state = null;
            return;
        }

        let s = this.selected.c;
        let t = node.c;
        switch (this.state) {
            case STATE_ADD_DIFFERENTIAL:
                if(s.x !== t.x + 1){
                    this._unselect();
                    break;
                }
                let length = t.y - s.y;
                this.sseq.undo.startMutationTracking();
                this.sseq.addDifferential(s, t, length);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_DIFFERENTIAL:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.type === "Differential" && e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_ADD_STRUCTLINE:
                this.sseq.undo.startMutationTracking();
                this.sseq.addStructline(s, t);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_STRUCTLINE:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.type === "Structline" && e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_EDGE:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            default:
                this._unselect();
                this.selected = node;
                this.sidebar.showPanel(this.classPanel);
                break;
        }
        this.state = null;
    }

    _onDifferentialAdded(d) {
        if (this.differentialColors[d.page])
            d.color = this.differentialColors[d.page];
    }
}
exports.EditorDisplay = EditorDisplay;
