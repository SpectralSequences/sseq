"use strict"

let SidebarDisplay = require("./SidebarDisplay.js").SidebarDisplay;
let Panel = require("../Panel/mod.js");
let Tooltip = require("../Tooltip.js").Tooltip;
let Mousetrap = require("mousetrap");

class TableDisplay extends SidebarDisplay {
    constructor(container, sseq) {
        super(container, sseq);
        
    }
}

exports.TableDisplay = TableDisplay;