import { renderMath } from "../Latex.js";
import { Panel } from "./Panel.js";

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
                    this.addLI(renderMath(`{\\color{blue}${sname}} \\text{---} ${tname}`));
                else
                    this.addLI(renderMath(`${sname} \\text{---} {\\color{blue}${tname}}`));
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

const _StructlinePanel = StructlinePanel;
export { _StructlinePanel as StructlinePanel };
