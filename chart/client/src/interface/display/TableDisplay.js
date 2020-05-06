"use strict"

import { SidebarDisplay } from "./SidebarDisplay.js";
import Panel from "../Panel/mod.js";
import { Tooltip } from "../Tooltip.js";
import Mousetrap from "mousetrap";

class TableDisplay extends SidebarDisplay {
    constructor(container, sseq) {
        super(container, sseq);
        
    }
}

const _TableDisplay = TableDisplay;
export { _TableDisplay as TableDisplay };