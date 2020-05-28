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
import { BidegreeHighlighter } from "chart/interface/BidegreeHighlighter";
import { SseqPageIndicator } from "chart/interface/SseqPageIndicator.js";
import { Tooltip } from "chart/interface/Tooltip.js";



import { Panel } from "chart/interface/Panel.js";
import { Matrix } from "chart/interface/Matrix.js";
import { KatexExprElement } from "chart/interface/KatexExprElement.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { Popup } from "chart/interface/Popup.js";
import { sleep, promiseFromDomEvent, throttle } from "chart/interface/utils.js";

window.SseqSocketListener = SseqSocketListener;


window.main = main;

function main(display, socket_address){

    let ws = new ReconnectingWebSocket(socket_address, [], 
        {
            /** debug : true, /**/
            minReconnectionDelay: 100,
            maxReconnectionDelay: 1000,
        }
    );
    window.socket_listener = new SseqSocketListener(ws);
    socket_listener.attachDisplay(display);
    Mousetrap.prototype.stopCallback = stopCallback;
    function stopCallback(e, element, combo){
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
            (keyCode >= 48 && keyCode < 58) // number keys
            || keyCode == 32 // space
            || (keyCode >= 37 && keyCode < 41) // Arrow keys (okay technically not printable but they do things in text boxes)
            || (keyCode >= 65 && keyCode < 91)    // letter keys
            || (keyCode >= 96 && keyCode < 112)   // numpad keys
            || (keyCode >= 186 && keyCode < 193)  // ;=,-./` (in order)
            || (keyCode >= 219 && keyCode < 223) ;   // [\]' (in order

        // Is the element a text input?
        let in_text_input = element.matches("input, select, textarea") || (element.contentEditable && element.contentEditable == 'true');
        return printable && in_text_input;
    }

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

    let popup = document.querySelector("sseq-popup");
    let sidebar = document.querySelector("sseq-panel");
    sidebar.addEventListener("interact-toggle", () => {
        document.activeElement.click();
    });
    sidebar.addEventListener("interact-submit", () => {
        document.activeElement.click();
    });

    function namedVecsListToObj(namedVecs){
        let result = {};
        for(let [k, v] of namedVecs){
            result[JSON.stringify(k)] = v;
        }
        return result;
    }

    function namedVecsObjToList(namedVecs){
        let result = [];
        for(let [k, v] of Object.entries(namedVecs)){
            result.push([JSON.parse(k), v]);
        }
        return result;
    }


    let names;
    let namedVecs;
    let nameMonos;
    let product_info;
    let matrix;
    let selected_bidegree;
    socket_listener.add_message_handler("interact.product_info", async function(cmd, args, kwargs){
        let sseq = display.querySelector("sseq-chart");
        namedVecs = namedVecsListToObj(kwargs.named_vecs);
        names = [];
        nameMonos = [];
        for(let [name, mono] of kwargs.names){
            names.push(name);
            nameMonos.push(mono);
        }
        let selectedIndex = undefined;
        if(document.activeElement.closest("sseq-panel")){
            selectedIndex = sidebar.querySelectorAll("[tabindex='0']");
            selectedIndex = selectedIndex && Array.from(selectedIndex);
            selectedIndex = selectedIndex && selectedIndex.indexOf(document.activeElement);
        }
        let name_group = 1;
        let group = name_group;
        product_info = kwargs.product_info;
        matrix = kwargs.matrix;
        let result = [];
        let out_vecs = {};
        for(let { left : [in1,name1, ], right : [in2, name2, ], out_name, out_res_basis} of product_info){
            let name_str = "";
            if(out_name){
                name_str = `{}= ${out_name}`
            }
            if(!(JSON.stringify(out_res_basis) in out_vecs)){
                group ++;
                out_vecs[JSON.stringify(out_res_basis)] = 1;
            }
            result.push([`${name1} \\cdot ${name2} = ${JSON.stringify(out_res_basis)}`, name_str, group]);
        }
        let matrix_group = group + 1;
        let bidegree_html = `<h4>Bidegree (${selected_bidegree.join(", ")})</h4>`;
        let class_html = "";
        let product_html = "";
        let matrix_html = "";
        class_html = `
            <h5>Classes</h5>
            <p style="align-self: center;">
                ${names.map(e => `<katex-expr class="name" group="${name_group}">${e}</katex-expr>`)
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
                        ${result.map(([e, n, group]) => `
                            <tr class="product-item" focus group="${group}">
                                <td align='right'><katex-expr>${e}</katex-expr></td>
                                <td><katex-expr>${n}</katex-expr></td>
                            </tr>
                        `).join("")}
                    </tbody></table>
                </div>            
            `;
        }
        matrix_html = `
            <h5 style="margin-top:12pt;">Matrix</h5>
            <sseq-matrix style="align-self:center;" group="${matrix_group}"></sseq-matrix>
        `;
        sidebar.querySelector("#product-info-bidegree").innerHTML = bidegree_html;
        sidebar.querySelector("#product-info-classes").innerHTML = class_html;
        sidebar.querySelector("#product-info-products").innerHTML = product_html;
        sidebar.querySelector("#product-info-matrix").innerHTML = matrix_html;
        let matrixElt = sidebar.querySelector("sseq-matrix");
        matrixElt.value = matrix;
        matrixElt.labels = names.map((n, idx) => nameMonos[idx] ? n : "");
        matrixElt.tabIndex = '0';
        matrixElt.addEventListener("click",  () => {
            handleMatrixEditClick();
        });
        sidebar.displayChildren("#product-info");

        sidebar.querySelectorAll(".name").forEach((e, idx) => {
            e.tabIndex = 0;
            e.addEventListener("click",  () => {
                handleNameItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });
        Array.from(sidebar.querySelectorAll(".product-item"))
             .map((e,idx) => [e,idx])
             .filter((_, idx) => product_info[idx].left[2] && product_info[idx].right[2])
        .forEach(([e, idx]) => {
            e.tabIndex = 0;
            e.addEventListener("click",  () => {
                handleProductItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });

        await sleep(0);

        let focusElt;
        if(selectedIndex){
            focusElt = sidebar.querySelectorAll("[tabindex='0']")[selectedIndex];
        }
        focusElt = focusElt || sidebar.querySelector(".product-item[tabindex='0']");
        focusElt = focusElt || sidebar.querySelector("[tabindex='0']");
        if(focusElt){
            focusElt.focus();
            sleep(50).then(() => focusElt.focus());
        }        

        // div.style.display = "flex";
        // div.style.flexDirection = "column";
        // div.style.height = "90%";
    });

    function setError(type, message){
        if(type !== undefined){
            let input = popup.querySelector("input");
            if(input){
                input.setCustomValidity(message);
            }
            let errorElt = popup.querySelector(".error");
            errorElt.classList.add("active");
            errorElt.error_type = type;
            errorElt.innerHTML = message;
        } else {
            clearError();
        }
    }

    function clearError(type){
        let input = popup.querySelector("input");
        let errorElt = popup.querySelector(".error");
        if(type !== undefined && errorElt.error_type !== type){
            return;
        }
        if(input){
            input.setCustomValidity("");
        }
        errorElt.classList.remove("active");
        errorElt.innerHTML = "";
    }

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
            <span class="error"></span>
        `;
        let input = popup_body.querySelector("input");
        input.addEventListener("focus", () => {
            input.select();
            clearError("UnexpectedEOF");
        });
        input.addEventListener("input", async () => {
            let [validated, error] = await validateName(input.value);
            popup.okEnabled = validated;
            validated = validated || error.name === "UnexpectedEOF";
            if(validated){
                clearError();
            } else {
                setError(error.name, `${error.name} column: ${error.column}`);
            }
        });
        input.addEventListener("blur", async () => {
            let [validated, error] = await validateName(input.value);
            popup.okEnabled = validated;
            if(validated){
                clearError();
            } else {
                setError(error.name, error.name);
            }
        });        
        if(hasName){
            input.value = name;
        }
        popup.show();
        let ok_q = await popup.submited();
        if(!ok_q){
            return;
        }
        let vec = Array(names.length).fill(0).map((_e, idx) => idx === item_idx ? 1 : 0);
        namedVecs[JSON.stringify(vec)] = input.value;
        console.log("sending action");
        socket_listener.send("interact.action", 
            {
                "bidegree" : selected_bidegree,  
                "named_vecs" : namedVecsObjToList(namedVecs),
                "matrix" : matrix
            }
        );        
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }
    
    async function handleProductItemClick(item_idx){
        let sseq = display.querySelector("sseq-chart").sseq;
        let jsoned_matrix = matrix.map(JSON.stringify);
        let product_data = product_info[item_idx];
        let { left : [in1, ,], right : [in2, ,], out_our_basis, out_res_basis } = product_data;
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
            await handleProductItemClick_changeBasis(product_data, out_our_basis);
        }
    }

    async function handleProductItemClick_inBasis(product_data, out_tuple){
        let { left : [, name1,], right : [, name2,], out_name, out_res_basis } = product_data;
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
        if(!ok_q){
            return;
        }
        namedVecs[JSON.stringify(out_res_basis)] = `${name1} ${name2}`;
        socket_listener.send("interact.action", 
            {
                "bidegree" : selected_bidegree,  
                "named_vecs" : namedVecsObjToList(namedVecs),
                "matrix" : matrix
            }
        );
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }
    
    async function handleProductItemClick_changeBasis(product_data, out_our_basis){
        popup.okEnabled = false;
        let {left : [, name1, ], right :[, name2, ], out_res_basis} = product_data;
        let popup_header = popup.querySelector("[slot=header]");
        let popup_body = popup.querySelector("[slot=body]");        
        popup_header.innerText = "Update basis?";
        let new_body = document.createElement("div");
        new_body.innerHTML = `
            Select a basis vector to replace with 
            <p><katex-expr>${name1} \\cdot ${name2} = ${JSON.stringify(out_res_basis)}</katex-expr>:</p>
            <p><sseq-matrix focus type=select-row></sseq-matrix></p>
        `;
        await sleep(0); // Allow matrix to render
        popup_body.innerHTML = "";
        popup_body.appendChild(new_body);
        let matrix_elt = new_body.querySelector("sseq-matrix");
        matrix_elt.value = matrix;
        matrix_elt.labels = names;
        matrix_elt.enabledRows = out_our_basis.map(e => e !== 0);
        popup.okEnabled = false;
        matrix_elt.addEventListener("matrix-select", (e) => {
            popup.okEnabled = matrix_elt.selectedRows.length > 0;
            if(matrix_elt.selectedRows.length > 0){
                let matrix_clone = matrix.map(r => r.slice());
                let replace_row = matrix_elt.selectedRows[0];
                matrix_clone[replace_row] = out_res_basis;
                validateMatrix(selected_bidegree, matrix_clone);
            } else {
                socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            }
            console.log("      matrix:", JSON.stringify(matrix));
            console.log("matrix_clone:", JSON.stringify(matrix_clone) );
        });
        // TODO: disable rows of matrix not set in one_entries.
        popup.show();
        let ok_q = await popup.submited();
        if(!ok_q){
            socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            return;
        }
        let result_matrix = matrix_elt.value;
        let replace_row = matrix_elt.selectedRows[0];
        result_matrix[replace_row] = out_res_basis;
        namedVecs[JSON.stringify(out_res_basis)] = `${name1} ${name2}`;
        socket_listener.send("interact.action", 
            {
                "bidegree" : selected_bidegree,  
                "named_vecs" : namedVecsObjToList(namedVecs),
                "matrix" : result_matrix
            }
        );
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }

    async function handleMatrixEditClick(){
        let popup_header = popup.querySelector("[slot=header]");
        let popup_body = popup.querySelector("[slot=body]");        
        popup_header.innerText = "Update basis?";
        let new_body = document.createElement("div");
        new_body.innerHTML = `
            <p><sseq-matrix focus type=input></sseq-matrix></p>
        `;
        await sleep(0); // Allow matrix to render
        popup_body.innerHTML = "";
        popup_body.appendChild(new_body);
        let matrix_elt = new_body.querySelector("sseq-matrix");
        matrix_elt.value = matrix.map(e => e.slice());
        matrix_elt.labels = names;
        matrix_elt.addEventListener("change", async (e) => {
            let { singular, row_labels } = await validateMatrix(selected_bidegree, matrix_elt.value);
            popup.okEnabled = !singular;
            matrix_elt.labels = row_labels;
            if(JSON.stringify(matrix_elt.value) === JSON.stringify(matrix)){
                socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            }
        });
        // matrix_elt.addEventListener("blur", (e) => console.log("blurred", e));
        popup.show();
        let ok_q = await popup.submited();
        if(!ok_q){
            socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            return;
        }
        socket_listener.send("interact.action", 
            {
                "bidegree" : selected_bidegree,  
                "named_vecs" : namedVecsObjToList(namedVecs),
                "matrix" : matrix_elt.value
            }
        );
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }

    display.addEventListener("click", function(e){
        let sseq = display.querySelector("sseq-chart").sseq;
        let new_bidegree = e.detail[0].mouseover_bidegree;
        if(!new_bidegree){
            return;
        }
        if(
            selected_bidegree
            && new_bidegree[0] == selected_bidegree[0] 
            && new_bidegree[1] == selected_bidegree[1]
        ){
            let focusElt = document.querySelector("sseq-panel").querySelector(".name");
            focusElt.focus();
            sleep(50).then(() => focusElt.focus());
            return;
        }
        let classes = sseq.classes_in_bidegree(...new_bidegree);
        if(classes.length == 0){
            return;
        }
        select_bidegree(...new_bidegree);
        display.update();
    });

    let validationPromise;
    let validationPromiseResolve;
    async function validateName(name){
        validationPromise = new Promise(resolve => validationPromiseResolve = resolve);
        validationPromise.message = "interact.validate.name";
        socket_listener.send("interact.validate.name", {"name" : name});
        let result = await validationPromise;
        validationPromise = null;
        return result;
    }

    socket_listener.add_message_handler("interact.validate.name", async function(cmd, args, kwargs){
        if(!validationPromise || validationPromise.message !== "interact.validate.name"){
            throw Error(`Received unexpected "interact.validate.name"`);
        }
        validationPromiseResolve([kwargs.validated, kwargs.error]);
    });

    async function validateMatrix(bidegree, matrix){
        validationPromise = new Promise(resolve => validationPromiseResolve = resolve);
        validationPromise.message = "interact.validate.matrix";
        socket_listener.send("interact.validate.matrix", {"bidegree": bidegree, "matrix" : matrix});
        let result = await validationPromise;
        validationPromise = null;
        return result;
    }

    socket_listener.add_message_handler("interact.validate.matrix", async function(cmd, args, kwargs){
        if(!validationPromise || validationPromise.message !== "interact.validate.matrix"){
            throw Error(`Received unexpected "interact.validate.matrix"`);
        }
        validationPromiseResolve(kwargs);
    });


    let moving = false;
    async function select_bidegree(x, y){
        let sseq = display.querySelector("sseq-chart").sseq;
        selected_bidegree = [x, y];
        display.seek(x,y);
        let bidegree_highlighter = document.querySelector("sseq-bidegree-highlighter");
        let classes = sseq.classes_in_bidegree(...selected_bidegree);
        let class_highlighter = document.querySelector("sseq-class-highlighter");
        class_highlighter.clear();
        class_highlighter.highlight(classes);
        
        bidegree_highlighter.clear();
        bidegree_highlighter.highlight([selected_bidegree]);
        

            
            
        let bidegree_html = `<h4>Bidegree (${selected_bidegree.join(", ")})</h4>`;
        let class_html = "";
        let product_html = "";
        let matrix_html = "";
        let sidebar = document.querySelector("sseq-panel");
        sidebar.querySelector("#product-info-bidegree").innerHTML = bidegree_html;
        sidebar.querySelector("#product-info-classes").innerHTML = class_html;
        sidebar.querySelector("#product-info-products").innerHTML = product_html;
        sidebar.querySelector("#product-info-matrix").innerHTML = matrix_html;
        if(classes.length == 0){
            return;
        }
        await Promise.all([handleKeyDown.stoppedPromise, display.seek(x,y)]);
        socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
    }

    document.addEventListener("keydown", handleKeyDown);
    function handleKeyDown(e) {
        if(stopCallback(e, e.target || e.srcElement)){
            return;
        }
        if(e.code.startsWith("Arrow")){
            handleArrow(e);
        }
        if(e.code.startsWith("Digit")){
            handleDigit(e);
        }
        if(["+","-"].includes(e.key)){
            handlePM(e);
        }
    }

    let handleArrow = throttle(75, { trailing : false })(function handleArrow(e){
        if(!selected_bidegree){
            return;
        }
        let direction = e.code.slice("Arrow".length).toLowerCase();
        let dx = {"up" : 0, "down" : 0, "left" : -1, "right" : 1}[direction];
        let dy = {"up" : 1, "down" : -1, "left" : 0, "right" : 0}[direction];
        let [x, y] = selected_bidegree;
        x += dx;
        y += dy;
        let [minX, maxX] = display.xRange;
        let [minY, maxY] = display.yRange;
        x = Math.min(Math.max(x, minX), maxX);
        y = Math.min(Math.max(y, minY), maxY);

        select_bidegree(x, y);
    });
    

    let handlePM = throttle(75, { trailing : false })(function handleArrow(e){
        let d = {"+" : 1, "-" : -1}[e.key];
        let zoomCenter = undefined;
        if(selected_bidegree){
            let [x, y] = selected_bidegree;
            zoomCenter = [display.xScale(x), display.yScale(y)];
        }
        display.zoomBy(d, zoomCenter);
    });

    let handleDigit = throttle(75, { trailing : false })(function handleDigit(e) {
        let focusElt = document.querySelector(`sseq-panel [group='${e.key}'][tabindex='0']`);
        if(!focusElt){
            let panel_focuses = document.querySelectorAll(`sseq-panel [tabindex='0']`);
            if(panel_focuses){
                focusElt = panel_focuses[panel_focuses.length - 1];
            }
        }
        if(focusElt){
            focusElt.focus();
        }
    });

    Mousetrap.bind("z", () =>  {
        popup.cancel();
        socket_listener.send("interact.undo", {});
        if(selected_bidegree){
            socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
        }
    });
    Mousetrap.bind("Z", () => {
        popup.cancel();
        socket_listener.send("interact.redo", {});
        if(selected_bidegree){
            socket_listener.send("interact.select_bidegree", {"bidegree" : selected_bidegree});
        }
    });

    socket_listener.start();
}
