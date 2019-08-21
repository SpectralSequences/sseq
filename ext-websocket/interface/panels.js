import { STATE_ADD_DIFFERENTIAL, STATE_QUERY_TABLE, STATE_ADD_PRODUCT } from "./display.js";
import { rowToKaTeX, rowToLaTeX, matrixToKaTeX } from "./utils.js";

function addLI(ul, text) {
    let x = document.createElement("li");
    x.innerHTML = text;
    ul.appendChild(x);
}

const ACTION_DISPLAY_NAME = {
    "AddDifferential": "Add Differential",
    "AddPermanentClass": "Add Permanent Class",
    "AddProduct": "Add Product",
    "AddProductDifferential": "AddProductDifferential"
}
export class GeneralPanel extends Panel.TabbedPanel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.overviewTab = new OverviewPanel(this.container, this.display);
        this.addTab("Main", this.overviewTab);

        this.structlineTab = new StructlinePanel(this.container, this.display);
        this.addTab("Prod", this.structlineTab);

        this.historyTab = new HistoryPanel(this.container, this.display);
        this.addTab("Hist", this.historyTab);
    }
}

class HistoryPanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.newGroup();
        this.display.sseq.on("new-history", (data) => this.addMessage(data));
        this.display.sseq.on("clear-history", () => {this.clear(); this.newGroup();});
    }

    _addHistoryItem(title, content, highlightClasses, msg) {
        let d = document.createElement("details");
        let s = document.createElement("summary");
        let t = document.createElement("span");
        t.innerHTML = title;
        s.appendChild(t);

        let rem = document.createElement("a");
        rem.className = "text-danger float-right";
        rem.innerHTML = "&times;";
        rem.href = "#";
        s.appendChild(rem);

        rem.addEventListener("click", () => {
            for (let pair of highlightClasses) {
                let classes = this.display.sseq.getClasses(pair[0], pair[1], this.display.page);
                for (let c of classes) {
                    c.highlight = false;
                }
            }
            this.display.sseq.removeHistoryItem(msg);
        });

        d.appendChild(s);

        let div = document.createElement("div");
        div.className = "text-center py-1";
        div.innerHTML = content;
        d.appendChild(div);

        this.addObject(d);

        d.addEventListener("mouseover", () => {
            d.style = "color: blue";
            for (let pair of highlightClasses) {
                let classes = this.display.sseq.getClasses(pair[0], pair[1], this.display.page);
                for (let c of classes) {
                    c.highlight = true;
                }
            }
            this.display.update();
        });
        d.addEventListener("mouseout", () => {
            d.style = "";
            for (let pair of highlightClasses) {
                let classes = this.display.sseq.getClasses(pair[0], pair[1], this.display.page);
                for (let c of classes) {
                    c.highlight = false;
                }
            }
            this.display.update();
        });

    }

    _AddDifferential(details, msg) {
        this._addHistoryItem(
            `<span>Differential</span> <span class="history-sub">(${details.x}, ${details.y})</span>`,
            Interface.renderMath(`d_{${details.r}}(${rowToLaTeX(details.source)}) = ${rowToLaTeX(details.target)}`),
            [[details.x, details.y], [details.x - 1, details.y + details.r]],
            msg
        );
    }

    _AddProductDifferential(details, msg) {
        let content = `
<ul class="text-left" style="padding-left: 20px; list-style-type: none">
  <li>
    <details>
      <summary>source: ${Interface.renderMath(details.source.name)}</summary>
      (${details.source.x}, ${details.source.y}): ${Interface.renderMath(rowToLaTeX(details.source.class))}
    </details>
  </li>
  <li>
    <details>
      <summary>target: ${Interface.renderMath(details.target.name)}</summary>
      (${details.target.x}, ${details.target.y}): ${Interface.renderMath(rowToLaTeX(details.target.class))}
    </details>
  </li>
</ul>`;
        this._addHistoryItem(
            `<span>Product Differential (${Interface.renderMath(details.source.name + '\\to ' + details.target.name)})</span>`,
            content,
            [[details.source.x, details.source.y], [details.target.x, details.target.y]],
            msg
        );

        let sseq = this.display.sseq;
    }

    _AddProductType(details, msg) {
        this._addHistoryItem(
            `<span>Product (${Interface.renderMath(details.name)})</span>`,
            (details.permanent ? "Permanent" : "Non-permanent") + `: (${details.x}, ${details.y}): ${Interface.renderMath(rowToLaTeX(details.class))}`,
            [[details.x, details.y]]
            ,msg
        );
    }

    _AddPermanentClass(details, msg) {
        this._addHistoryItem(
            `<span>Permanent Class</span> <span class="history-sub">(${details.x}, ${details.y})</span>`,
            Interface.renderMath(`${rowToLaTeX(details.class)}`),
            [[details.x, details.y]]
            ,msg
        );
    }

    addMessage(data) {
        let action = data.action;
        let actionName = Object.keys(action)[0];
        let actionInfo = action[actionName];

        this["_" + actionName](actionInfo, data);
    }

}

