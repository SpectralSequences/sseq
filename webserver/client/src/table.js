"use strict"

import {LitElement, html} from 'lit-element';


export class MyElement extends LitElement {
    render() {
        return html`<p>template content <slot>something</slot> more words <slot name="one"></slot></p>`;
    }

    test(){
        alert("hi");
    }
}
customElements.define('my-element', MyElement);


import { TablePanel, DifferentialPanel } from "chart/interface/panel/mod.js";
import { Tooltip } from "chart/interface/Tooltip.js";
import Mousetrap from "mousetrap";

// import { SocketDisplay } from "chart/SocketDisplay.js";
import { Display } from "chart/interface/display/Display.js";
import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;
import { SseqPageIndicator } from "chart/interface/display/SseqPageIndicator.js";
import { Panel } from "chart/interface/panel/Panel.js";
// window.


// export class TableDisplay extends Display {
//     constructor(container, sseq) {
//         super(container, sseq);
//         this.tooltip = new Tooltip(this);
//         this.on("mouseover-class", this._onMouseoverClass.bind(this));
//         this.on("mouseout-class", this._onMouseoutClass.bind(this));
//         this.on("mouseover-bidegree", this._onMouseoverBidegree.bind(this));
//         this.on("mouseout-bidegree", this._onMouseoutBidegree.bind(this));

//         this.on("click", this._onClick.bind(this));
//         this.tablePanel = new TablePanel(this.sidebar.main_div, this);
//         this.tablePanel.show();
//     }

//     _onClick() {
//         if(this.selected_bidegree){
//             if(
//                 this.mouseover_bidegree 
//                 && this.mouseover_bidegree[0] === this.selected_bidegree[0] 
//                 && this.mouseover_bidegree[1] === this.selected_bidegree[1]
//             ){
//                 return;
//             }
//             console.log("hi");
//             this.setBidegreeHighlight(this.selected_bidegree, false);
//         }
//         if(this.mouseover_bidegree) {
//             this.selected_bidegree = this.mouseover_bidegree;
//             this.setBidegreeHighlight(this.selected_bidegree, true);
//         }
//     }

//     _onMouseoverClass(c) {
//         this.tooltip.setHTML(`(${c.x}, ${c.y})`);
//         this.tooltip.show(c._canvas_x, c._canvas_y);
//         // c._highlight = true;
//     }

//     _onMouseoutClass(c) {
//         this.tooltip.hide();
//     }

//     _onMouseoverBidegree(b){

//     }

//     _onMouseoutBidegree(b){
        
//     }

//     setBidegreeHighlight(b, highlight){
//         let classes = this.sseq.classes_in_bidegree(b)
//         for(let c of classes){
//             c._highlight = highlight;
//         }
//         this.update();
//     }

// }
