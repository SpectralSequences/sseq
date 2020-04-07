"use strict"

let Display = require("./Display.js").Display;
let Panel = require("./Panel.js");

class Sidebar {
    constructor(parentContainer) {
        this.adjuster = document.createElement("div");
        this.adjuster.style.backgroundColor = "rgba(0,0,0,0.125)";
        this.adjuster.style.height = "100%";
        this.adjuster.style.cursor = "ew-resize";
        this.adjuster.style.width = "2px";

        parentContainer.appendChild(this.adjuster);

        this.resize = this.resize.bind(this);
        this.stopResize = this.stopResize.bind(this);

        this.adjuster.addEventListener("mousedown", (function(e) {
            e.preventDefault();
            window.addEventListener('mousemove', this.resize);
            window.addEventListener('mouseup', this.stopResize);
        }).bind(this));

        this.sidebar = document.createElement("div");
        this.sidebar.style.height = "100%";
        this.sidebar.style.width = "240px";
        this.sidebar.style.border = "none";
        this.sidebar.style.display = "flex";
        this.sidebar.style.flexDirection = "column";
        this.sidebar.className = "card";

        parentContainer.appendChild(this.sidebar);

        this.main_div = document.createElement("div");
        this.main_div.style.overflow = "auto";
        this.sidebar.appendChild(this.main_div);

        let filler = document.createElement("div");
        filler.style.flexGrow = "1";
        this.sidebar.appendChild(filler);

        this.footer_div = document.createElement("div");
        this.sidebar.appendChild(this.footer_div);

        this.panels = [];
        this.currentPanel = null;
    }

    addPanel(panel) {
        this.panels.push(panel);
        return this.panels.length;
    }

    init(display) {
        this.display = display;
        this.footer = new Panel.Panel(this.footer_div, display);
    }

    resize(e) {
        let width = this.sidebar.getBoundingClientRect().right - e.pageX;
        this.sidebar.style.width = `${width}px`;
    }

    stopResize() {
        window.removeEventListener('mousemove', this.resize);
        window.removeEventListener('mouseup', this.stopResize);
        this.display.resize();
    }

    showPanel(panel) {
        if (!panel) panel = this.currentPanel;
        this.currentPanel = panel;

        for (let x of this.panels) {
            if (x == panel)
                x.show();
            else
                x.hide();
        }
    }
}

class SidebarDisplay extends Display {
    constructor(container, sseq) {
        if (typeof container == "string")
            container = document.querySelector(container);

        container.style.display = "flex";
        container.style.displayDirection = "row";

        let child = document.createElement("div");
        child.style.height = "100%";
        child.style.minHeight = "100%";
        child.style.overflow = "hidden";
        child.style.position = "relative";
        child.style.flexGrow = "1";

        container.appendChild(child);

        let sidebar = new Sidebar(container)

        super(child, sseq);

        this.sidebar = sidebar;
        this.sidebar.init(this);
    }
}

exports.SidebarDisplay = SidebarDisplay;
