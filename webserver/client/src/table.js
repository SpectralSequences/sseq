"use strict"
import { Tooltip } from "chart/interface/Tooltip.js";
import Mousetrap from "mousetrap";

import { Display } from "chart/interface/display/Display.js";
import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;
import { SseqPageIndicator } from "chart/interface/display/SseqPageIndicator.js";
import { Panel } from "chart/interface/panel/Panel.js";
import { Matrix } from "chart/interface/panel/Matrix.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
window.SseqSocketListener = SseqSocketListener;

import { renderMath } from "chart/interface/Latex.js";

window.main = main;

function main(display, socket_address){
    // let matrix = document.querySelector("sseq-matrix");
    // matrix.value = [
    //     [0,0,0],
    //     [1, 0, 1],
    //     [1,1,1]
    // ];
    // matrix.addEventListener("matrix-click", (e) => {
    //     let row = e.detail.row_idx;
    //     if(matrix.selectedRows.includes(row)){
    //         matrix.selectedRows = [];
    //     } else {
    //         matrix.selectedRows = [e.detail.row_idx];
    //     }
    // });

    // Mousetrap(matrix).bind("escape", () =>{
    //     matrix.selectedRows = [];
    // });


    let ws = new WebSocket(socket_address);
    window.socket_listener = new SseqSocketListener(ws);
    socket_listener.attachDisplay(display);
    Mousetrap.bind("left", display.previousPage)
    Mousetrap.bind("right", display.nextPage)
    Mousetrap.bind("t", () => {
        socket_listener.send("console.take", {});
    });


    function productMouseover(e){
        console.log(e);
    }

    function productMouseout(e){
        console.log(e);
    }

    socket_listener.add_message_handler("interact.product_info", function(cmd, args, kwargs){
        let sseq = display.sseq;
        let product_info = kwargs.product_info;
        let names = kwargs.names;
        let matrix = kwargs.matrix;
        let result = [];
        for(let [[in1,mono1], [in2, mono2], out] of product_info){
            if(in1[0] === 0 && in1[1] === 0) {
                continue;
            }
            result.push(`${in1} \\cdot ${in2} = ${JSON.stringify(out)}`);
        }
        let sidebar = document.querySelector("sseq-panel");
        let div = document.createElement("div");
        if(result.length > 0){
            div.innerHTML = `
                <div style="overflow: overlay; display:flex; flex-direction:column; padding-right: 1.5rem;">
                    <h5 style="">
                    Classes in (${sseq._selected_bidegree.join(", ")})
                    </h5>
                    <p style="align-self: center; display:inherit;">
                        ${
                            names
                                .map(e => `<katex-expr class="name">${e}</katex-expr>`)
                                .join(`, <span style="padding-right:6pt; display:inline-block;"></span>`)
                        }
                    </p>
                
                    <h5 style="">
                        Products
                    </h5>
                    <div style="align-self: center;">
                        <div style="text-align:right;">
                            ${result.map(e => `
                                <div class="product-item"> ${renderMath(e)}</div>
                            `).join("")}
                        </div>
                    </div>
                
                    <h5 style="margin-top:12pt;">Matrix:</h5>
                    <sseq-matrix type="display" style="align-self:center;"></sseq-matrix>
                </div>
                `
        } else {
            div.innerHTML = `<div>
                <p></p>
            </div>`;
        }
        div.querySelectorAll(".product-item").forEach((e, idx) => {
            e.addEventListener("click",  () => {
                socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });
        div.style.display = "flex";
        div.style.flexDirection = "column";
        div.style.height = "90%";
        div.querySelector("sseq-matrix").value = matrix;
        div.querySelector("sseq-matrix").labels = names;
        sidebar.innerHTML = "";
        sidebar.appendChild(div);
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
        if(sseq.classes_in_bidegree(...new_bidegree).length == 0){
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
