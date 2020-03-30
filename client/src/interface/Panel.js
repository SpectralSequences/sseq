"use strict"

let EventEmitter = require("events");
let Mousetrap = require("mousetrap");
let Interface = require("./Interface.js");

const STATE_ADD_DIFFERENTIAL = 1;
const STATE_RM_DIFFERENTIAL = 2;
const STATE_ADD_STRUCTLINE = 3;
const STATE_RM_STRUCTLINE = 4;
const STATE_RM_EDGE = 5;

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
class Panel extends EventEmitter {
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

class DifferentialPanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.differential_list = document.createElement("ul");
        this.differential_list.className = "list-group list-group-flush";
        this.differential_list.style["text-align"] = "center";
        this.addObject(this.differential_list);

        this.on("show", () => {
            while(this.differential_list.firstChild)
                this.differential_list.removeChild(this.differential_list.firstChild);

            let edges = this.display.selected.c.edges.filter(e => e.type === "Differential").sort((a, b) => a.page - b.page);

            let sname, tname;
            for (let e of edges) {
                sname = e.source.name ? e.source.name : "?"
                tname = e.target.name ? e.target.name : "?"
                if (e.source == this.display.selected.c)
                    this.addLI(Interface.renderMath(`d_${e.page}({\\color{blue}${sname}}) = ${tname}`));
                else
                    this.addLI(Interface.renderMath(`d_${e.page}(${sname}) = {\\color{blue}${tname}}`));
            }

            this.addLI("<a href='#'>Add differential</a>", () => this.display.state = STATE_ADD_DIFFERENTIAL );
            this.addLI("<a href='#'>Remove differential</a>", () => this.display.state = STATE_RM_DIFFERENTIAL );
        });
    }

    addLI(html, callback) {
        let node = document.createElement("li");
        node.className = "list-group-item";
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback)
            node.addEventListener("click", callback);
        this.differential_list.appendChild(node);
    }
}

class StructlinePanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.structline_list = document.createElement("ul");
        this.structline_list.className = "list-group list-group-flush";
        this.structline_list.style["text-align"] = "center";
        this.addObject(this.structline_list);

        this.on("show", () => {
            while(this.structline_list.firstChild)
                this.structline_list.removeChild(this.structline_list.firstChild);

            let edges = this.display.selected.c.edges.filter(e => e.type === "Structline").sort((a, b) => a.page - b.page);

            let sname, tname;
            for (let e of edges) {
                sname = e.source.name ? e.source.name : "?"
                tname = e.target.name ? e.target.name : "?"
                if (e.source == this.display.selected.c)
                    this.addLI(Interface.renderMath(`{\\color{blue}${sname}} \\text{---} ${tname}`));
                else
                    this.addLI(Interface.renderMath(`${sname} \\text{---} {\\color{blue}${tname}}`));
            }

            this.addLI("<a href='#'>Add structline</a>", () => this.display.state = STATE_ADD_STRUCTLINE );
            this.addLI("<a href='#'>Remove structline</a>", () => this.display.state = STATE_RM_STRUCTLINE );
        });

    }

    addLI(html, callback) {
        let node = document.createElement("li");
        node.className = "list-group-item";
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback)
            node.addEventListener("click", callback);
        this.structline_list.appendChild(node);
    }
}

exports.Panel = Panel;
exports.TabbedPanel = TabbedPanel;
exports.DifferentialPanel = DifferentialPanel;
exports.StructlinePanel = StructlinePanel;
