"use strict"
import { Tooltip } from "chart/interface/Tooltip.js";
import Mousetrap from "mousetrap";

import { Display } from "chart/interface/display/Display.js";
import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;
import { SseqPageIndicator } from "chart/interface/display/SseqPageIndicator.js";
import { Panel } from "chart/interface/panel/Panel.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
window.SseqSocketListener = SseqSocketListener;

import { renderMath } from "chart/interface/Latex.js";

window.main = main;

function main(display, socket_address){
    let ws = new WebSocket(socket_address);
    window.socket_listener = new SseqSocketListener(ws);
    socket_listener.attachDisplay(display);
    Mousetrap.bind("left", display.previousPage)
    Mousetrap.bind("right", display.nextPage)
    Mousetrap.bind("t", () => {
        console.log("take?");
        socket_listener.send("console.take", {});
    });

    socket_listener.add_message_handler("interact.product_info", function(cmd, args, kwargs){
        let product_info = kwargs.product_info;
        let result = [];
        for(let [in1, in2, out] of product_info){
            if(in1[0] === 0 && in1[1] === 0) {
                continue;
            }
            result.push(`<div class="product">${renderMath(`(${in1}) * (${in2}) == ${JSON.stringify(out)}`)}</div>`);
        }
        let sidebar = document.querySelector("sseq-panel");
        sidebar.innerHTML = result.join("");
    })
    
    display.addEventListener("click", function(e){
        let sseq = display.sseq;
        let new_bidegree = e.detail[0].mouseover_bidegree;
        if(
            sseq._selected_bidegree
            && new_bidegree[0] == sseq._selected_bidegree[0] 
            && new_bidegree[1] == sseq._selected_bidegree[1]
        ){
            return;
        }        
        if(sseq._selected_bidegree){
            for(let c of sseq.classes_in_bidegree(...sseq._selected_bidegree)){
                c._highlight = false;
            }
        }
        sseq._selected_bidegree = new_bidegree;
        for(let c of sseq.classes_in_bidegree(...sseq._selected_bidegree)){
            c._highlight = true;
        }
        socket_listener.send("interact.select_bidegree", {"bidegree" : sseq._selected_bidegree});
        display.update();
    });

    socket_listener.start();
}
