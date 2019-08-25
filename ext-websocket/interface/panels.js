import { STATE_ADD_DIFFERENTIAL, STATE_QUERY_TABLE, STATE_ADD_PRODUCT } from "./display.js";
import { rowToKaTeX, rowToLaTeX, matrixToKaTeX, vecToName } from "./utils.js";
import { MIN_PAGE } from "./sseq.js";

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

/**
 * A panel is a collection of objects (button etc.) to be displayed in a
 * sidepanel. The main function to implement is show(), which is called
 * whenever the panel is to be displayed.
 *
 * The standard way to deal with panels is that all children of the panel are
 * created when the panel is initialized, and all callbacks are appropriately
 * set up. When show() is called, we decide which elements to display by
 * setting the display property, and then initialize the values of the elements
 * accordingly.
 *
 * Panels and its children are expected to properly track mutations and write
 * them to this.display.sseq.undo upon each change.
 *
 * There are a few helper functions that add elements to the panel, such as
 * addButton.
 *
 * @property {Node} container - Top node of the panel, to which we add all
 * children. This is a plain div element that is not styled. All styling should
 * be applied to children of this container.
 * @property {Node} currentGroup - This is the DOM element that the helper
 * functions will add the buttons/fields to. This defaults to this.container
 * but is modified by newGroup() and endGroup(). It can also be manually
 * modified as desired.
 *
 * @fires Panel#show
 * @extends EventEmitter
 */
export class Panel extends EventEmitter {
    /**
     * Constructs a panel.
     *
     * @param {Node} parentContainer - The node to add the panel to
     * @param {Display:Display} - The Display object the panel is about.
     * This is used by the helper functions to know where to track mutations,
     * update the display when properties change, etc.
     */
    constructor (parentContainer, display) {
        super();

        this.display = display;
        this.container = document.createElement("div");
        parentContainer.appendChild(this.container);
        this.links = [];

        this.currentGroup = this.container;
    }

    /**
     * This hides the panel. It does nothing but set the display property to
     * none.
     */
    hide() {
        this.container.style.display = "none";
    }

    /**
     * This clears everything in the panel. This currently does not unbind the
     * shortcuts.
     */
    clear() {
        while (this.container.firstChild)
            this.container.removeChild(this.container.firstChild);

        this.links = [];
    }

    /**
     * This shows the panel, and populates the values of the children.  This
     * correctly populates the children added by the helper functions, and no
     * extra work has to be done for them. If custom children are added, one
     * will want to customize the show() function to ensure the children are
     * correctly displayed. This can be done by overwriting the show() function
     * or by listening to the Panel#show event.
     *
     * This function may be called when the panel is already shown. In this
     * case, the correct behaviour is to refresh the display (e.g. update the
     * values of the fields)
     */
    show() {
        this.container.style.removeProperty("display");

        for (let link of this.links) {
            let t = this.display;
            for (let attr of link[0].split(".")) {
                t = t[attr];
                if (t === undefined || t === null) {
                    return;
                }
            }
            link[1].value = t;
        }
        /**
         * Show event. This is emitted when show() is called. One may opt to
         * listen and respond to the show event instead of overwriting show()
         * when designing custom panels, c.f. DifferentialPanel.
         *
         * @event Panel#show
         */
        this.emit("show");
    }

    /**
     * This creates a new div and adds it to the container. This new div is
     * then set as currentGroup and has class card-body.
     *
     * This should be used if one wishes to add a collection of children that
     * are to be grouped together. The procedure for using this is as follows:
     * (1) Run Panel#addGroup
     * (2) Add the children using the helper functions (addButton, addObject, etc.)
     * (3) Run Panel#endGroup to set currentGroup back to this.container.
     */
    newGroup() {
        this.currentGroup = document.createElement("div");
        this.currentGroup.className = "card-body";
        this.container.appendChild(this.currentGroup);
    }
    /**
     * See newGroup().
     */
    endGroup() {
        this.currentGroup = this.container;
    }

    /**
     * Does nothing but this.currentGroup.appendChild(obj);
     *
     * @param {Node} obj - The object to be added.
     */
    addObject(obj) {
        this.currentGroup.appendChild(obj);
    }

