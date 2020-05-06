import { Panel } from "./Panel.js";

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

const _TabbedPanel = TabbedPanel;
export { _TabbedPanel as TabbedPanel };
