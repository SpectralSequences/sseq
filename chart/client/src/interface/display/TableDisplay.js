"use strict"

import { SidebarDisplay } from "./SidebarDisplay.js";
import { TablePanel, DifferentialPanel } from "../Panel/mod.js";
import { Tooltip } from "../Tooltip.js";
import Mousetrap from "mousetrap";

export class TableDisplay extends SidebarDisplay {
    constructor(container, sseq) {
        super(container, sseq);
        this.tooltip = new Tooltip(this);
        this.on("mouseover-class", this._onMouseoverClass.bind(this));
        this.on("mouseout-class", this._onMouseoutClass.bind(this));
        this.on("mouseover-bidegree", this._onMouseoverBidegree.bind(this));
        this.on("mouseout-bidegree", this._onMouseoutBidegree.bind(this));

        this.on("click", this.__onClick.bind(this)); // Display already has an _onClick
        this.tablePanel = new TablePanel(this.sidebar.main_div, this);
        this.tablePanel.show();
    }

    __onClick() { // Display already has an _onClick
        if(this.selected_bidegree){
            if(
                this.mouseover_bidegree 
                && this.mouseover_bidegree[0] === this.selected_bidegree[0] 
                && this.mouseover_bidegree[1] === this.selected_bidegree[1]
            ){
                return;
            }
            console.log("hi");
            this.setBidegreeHighlight(this.selected_bidegree, false);
        }
        if(this.mouseover_bidegree) {
            this.selected_bidegree = this.mouseover_bidegree;
            this.setBidegreeHighlight(this.selected_bidegree, true);
        }
    }

    _onMouseoverClass(c) {
        this.tooltip.setHTML(`(${c.x}, ${c.y})`);
        this.tooltip.show(c._canvas_x, c._canvas_y);
        // c._highlight = true;
    }

    _onMouseoutClass(c) {
        this.tooltip.hide();
    }

    _onMouseoverBidegree(b){

    }

    _onMouseoutBidegree(b){
        
    }

    setBidegreeHighlight(b, highlight){
        let classes = this.sseq.classes_in_bidegree(b)
        for(let c of classes){
            c._highlight = highlight;
        }
        this.update();
    }

}