    /**
     * This adds a button to currentGroup.
     *
     * @param {string} text - Text to appear on the button.
     * @param {function} callback - Function to call when button is clicked.
     * @param {Object} extra - Extra (optional) properties to supply.
     * @param {string} extra.tooltip - Tooltip text to display
     * @param {string[]} shortcuts - A list of shortcuts that will be bound to callback
     */
    addButton(text, callback, extra = {}) {
        let o = document.createElement("button");
        if (extra.style)
            o.className = `btn btn-${extra.style} mb-2`;
        else
            o.className = "btn btn-primary mb-2";

        o.style.width = "100%";
        o.innerHTML = text;
        o.addEventListener("click", callback);

        if (extra.tooltip)
            o.setAttribute("title", extra.tooltip);
        if (extra.shortcuts)
            for (let k of extra.shortcuts)
                Mousetrap.bind(k, callback);

        this.currentGroup.appendChild(o);
    }

    /**
     * This adds several buttons placed side-by-side on a row.
     *
     * @param {Array[]} buttons - An array of arguments specifying the buttons
     * to be added. Each entry in the array should itself be an array, which
     * consists of the arguments to Panel#addButton for the corresponding
     * button.
     */
    addButtonRow(buttons){
        let group = this.currentGroup;
        let o = document.createElement("div");
        o.className = "form-row";
        for (let button of buttons) {
            let c = document.createElement("div");
            c.className = "col";
            this.currentGroup = c;
            this.addButton(...button);
            o.appendChild(c);
        }
        this.currentGroup = group;
        this.currentGroup.appendChild(o);
    }

    /**
     * This adds a header.
     * @param {String} header - The header text.
     */
    addHeader(header) {
        let node = document.createElement("h5");
        node.className = "card-title";
        node.innerHTML = header;
        this.addObject(node);
    }

    /**
     * This adds a linked input. A linked input is an entry that looks like
     *
     *       +-----+
     * Label |     |
     *       +-----+
     *
     * The input field is linked to a certain property of display. When the
     * panel is shown, the initial value of the input field is set to the value
     * of the corresponding property, and when the input field is changed, the
     * property is changed accordingly.
     *
     * @param {string} label - The label displayed next to the input field
     * @param {string} target - The property the input field is linked to.
     * This is specified by a string of the from "foo.bar.xyz", which says the
     * field is linked to this.display.foo.bar.xyz.
     * @param {string} type - The type of the input field. This is "text" or
     * "number" would usually be sensible choices.
     * @param {Object=} mementoObject - By default, the undo/redo functions
     * will simply set the value of target to what it was. Here the target
     * is remembered as an *object*, not as a property of this.display via
     * target (for example, if the input is about the currently active node
     * (this.display.selected), the undo function should undo the change on the
     * node that was affected, not the node that is active when the undo button
     * is pressed). It turns out this is problematic when dealing with nodes of
     * classes, since when classes are restored via undo/redo, the set of nodes
     * is copied and all references are lost.
     *
     * If mementoObject is defined, then instead of tracking individual changes
     * of the properties, the mutation tracker remembers the previous and after
     * states of mementoObject and writes that into the undo stack instead.
     * c.f. the node color/size inputs in EditorDisplay.
     */
    addLinkedInput(label, target, type, mementoObject) {
        let o = document.createElement("div");
        o.className = "form-row mb-2";
        o.style.width = "100%";
        this.currentGroup.appendChild(o);

        let l = document.createElement("label");
        l.className = "col-form-label mr-sm-2";
        l.innerHTML = label;
        o.appendChild(l);

        let i = document.createElement("input");
        i.style["flex-grow"] = 1;
        i.setAttribute("type", type);
        o.appendChild(i);

        switch (type) {
            case "text":
                i.setAttribute("size", "1");
                break;
            default:
                i.style.width = "1px";
                break;
        }

        this.links.push([target, i]);

        i.addEventListener("change", (e) => {
            let target_pre;
            if (mementoObject) {
                mementoObject = Panel.unwrapProperty(this.display, mementoObject.split("."))
                target_pre = mementoObject.getMemento();
            }

            let l = target.split(".");
            let prop = l.pop();
            let t = Panel.unwrapProperty(this.display, l);

            let old_val = t[prop];
            let new_val = e.target.value;
            t[prop] = new_val;

            if (this.display.sseq.undo) {
                if (mementoObject) {
                    this.display.sseq.undo.startMutationTracking()
                    this.display.sseq.undo.addMutation(mementoObject, target_pre, mementoObject.getMemento())
                    this.display.sseq.undo.addMutationsToUndoStack();
                } else {
                    this.display.sseq.undo.addValueChange(t, prop, old_val, new_val, () => this.display.sidebar.showPanel());
                }
            }

            this.display.sseq.emit("update");
        });
    }

