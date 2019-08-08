class StructlinePanel extends Panel.Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        this.container.style.removeProperty("display");
        this.clear();

        this.newGroup();

        let types = Array.from(this.display.sseq.getStructlineTypes()).sort();
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
            i.checked = true;
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
    }
}

export default class MyDisplay extends SidebarDisplay {
    constructor(container, sseq) {
        super(container, sseq);

        this.tooltip = new Tooltip(this);
        this.on("mouseover", (node) => {
            this.tooltip.setHTML(`(${node.c.x}, ${node.c.y})`);
            this.tooltip.show(node.x, node.y);
        });

        this.structlinePanel = new StructlinePanel(this.sidebar.main_div, this);
        this.sidebar.addPanel(this.structlinePanel);
        this.sidebar.currentPanel = this.structlinePanel;

        this.sidebar.footer.newGroup();

        this.sidebar.footer.currentGroup.style.textAlign = "center";
        this.runningSign = document.createElement("p");
        this.runningSign.className = "card-text"
        this.runningSign.innerHTML = "Running...";
        this.sidebar.footer.addObject(this.runningSign);

        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG("sseq.svg"));
        this.sidebar.footer.addButton("Save", () => this.sseq.download("sseq.json"));
    }
}
