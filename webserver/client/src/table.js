"use strict"
import Mousetrap from "mousetrap";

import { SpectralSequenceChart } from "chart/sseq/SpectralSequenceChart.js";
window.SpectralSequenceChart = SpectralSequenceChart;

import ReconnectingWebSocket from 'reconnecting-websocket';

import { UIElement } from "chart/interface/UIElement.js";
import { Display } from "chart/interface/Display.js";
import { AxesElement } from "chart/interface/Axes.js";
import { GridElement } from "chart/interface/GridElement.js";
import { ChartElement } from "chart/interface/ChartElement.js";
import { ClassHighlighter } from "chart/interface/ClassHighlighter";
import { SseqPageIndicator } from "chart/interface/SseqPageIndicator.js";
import { Tooltip } from "chart/interface/Tooltip.js";



import { Panel } from "chart/interface/Panel.js";
import { Matrix } from "chart/interface/Matrix.js";
import { KatexExprElement } from "chart/interface/KatexExprElement.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { Popup } from "chart/interface/Popup.js";
import { sleep, promiseFromDomEvent } from "chart/interface/utils.js";

window.SseqSocketListener = SseqSocketListener;


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


    let ws = new ReconnectingWebSocket(socket_address, [], 
        {
            debug : true,
            minReconnectionDelay: 100,
            maxReconnectionDelay: 1000,
        }
    );
    window.socket_listener = new SseqSocketListener(ws);
    socket_listener.attachDisplay(display);
    Mousetrap.prototype.stopCallback = function(e, element, combo){
        // Find the correct target of the event inside the shadow dom
        while(element.shadowRoot && element.shadowRoot.activeElement){
            element = element.shadowRoot.activeElement;
        }
        // if the element has the class "mousetrap" then no need to stop
        if(element.matches(".mousetrap")) {
            return false;
        }
        // Is the key printable?
        let keyCode = e.keyCode;
        let printable = 
            (keyCode > 47 && keyCode < 58)   || // number keys
            keyCode == 32   || // space
            (keyCode > 64 && keyCode < 91)   || // letter keys
            (keyCode > 95 && keyCode < 112)  || // numpad keys
            (keyCode > 185 && keyCode < 193) || // ;=,-./` (in order)
            (keyCode > 218 && keyCode < 223);   // [\]' (in order

        // Is the element a text input?
        let in_text_input = element.matches("input, select, textarea") || (element.contentEditable && element.contentEditable == 'true');
        return printable && in_text_input;
    }

    Mousetrap.bind("left", display.previousPage)
    Mousetrap.bind("right", display.nextPage)
    Mousetrap.bind("t", () => {
        socket_listener.send("console.take", {});
    });

    let enterPressed = false;
    let spacePressed = false;
    Mousetrap.bind("enter", handleEnter, "keydown");
    Mousetrap.bind("enter", () => { enterPressed = false; }, "keyup");
    Mousetrap.bind("space", handleSpace, "keydown");
    Mousetrap.bind("space", () => { spacePressed = false; }, "keyup");

    function handleSpace(e){
        if(spacePressed){
            return
        }
        spacePressed = true;
        console.log(e);
        let elt = document.activeElement;
        if(!elt){
            return;
        }
        while(elt.shadowRoot && elt.shadowRoot.activeElement){
            elt = elt.shadowRoot.activeElement;
        }
        elt.dispatchEvent(new CustomEvent("interact-toggle", {
            bubbles : true,
            composed : true,
            detail : { "originalEvent" : e }
        }));
    }

    function handleEnter(e){
        if(enterPressed){
            return
        }
        enterPressed = true;        
        let elt = document.activeElement;
        if(!elt){
            return;
        }
        while(elt.shadowRoot && elt.shadowRoot.activeElement){
            elt = elt.shadowRoot.activeElement;
        }

        elt.dispatchEvent(new CustomEvent("interact-submit", {
            bubbles : true,
            composed : true,
            detail : { "originalEvent" : e }
        }));
    }

    function productMouseover(e){
        console.log(e);
    }

    function productMouseout(e){
        console.log(e);
    }


    let popup = document.querySelector("sseq-popup");

    let names;
    let nameMonos;
    let product_info;
    let matrix;
    let selected_bidegree;
    socket_listener.add_message_handler("interact.product_info", function(cmd, args, kwargs){
        let sseq = display.querySelector("sseq-chart");
        names = [];
        nameMonos = [];
        console.log(kwargs.names);
        for(let [name, mono] of kwargs.names){
            names.push(name);
            nameMonos.push(mono);
        }
        console.log("names");
        product_info = kwargs.product_info;
        matrix = kwargs.matrix;
        let result = [];
        for(let [[in1,name1, mono1], [in2, name2, mono2], out, preimage, possible_name] of product_info){
            let name_str = "";
            if(possible_name){
                name_str = `{}= ${possible_name}`
            }
            result.push([`${name1} \\cdot ${name2} = ${JSON.stringify(out)}`, name_str]);
        }
        let sidebar = document.querySelector("sseq-panel");
        let class_html = "";
        let product_html = "";
        let matrix_html = "";
        class_html = `
            <h5>
            Classes in (${selected_bidegree.join(", ")})
            </h5>
            <p style="align-self: center;">
                ${names.map(e => `<katex-expr tabindex=0 class="name">${e}</katex-expr>`)
                        .join(`, <span style="padding-right:6pt; display:inline-block;"></span>`)}
            </p>
        `;        
        if(result.length > 0){
            product_html = `
                <h5 style="">
                    Products
                </h5>
                <div class="product-list" style="align-self: center; width: max-content; overflow: hidden;">
                    <table><tbody>
                        ${result.map(([e, n]) => `
                            <tr class="product-item" tabindex=0>
                                <td align='right'><katex-expr>${e}</katex-expr></td>
                                <td><katex-expr>${n}</katex-expr></td>
                            </tr>
                        `).join("")}
                    </tbody></table>
                </div>            
            `;

            matrix_html = `
                <h5 style="margin-top:12pt;">Matrix:</h5>
                <sseq-matrix style="align-self:center;"></sseq-matrix>
            `;
        }
        sidebar.querySelector("#product-info-classes").innerHTML = class_html;
        sidebar.querySelector("#product-info-products").innerHTML = product_html;
        sidebar.querySelector("#product-info-matrix").innerHTML = matrix_html;

        sidebar.querySelectorAll(".name").forEach((e, idx) => {
            e.addEventListener("click",  () => {
                handleNameItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        })

        sidebar.querySelectorAll(".product-item").forEach((e, idx) => {
            e.addEventListener("click",  () => {
                handleProductItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });
        sidebar.querySelector("sseq-matrix").value = matrix;
        sidebar.querySelector("sseq-matrix").labels = names;
        sidebar.displayChildren("#product-info");

        // div.style.display = "flex";
        // div.style.flexDirection = "column";
        // div.style.height = "90%";
    })

    async function handleNameItemClick(item_idx){
        let popup_header = popup.querySelector("[slot=header]");
        let popup_body = popup.querySelector("[slot=body]");
        let hasName = nameMonos[item_idx] !== null;
        let name = names[item_idx];
        let nameWord = hasName ? "Rename" : "Name";
        popup_header.innerText = `${nameWord} class?`;
        let tuple = [...selected_bidegree, item_idx];
        popup_body.innerHTML =`   
            Input ${hasName ? "new " : ""}name for class (${tuple.join(", ")}):
            <input type="text" focus style="width : 100%; margin-top : 0.6rem;">
        `;
        let input = popup_body.querySelector("input");
        input.addEventListener("focus", () => input.select())
        if(hasName){
            input.value = name;
        }
        popup.show();
        let ok_q = await popup.submited();
        if(!ok_q){
            return;
        }
        socket_listener.send("interact.name_class.free", {"class_tuple" : tuple,  "name" : product_data});
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }
    
    async function handleProductItemClick(item_idx){
        let sseq = display.querySelector("sseq-chart").sseq;
        let jsoned_matrix = matrix.map(JSON.stringify);
        let product_data = product_info[item_idx];
        let [[in1, _name1, _nm1], [in2, _name2, _nm2], out_res_basis, out_our_basis, out_name] = product_data;
        console.log("out_name", out_name);
        let one_entries = [];
        out_our_basis.forEach((v, idx) => {
            if(v === 1){
                one_entries.push(idx);
            }
        });
        let inbasis = one_entries.length === 1;
        let highlightClasses = [sseq.class_by_index(...in1), sseq.class_by_index(...in2)];
        let out_deg = [in1[0] + in2[0], in1[1] + in2[1]];
        for(let idx of one_entries){
            highlightClasses.push(sseq.class_by_index(...out_deg, idx));
        }
        let class_highlighter = document.querySelector("sseq-class-highlighter");
        class_highlighter.clear();
        class_highlighter.fire(highlightClasses, 0.8);        
        if(inbasis){
            let out_tuple = [...out_deg, one_entries[0]];
            await handleProductItemClick_inBasis(product_data, out_tuple );
        } else {
            await handleProductItemClick_changeBasis(product_data, one_entries);
        }
    }

    async function handleProductItemClick_inBasis(product_data, out_tuple){
        let [[_in1, name1, _nm1], [_in2, name2, _nm2], out_res_basis, out_our_basis, out_name] = product_data;
        popup.okEnabled = true;
        let popup_header = popup.querySelector("[slot=header]");
        let popup_body = popup.querySelector("[slot=body]");
        let nameWord = out_name ? "Rename" : "Name";
        popup_header.innerText = `${nameWord} class?`;
        popup_body.innerHTML = `
            <p>${nameWord} class (${out_tuple.join(", ")}) as <katex-expr>${name1}\\cdot ${name2}</katex-expr>?</p>
            ${out_name ? `<p>Current name is <katex-expr>${out_name}</katex-expr>.</p>` : ``}
        `;
        popup.show();
        let ok_q = await popup.submited();
        console.log(ok_q);
        if(!ok_q){
            return;
        }
        socket_listener.send("interact.name_class.product.in_basis", {"product_data" : product_data});        
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }
    
    async function handleProductItemClick_changeBasis(product_data, one_entries){
        popup.okEnabled = false;
        let popup_header = popup.querySelector("[slot=header]");
        let popup_body = popup.querySelector("[slot=body]");        
        popup_header.innerText = "Update basis?";
        let new_body = document.createElement("div");
        new_body.innerHTML = `
            Select a basis vector to replace:
            <p><sseq-matrix type=select-row></sseq-matrix></p>
        `;
        await sleep(0); // Allow matrix to render
        popup_body.innerHTML = "";
        popup_body.appendChild(new_body);
        let matrix_elt = new_body.querySelector("sseq-matrix");
        matrix_elt.value = matrix;
        matrix_elt.labels = names;
        popup.okEnabled = false;
        matrix_elt.addEventListener("matrix-select", (e) => {
            popup.okEnabled = matrix_elt.selectedRows.length > 0;
        });
        // TODO: disable rows of matrix not set in one_entries.
        popup.show();
        let ok_q = await popup.submited();
        if(!ok_q){
            return;
        }
        socket_listener.send("interact.name_class.product.change_basis", {"product_data" : product_data, replace_row : matrix_elt.selectedRows[0] });
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }

    display.addEventListener("click", function(e){
        let sseq = display.querySelector("sseq-chart").sseq;
        let new_bidegree = e.detail[0].mouseover_bidegree;
        if(
            selected_bidegree
            && new_bidegree[0] == selected_bidegree[0] 
            && new_bidegree[1] == selected_bidegree[1]
        ){
            return;
        }
        let classes = sseq.classes_in_bidegree(...new_bidegree);
        if(classes.length == 0){
            return;
        }
        selected_bidegree = new_bidegree;
        let class_highlighter = document.querySelector("sseq-class-highlighter");
        let result = class_highlighter.clear();
        class_highlighter.highlight(classes, 0.8);
        
        
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
        display.update();
    });

    // display.addEventListener("mouseover-class", (e) => {
    //     let [c, ms] = e.detail;
    //     document.querySelector("sseq-class-highlighter").fire(c);
        
    // });

    socket_listener.start();
}