    addLine(html, callback, highlights) {
        let node = document.createElement("div");
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback) {
            node.addEventListener("click", callback);
        }
        if (highlights) {
            node.addEventListener("mouseover", () => {
                node.style.color = "blue";
                for (let highlight of highlights) {
                    this.display.highlightClass([highlight[0], highlight[1]]);
                }
                this.display.update();
            });
            node.addEventListener("mouseout", () => {
                node.style.removeProperty("color");
                this.display.clearHighlight();
                this.display.update();
            });
        }
        this.addObject(node);
    }

    static unwrapProperty(start, list) {
        let t = start;
        for (let i of list)
            t = t[i];
        return t;
    }
}

/**
 * This is a panel whose some purpose is to contain further panels arranged in
 * tabs. This is used, for example, in EditorDisplay for configuring different
 * properties of a class.
 *
 * @property {Panel} currentTab - The current tab that is displayed.
 */
class TabbedPanel extends Panel {
    constructor (parentContainer, display) {
        super(parentContainer, display);

        let head = document.createElement("div");
        head.className = "card-header";
        this.container.appendChild(head);

        this.header = document.createElement("ul");
        this.header.className = "nav nav-tabs card-header-tabs";
        head.appendChild(this.header);

        this.tabs = [];
        this.currentTab = null;
    }

    /**
     * This adds a tab to TabbedPanel.
     *
     * @param {string} name - The name of the tab, to be displayed in the
     * header. Avoid making this too long.
     * @param {Panel} tab - The tab to be added.
     */
    addTab(name, tab) {
        let li = document.createElement("li");
        li.className = "nav-item";
        this.header.appendChild(li);

        let a = document.createElement("a");
        a.className = "nav-link";
        a.href = "#";
        a.innerHTML = name;
        li.appendChild(a);

        a.addEventListener("click", () => this.showTab(tab));
        this.tabs[this.tabs.length] = [tab, a];

        if (!this.currentTab) this.currentTab = tab;
    }

    show() {
        super.show();
        this.showTab(this.currentTab);
    }

    nextTab() {
        let n = this.tabs.findIndex(t => t[0] == this.currentTab);
        n = (n + 1) % this.tabs.length;
        this.showTab(this.tabs[n][0]);
    }

    prevTab() {
        let n = this.tabs.findIndex(t => t[0] == this.currentTab);
        n = (n + this.tabs.length - 1) % this.tabs.length;
        this.showTab(this.tabs[n][0]);
    }

    /**
     * Sets the corresponding tab to be the active tab and shows it (of course,
     * the tab will not be actually shown if the panel itself is hidden).
     *
     * @param {Panel} tab - Tab to be shown.
     */
    showTab(tab) {
        this.currentTab = tab;
        for (let t of this.tabs) {
            if (t[0] == tab) {
                t[1].className = "nav-link active";
                t[0].show();
            } else {
                t[1].className = "nav-link";
                t[0].hide();
            }
        }
    }
}