class OverviewPanel extends Panel.Panel {
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

class StructlinePanel extends Panel.Panel {
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

        if (!this.display.isUnit) {
            this.addButton("Add", () => window.unitDisplay.openModal(), { "tooltip": "Add product to display" });
        }
    }
}

export class ClassPanel extends Panel.TabbedPanel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
        this.mainTab = new MainPanel(this.container, this.display);
        this.addTab("Main", this.mainTab);

        this.differentialTab = new DifferentialPanel(this.container, this.display);
        this.addTab("Diff", this.differentialTab);

        this.productsTab = new ProductsPanel(this.container, this.display);
        this.addTab("Prod", this.productsTab);

    }
}

class MainPanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.container.className = "text-center";
        this.clear();

        this.newGroup();
        this.addHeader("Classes in E2 basis");
        let x = this.display.selected.x;
        let y = this.display.selected.y;
        let page = this.display.page;
        let sseq = this.display.sseq;

        let classes = sseq.getClasses(x, y, page);
        this.addLine(classes.map(x => rowToKaTeX(x.data)).join("<br />"));

        if (this.display.isUnit) {
            this.newGroup();
            this.addButton("Add Product", () => this.display.sseq.addProductInteractive(this.display.selected.x, this.display.selected.y, this.display.sseq.getClasses(this.display.selected.x, this.display.selected.y, 2).length), { shortcuts : ["m"] });
        }
    }

    addLine(html) {
        let node = document.createElement("div");
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        this.addObject(node);
    }
}

class DifferentialPanel extends Panel.Panel {
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

        // We don't use display.selected because this would refer to the wrong object after we add a differential.
        if (sseq.getClasses(x, y, page)[0].state == "InProgress") {
            this.newGroup();
            this.addHeader("Possible Differentials");

            let maxR = Math.ceil(eval(sseq.vanishingSlope) * x + eval(sseq.vanishingIntercept)) - y;

            let node = document.createElement("div");

            for (let r = 2; r <= maxR; r ++) {
                let classes = sseq.getClasses(x - 1, y + r, r);
                if (classes && classes.length > 0 &&
                    (!sseq.trueDifferentials.get([x, y]) || !sseq.trueDifferentials.get([x, y])[r] || sseq.getClasses(x, y, r).length != sseq.trueDifferentials.get([x, y])[r].length)) {
                    let spn = document.createElement("span");
                    spn.style.padding = "0.75rem";
                    spn.innerHTML = r;

                    // We want to update the classes on *this* page, not on the rth page
                    classes = sseq.getClasses(x - 1, y + r, page);
                    spn.addEventListener("mouseover", () => {
                        spn.style.color = "blue";
                        for (let c of classes) {
                            c.highlight = true;
                        }
                        this.display.update();
                    });
                    spn.addEventListener("mouseout", () => {
                        spn.style.removeProperty("color");
                        for (let c of classes) {
                            c.highlight = false;
                        }
                        this.display.update();
                    });
                    node.appendChild(spn);
                }
            }

            if (!node.hasChildNodes()) {
                node.innerHTML = "No possible differentials!";
            }
            this.addObject(node);
        }

        this.newGroup();
        this.addHeader("Differentials");
        let trueDifferentials = sseq.trueDifferentials.get([x, y]);
        if (trueDifferentials && trueDifferentials.length > page) {
            for (let [source, target] of trueDifferentials[page]) {
                let callback;
                if (this.display.isUnit) {
                    callback = () => {
                        let source_ = sseq.pageBasisToE2Basis(page, x, y, source);
                        let target_ = sseq.pageBasisToE2Basis(page, x, y, target);
                        sseq.addProductDifferentialInteractive(x, y, page, source_, target_);
                    }
                }
                this.addLine(Interface.renderMath(`d_${page}(${rowToLaTeX(source)}) = ${rowToLaTeX(target)}`), callback);
            }
        }
        if (this.display.isUnit) {
            this.addLine("<span style='font-size: 80%'>Click differential to propagate</span>");
        }
        this.addButton("Add", () => this.display.state = STATE_ADD_DIFFERENTIAL, { shortcuts: ["d"]});

        this.newGroup();
        this.addHeader("Permanent Classes");
        let permanentClasses = sseq.permanentClasses.get([x, y]);
        if (permanentClasses.length > 0) {
            this.addLine(permanentClasses.map(rowToKaTeX).join("<br />"));
        }
        this.addButton("Add", () => {
            this.display.sseq.addPermanentClassInteractive(this.display.selected);
        }, { shortcuts: ["p"]});

    }

    addLine(html, callback) {
        let node = document.createElement("div");
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback) {
            node.addEventListener("click", callback);
        }
        this.addObject(node);
    }
}

class ProductsPanel extends Panel.Panel {
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
