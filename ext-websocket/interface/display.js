import { GeneralPanel, ClassPanel } from "./panels.js";

export const STATE_ADD_DIFFERENTIAL = 1;
export const STATE_QUERY_TABLE = 2;
export const STATE_ADD_PRODUCT = 3;

export class MainDisplay extends SidebarDisplay {
    constructor(container, sseq, isUnit) {
        super(container, sseq);

        this.isUnit = isUnit;
        this.selected = null;
        this.tooltip = new Tooltip(this);
        this.on("mouseover", this._onMouseover.bind(this));
        this.on("mouseout", this._onMouseout.bind(this));
        this.on("click", this.__onClick.bind(this));
        this.on("page-change", this._unselect.bind(this));

        Mousetrap.bind('left',  this.previousPage);
        Mousetrap.bind('right', this.nextPage);

        this.generalPanel = new GeneralPanel(this.sidebar.main_div, this);
        this.sidebar.addPanel(this.generalPanel);
        this.sidebar.currentPanel = this.generalPanel;

        this.classPanel = new ClassPanel(this.sidebar.main_div, this);
        this.sidebar.addPanel(this.classPanel);

        this.sidebar.footer.newGroup();

        this.sidebar.footer.currentGroup.style.textAlign = "center";
        this.runningSign = document.createElement("p");
        this.runningSign.className = "card-text"
        this.runningSign.innerHTML = "Running...";
        this.sidebar.footer.addObject(this.runningSign);

        this.sidebar.footer.addButtonRow([
            ["Undo", () => this.sseq.undo()],
            ["Redo", () => this.sseq.redo()]
        ]);

        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG("sseq.svg"));
        this.sidebar.footer.addButton("Save", () => window.save());

        sseq.on("update", (x, y) => { if (this.selected && this.selected.x == x && this.selected.y == y) this.sidebar.showPanel() });
    }

    _onMouseover(node) {
        this.tooltip.setHTML(`(${node.x}, ${node.y})`);
        this.tooltip.show(node.canvas_x, node.canvas_y);
    }

    __onClick(node, e) {
        if (this.state == STATE_QUERY_TABLE) {
            let x = Math.round(this.xScale.invert(e.clientX));
            let y = Math.round(this.yScale.invert(e.clientY));
            this.sseq.queryTable(x, y);
            this.state = null;
            return;
        }

        if (!node) {
            this._unselect();
            return;
        }

        switch (this.state) {
            case STATE_ADD_DIFFERENTIAL:
                if (this.selected) {
                    this.sseq.addDifferentialInteractive(this.selected, node);
                    this.state = null;
                    break;
                }
            case STATE_ADD_PRODUCT:
                this.sseq.addProductInteractive(node.x, node.y, node.idx);
            default:
                this._unselect();
                this.selected = node;
                let x = node.x;
                let y = node.y;

                for (let c of this.sseq.getClasses(x, y, this.page)) {
                    c.highlight = true;
                }
        }

        this.update();

        this.sidebar.showPanel(this.classPanel);
    }

    _onMouseout() {
        if (this.selected) this.selected.highlight = true;
        this.tooltip.hide();
    }

    _unselect() {
        if (this.selected === null) return;

        let x = this.selected.x;
        let y = this.selected.y;

        for (let c of this.sseq.getClasses(x, y, this.page)) {
            c.highlight = false;
        }

        this.selected = null;
        this.state = null;

        this.sidebar.showPanel(this.generalPanel);

        this._drawSseq(this.context);
    }

    setSseq(sseq) {
        super.setSseq(sseq);

        sseq.on("new-structline", () => this.sidebar.showPanel());
    }
}

export class UnitDisplay extends Display {
    constructor(container, sseq) {
        super(container, sseq);

        this.tooltip = new Tooltip(this);
        this.on("mouseover", (node) => {
            this.tooltip.setHTML(`(${node.x}, ${node.y})`);
            this.tooltip.show(node.canvas_x, node.canvas_y);
        });

        this.on("mouseout", () => {
            if (this.selected) this.selected.highlight = true;
            this.tooltip.hide();
        });

        document.querySelectorAll(".close-modal").forEach((c) => {
            c.addEventListener("click", this.closeModal.bind(this));
        });

        document.querySelector("#modal-diff").addEventListener("click", () => {
            document.querySelector("#modal-title").innerHTML = "Select target element";
            this.state = STATE_ADD_DIFFERENTIAL;
        });

        document.querySelector("#modal-ok").addEventListener("click", () => {
            window.mainSseq.addProductInteractive(this.selected.x, this.selected.y, this.selected.idx);
            this.closeModal();
        });

        document.querySelector("#modal-more").addEventListener("click", () => this.sseq.resolveFurther());

        this.on("click", this.__onClick.bind(this));
    }

    openModal() {
        this._unselect();
        this.sseq.resolveFurther(10);
        document.querySelector("#overlay").style.removeProperty("display");
        document.querySelector("#modal-ok").disabled = true;
        document.querySelector("#modal-diff").disabled = true;
        let dialog = document.querySelector("#modal-dialog");
        dialog.classList.add("modal-shown");
    }

    closeModal() {
        document.querySelector("#overlay").style.display = "none";
        let dialog = document.querySelector("#modal-dialog");
        dialog.classList.remove("modal-shown");
        this._unselect();
    }

    __onClick(node, e) {
        if (!node) {
            this._unselect();
            return;
        }

        if (this.state == STATE_ADD_DIFFERENTIAL) {
            if (node.x == this.selected.x - 1 && node.y >= this.selected.y + 2) {
                let check = confirm(`Add differential from (${this.selected.x}, ${this.selected.y}, ${this.selected.idx}) to (${node.x}, ${node.y}, ${node.idx})?`);
                if (check) {
                    window.mainSseq.addProductDifferentialInteractive(this.selected, node);
                    this.state = null;
                    this.closeModal();
                }
            } else {
                alert("Invalid target for differential");
            }
        }

        this._unselect();
        this.selected = node;
        document.querySelector("#modal-ok").disabled = false;
        document.querySelector("#modal-diff").disabled = false;
    }

    _unselect() {
        if (!this.selected) return;
        this.selected.highlight = false;
        this.selected = null;
        this.update();
        document.querySelector("#modal-title").innerHTML = "Select element to multiply with";
        document.querySelector("#modal-ok").disabled = true;
    }
}