export class GeneralPanel extends TabbedPanel {
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

class HistoryPanel extends Panel {
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
            this.display.clearHighlight();
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
                this.display.highlightClass([pair[0], pair[1]]);
            }
            this.display.update();
        });
        d.addEventListener("mouseout", () => {
            d.style = "";
            this.display.clearHighlight();
            this.display.update();
        });

    }

    _AddDifferential(details, msg) {
        this._addHistoryItem(
            `<span>Differential</span> <span class="history-sub">(${details.x}, ${details.y})</span>`,
            katex.renderToString(`d_{${details.r}}(${rowToLaTeX(details.source)}) = ${rowToLaTeX(details.target)}`),
            [[details.x, details.y], [details.x - 1, details.y + details.r]],
            msg
        );
    }

    _AddProductDifferential(details, msg) {
        let content = `
<ul class="text-left" style="padding-left: 20px; list-style-type: none">
  <li>
    <details>
      <summary>source: ${katex.renderToString(details.source.name)}</summary>
      (${details.source.x}, ${details.source.y}): ${katex.renderToString(rowToLaTeX(details.source.class))}
    </details>
  </li>
  <li>
    <details>
      <summary>target: ${katex.renderToString(details.target.name)}</summary>
      (${details.target.x}, ${details.target.y}): ${katex.renderToString(rowToLaTeX(details.target.class))}
    </details>
  </li>
</ul>`;
        this._addHistoryItem(
            `<span>Product Differential (${katex.renderToString(details.source.name + '\\to ' + details.target.name)})</span>`,
            content,
            [[details.source.x, details.source.y], [details.target.x, details.target.y]],
            msg
        );

        let sseq = this.display.sseq;
    }

    _AddProductType(details, msg) {
        this._addHistoryItem(
            `<span>Product (${katex.renderToString(details.name)})</span>`,
            (details.permanent ? "Permanent" : "Non-permanent") + `: (${details.x}, ${details.y}): ${katex.renderToString(rowToLaTeX(details.class))}`,
            [[details.x, details.y]]
            ,msg
        );
    }

    _AddPermanentClass(details, msg) {
        this._addHistoryItem(
            `<span>Permanent Class</span> <span class="history-sub">(${details.x}, ${details.y})</span>`,
            katex.renderToString(`${rowToLaTeX(details.class)}`),
            [[details.x, details.y]]
            ,msg
        );
    }

    _SetClassName(details, msg) {
        this._addHistoryItem(
            `<span>Set Name</span> <span class="history-sub">(${details.x}, ${details.y})</span>`,
            katex.renderToString(`${details.idx} \\rightsquigarrow ${details.name}`),
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

class OverviewPanel extends Panel {
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

class StructlinePanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.clear();

        this.newGroup();

        let names = Array.from(this.display.sseq.products.keys()).sort();
        for (let name of names) {
            let o = document.createElement("div");
            o.className = "form-row mb-2";
            o.style.width = "100%";
            this.currentGroup.appendChild(o);

            let l = document.createElement("label");
            l.className = "col-form-label mr-sm-2";
            l.innerHTML = katex.renderToString(name);
            o.appendChild(l);

            let s = document.createElement("span");
            s.style.flexGrow = 1;
            o.appendChild(s);

            let i = document.createElement("input");
            i.setAttribute("type", "checkbox");
            i.checked = this.display.visibleStructlines.has(name);
            o.appendChild(i);

            i.addEventListener("change", (e) => {
                if (i.checked) {
                    this.display.visibleStructlines.add(name)
                } else {
                    if (this.display.visibleStructlines.has(name))
                        this.display.visibleStructlines.delete(name)
                }
                this.display.update();
            });
        }

        if (!this.display.isUnit) {
            this.addButton("Add", () => window.unitDisplay.openModal(), { "tooltip": "Add product to display" });
        }
    }
}

export class ClassPanel extends TabbedPanel {
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

class MainPanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.container.className = "text-center";
        this.clear();

        this.newGroup();
        this.addHeader("Classes");
        let [x, y] = this.display.selected;
        let page = this.display.page;
        let sseq = this.display.sseq;

        let classes = sseq.getClasses(x, y, page);
        let names = sseq.classNames.get([x, y]);

        let div = document.createElement("div");
        for (let c of classes) {
            let n = document.createElement("span");
            n.style.padding = "0 0.6em";
            n.innerHTML = katex.renderToString(vecToName(c, names));
            if (classes.length == sseq.classes.get([x, y])[0].length) {
                n.addEventListener("click", () => {
                    let name = prompt("New class name");
                    if (name !== null) {
                        sseq.setClassName(x, y, c.indexOf(1), name);
                    }
                });
            }
            div.appendChild(n);
        }
        this.addObject(div);

        let decompositions = sseq.decompositions.get([x, y]);
        if (decompositions && decompositions.length > 0) {
            this.newGroup();
            this.addHeader("Decompositions");
            for (let d of decompositions) {
                let single = d[0].reduce((a, b) => a + b, 0) == 1;
                let highlights = [[x - d[2], y - d[3]]];
                if (this.display.isUnit) {
                    highlights[1] = [d[2], d[3]]
                }
                if (single) {
                    let idx = d[0].indexOf(1);
                    // If we named the element after the decomposition, there is no point in displaying it...
                    if (katex.renderToString(names[idx]) != katex.renderToString(d[1])) {
                        this.addLine(katex.renderToString(names[idx] + " = " + d[1]), () => {
                            if (confirm(`Rename ${names[idx]} as ${d[1]}?`)) {
                                sseq.setClassName(x, y, idx, d[1]);
                                this.display.clearHighlight();
                            }
                        }, highlights);
                    }
                } else {
                    this.addLine(katex.renderToString(vecToName(d[0], names) + " = " + d[1]), undefined, highlights);
                }
            }
        }

        if (this.display.isUnit) {
            this.newGroup();
            this.addButton("Add Product", () => {
                let [x, y] = this.display.selected;
                let num = this.display.sseq.getClasses(x, y, MIN_PAGE).length;
                this.display.sseq.addProductInteractive(x, y, num);
            }, { shortcuts : ["m"] });
        }
    }
}

class DifferentialPanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.container.className = "text-center";
        this.clear();

        let [x, y] = this.display.selected;
        let page = this.display.page;
        let sseq = this.display.sseq;

        // We don't use display.selected because this would refer to the wrong object after we add a differential.
        if (sseq.classState.get([x, y]) == "InProgress") {
            this.newGroup();
            this.addHeader("Possible Differentials");

            let maxR = Math.ceil(eval(sseq.vanishingSlope) * x + eval(sseq.vanishingIntercept)) - y;

            let node = document.createElement("div");

            for (let r = MIN_PAGE; r <= maxR; r ++) {
                let classes = sseq.getClasses(x - 1, y + r, r);
                if (classes && classes.length > 0 &&
                    (!sseq.trueDifferentials.get([x, y]) || !sseq.trueDifferentials.get([x, y])[r - MIN_PAGE] || sseq.getClasses(x, y, r).length != sseq.trueDifferentials.get([x, y])[r - MIN_PAGE].length)) {
                    let spn = document.createElement("span");
                    spn.style.padding = "0.75rem";
                    spn.innerHTML = r;

                    // We want to update the classes on *this* page, not on the rth page
                    classes = sseq.getClasses(x - 1, y + r, page);
                    spn.addEventListener("mouseover", () => {
                        spn.style.color = "blue";
                        this.display.highlightClass([x - 1, y + r]);
                        this.display.update();
                    });
                    spn.addEventListener("mouseout", () => {
                        spn.style.removeProperty("color");
                        this.display.clearHighlight();
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
        if (trueDifferentials && trueDifferentials.length > page - MIN_PAGE) {
            for (let [source, target] of trueDifferentials[page - MIN_PAGE]) {
                let callback;
                if (this.display.isUnit) {
                    callback = () => {
                        let source_ = sseq.pageBasisToE2Basis(page, x, y, source);
                        let target_ = sseq.pageBasisToE2Basis(page, x, y, target);
                        sseq.addProductDifferentialInteractive(x, y, page, source_, target_);
                    }
                }
                this.addLine(katex.renderToString(`d_${page}(${rowToLaTeX(source)}) = ${rowToLaTeX(target)}`), callback);
            }
        }
        if (this.display.isUnit) {
            this.addLine("<span style='font-size: 80%'>Click differential to propagate</span>");
        }
        this.addButton("Add", () => this.display.state = STATE_ADD_DIFFERENTIAL);

        this.newGroup();
        this.addHeader("Permanent Classes");
        let permanentClasses = sseq.permanentClasses.get([x, y]);
        if (permanentClasses.length > 0) {
            this.addLine(permanentClasses.map(rowToKaTeX).join("<br />"));
        }
        this.addButton("Add", () => {
            sseq.addPermanentClassInteractive(x, y);
        });

    }
}

class ProductsPanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.container.className = "text-center";
        this.clear();

        let [x, y] = this.display.selected;
        let page = this.display.page;
        let sseq = this.display.sseq;

        for (let [name, mult] of sseq.products) {
            let matrices = mult.matrices.get([x, y]);
            if (matrices === undefined)
                continue;

            let page_idx = Math.min(matrices.length - 1, page - MIN_PAGE);
            let matrix = matrices[page_idx];

            let node = document.createElement("div");
            node.style = "padding: 0.75rem 0";
            node.addEventListener("mouseover", () => {
                node.style = "padding: 0.75rem 0; color: blue; font-weight: bold";
                this.display.highlightClass([x + mult.x, y + mult.y]);
                this.display.highlightClass([x - mult.x, y - mult.y]);
                this.display.update();
            });
            node.addEventListener("mouseout", () => {
                node.style = "padding: 0.75rem 0";
                this.display.clearHighlight();
                this.display.update();
            });

            node.innerHTML = `${katex.renderToString(name)}: ${matrixToKaTeX(matrix)}`;
            this.addObject(node);
        }
    }

    addLine(html) {
        let node = document.createElement("div");
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        this.addObject(node);
    }
}

