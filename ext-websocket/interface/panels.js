import { STATE_ADD_DIFFERENTIAL, STATE_QUERY_TABLE } from "./display.js";
import { rowToKaTeX, rowToLaTeX, matrixToKaTeX } from "./utils.js";

export class GeneralPanel extends Panel.TabbedPanel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.overviewTab = new OverviewPanel(this.container, this.display);
        this.addTab("Overview", this.overviewTab);

        this.structlineTab = new StructlinePanel(this.container, this.display);
        this.addTab("Structlines", this.structlineTab);
    }
}

export class OverviewPanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
        this.newGroup();

        this.addHeader("Vanishing line");
        this.addLinkedInput("Slope", "sseq.vanishingSlope", "text");
        this.addLinkedInput("Intercept", "sseq.vanishingIntercept", "text");

        this.newGroup();

        this.addButton("Query table", () => this.display.state = STATE_QUERY_TABLE, { shorcuts : ["x"] });
        this.addButton("Resolve further", () => this.display.sseq.resolveFurther());
    }
}

export class StructlinePanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.clear();

        this.newGroup();

        let types = Array.from(this.display.sseq.structlineTypes).sort();
        for (let type of types) {
            let o = document.createElement("div");
            o.className = "form-row mb-2";
            o.style.width = "100%";
            this.currentGroup.appendChild(o);

            let l = document.createElement("label");
            l.className = "col-form-label mr-sm-2";
            l.innerHTML = Interface.renderMath(type);
            o.appendChild(l);

            let s = document.createElement("span");
            s.style.flexGrow = 1;
            o.appendChild(s);

            let i = document.createElement("input");
            i.setAttribute("type", "checkbox");
            i.checked = !this.display.hiddenStructlines.has(type);
            o.appendChild(i);

            i.addEventListener("change", (e) => {
                if (i.checked) {
                    if (this.display.hiddenStructlines.has(type))
                        this.display.hiddenStructlines.delete(type)
                } else {
                    this.display.hiddenStructlines.add(type)
                }
                this.display.update();
            });
        }

        this.addButton("Add", () => { window.unitDisplay.openModal(); }, { "tooltip": "Add product to display" });
    }
}

export class ClassPanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.container.className = "text-center";
        this.clear();

        let x = this.display.selected.x;
        let y = this.display.selected.y;
        let page = this.display.page;
        let sseq = this.display.sseq;

        this.newGroup();
        let classes = sseq.getClasses(x, y, page);

        this.addHeader("Classes");
        this.addLine(classes.map(x => rowToKaTeX(x.data)).join("<br />"));

        this.addHeader("Differentials");
        let trueDifferentials = sseq.trueDifferentials.get([x, y]);
        if (trueDifferentials && trueDifferentials.length > page) {
            for (let [source, target] of trueDifferentials[page]) {
                this.addLine(Interface.renderMath(`d_${page}(${rowToLaTeX(source)}) = ${rowToLaTeX(target)}`));
            }
        }
        this.addButton("Add", () => this.display.state = STATE_ADD_DIFFERENTIAL, { shortcuts: ["d"]});

        this.addHeader("Permanent Classes");
        let permanentClasses = sseq.permanentClasses.get([x, y]);
        if (permanentClasses.length > 0) {
            this.addLine(permanentClasses.map(rowToKaTeX).join("<br />"));
        }
        this.addButton("Add", () => {
            this.display.sseq.addPermanentClassInteractive(this.display.selected);
        }, { shortcuts: ["p"]});

        this.addHeader("Products");
        let products = sseq.getProducts(x, y, page);
        if (products) {
            for (let prod of products) {
                let node = document.createElement("div");
                node.style = "padding: 0.75rem 0";
                node.addEventListener("mouseover", () => {
                    node.style = "padding: 0.75rem 0; color: blue; font-weight: bold";
                    let prodClasses = sseq.getClasses(x + prod.x, y + prod.y, page);
                    if (prodClasses) {
                        for (let c of prodClasses) {
                            c.highlight = true;
                        }
                    }
                    let backClasses = sseq.getClasses(x - prod.x, y - prod.y, page);
                    if (backClasses) {
                        for (let c of backClasses) {
                            c.highlight = true;
                        }
                    }
                    this.display.update();
                });
                node.addEventListener("mouseout", () => {
                    node.style = "padding: 0.75rem 0";
                    let prodClasses = sseq.getClasses(x + prod.x, y + prod.y, page);
                    if (prodClasses) {
                        for (let c of prodClasses) {
                            c.highlight = false;
                        }
                    }
                    let backClasses = sseq.getClasses(x - prod.x, y - prod.y, page);
                    if (backClasses) {
                        for (let c of backClasses) {
                            c.highlight = false;
                        }
                    }
                    this.display.update();
                });

                node.innerHTML = `${Interface.renderMath(prod.name)}: ${matrixToKaTeX(prod.matrix)}`;
                this.addObject(node);
            }
        }
    }

    addLine(html) {
        let node = document.createElement("div");
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        this.addObject(node);
    }

}
