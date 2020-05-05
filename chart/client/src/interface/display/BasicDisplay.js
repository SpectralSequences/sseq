"use strict"

let Display = require("./Display.js").Display;
let Tooltip = require("../Tooltip.js").Tooltip;
let latex = require("../latex.js");
let Mousetrap = require("mousetrap");

class BasicDisplay extends Display {
    constructor(container, sseq, kwargs) {
        super(container, sseq);
        document.body.style.overflow = "hidden";
        this.page_indicator_div = this.container.append("div")
            .attr("id", "page_indicator")
            .style("position", "absolute")
            .style("left", "20px")
            .style("top","10px")
            .style("font-family","Arial")
            .style("font-size","15px");

        this.tooltip = new Tooltip(this);
        this.on("mouseover", (node) => {
            this.tooltip.setHTML(this.getClassTooltipHTML(node, this.page));
            this.tooltip.show(node._canvas_x, node._canvas_y);
        });
        this.on("mouseout", () => this.tooltip.hide());

        Mousetrap.bind('left',  this.previousPage);
        Mousetrap.bind('right', this.nextPage);
        Mousetrap.bind('x',
            () => {
                if(this.mouseover_node){
                    console.log(this.mouseover_node.c);
                }
            }
        );

        this.on("page-change", r => this.page_indicator_div.html(this.getPageDescriptor(r)));

        // Trigger page-change to set initial page_indicator_div
        this.setPage();

        this.status_div = this.container.append("div")
            .attr("id", "status")
            .style("position", "absolute")
            .style("left", `20px`)
            .style("bottom",`20px`)
            .style("z-index", 1000);
    }

    setStatus(html){
        if(this.status_div_timer){
            clearTimeout(this.status_div_timer);
        }
        this.status_div.html(html);
    }

    delayedSetStatus(html, delay){
        this.status_div_timer = setTimeout(() => setStatus(html), delay);
    }

    /**
     * Gets the tooltip for the current class on the given page (currently ignores the page).
     * @param c
     * @param page
     * @returns {string}
     */
    getClassTooltip(c, page) {
        let tooltip = c.getNameCoord();
        let extra_info = BasicDisplay.toTooltipString(c.extra_info, page);

        if (extra_info)
            tooltip += extra_info;

        return tooltip;
    }

    getClassTooltipHTML(c, page) {
        return latex.renderLatex(this.getClassTooltip(c,page));
    }

    static toTooltipString(obj, page) {
        if (!obj) {
            return false;
        }

        if(obj.constructor === String){
            return obj;
        }

        if(obj.constructor === Array) {
            return obj.map((x) => Tooltip.toTooltipString(x, page)).filter((x) => x).join("\n");
        }

        if(obj.constructor === Map){
            let lastkey;
            for (let k of obj.keys()) {
                if (k > page) {
                    break;
                }
                lastkey = k;
            }
            return BasicDisplay.toTooltipString(obj.get(lastkey));
        }

        return false;
    }
}

exports.BasicDisplay = BasicDisplay;
