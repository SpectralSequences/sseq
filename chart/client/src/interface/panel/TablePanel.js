import * as Latex from "../Latex.js";
import Panel from "./Panel.js";

export class TablePanel extends Panel {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        if (!this.display.selected)
            return;

        this.container.style.removeProperty("display");
        this.container.style.textAlign = "center";
        this.clear();

        this.newGroup();
        this.addHeader("Classes");
        let [x, y] = this.display.selected;
        let page = this.display.page;
        let sseq = this.display.sseq;

        let classes = sseq.getClasses(x, y, page);
        let names = sseq.classNames.get(x, y);

        let div = document.createElement("div");
        for (let c of classes) {
            let n = document.createElement("span");
            n.style.padding = "0 0.6em";
            n.innerHTML = katex.renderToString(vecToName(c, names));
            if (this.display.constructor.name != "CalculationDisplay" && classes.length == sseq.classes.get(x, y)[0].length) {
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

        let decompositions = sseq.decompositions.get(x, y);
        if (decompositions && decompositions.length > 0) {
            this.newGroup();
            this.addHeader("Decompositions");
            for (let d of decompositions) {
                let single = d[0].reduce((a, b) => a + b, 0) == 1;
                single = single && this.display.constructor.name != "CalculationDisplay";

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
    }
}