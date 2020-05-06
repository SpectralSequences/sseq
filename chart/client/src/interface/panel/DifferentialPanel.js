import { renderMath } from "../Latex.js";
import { Panel } from "./Panel.js";

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
                    this.addLI(renderMath(`d_${e.page}({\\color{blue}${sname}}) = ${tname}`));
                else
                    this.addLI(renderMath(`d_${e.page}(${sname}) = {\\color{blue}${tname}}`));
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
const _DifferentialPanel = DifferentialPanel;
export { _DifferentialPanel as DifferentialPanel };
