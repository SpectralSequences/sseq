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

import {Mutex, Semaphore, withTimeout} from 'async-mutex';

import { Panel } from "chart/interface/Panel.js";
import { Matrix } from "chart/interface/Matrix.js";
import { KatexExprElement } from "chart/interface/KatexExprElement.js";
import { SseqSocketListener } from "chart/SseqSocketListener.js";
import { Popup } from "chart/interface/Popup.js";
import { sleep, promiseFromDomEvent, throttle, animationFrame } from "chart/interface/utils.js";

window.SseqSocketListener = SseqSocketListener;
window.UIElement = UIElement;

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


function setNameCommand(bidegree, res_basis_vec,  our_basis_vec,  name){
    return {
        type : "set_name",
        bidegree : bidegree,  
        vec : res_basis_vec,
        our_basis_vec : our_basis_vec,
        name : name
    };
}

function setMatrixCommand(bidegree, matrix, changedRows){
    return {
        type : "set_matrix",
        bidegree : bidegree,  
        matrix : matrix,
        changedRows : changedRows
    };
}

class TableUI {
    constructor(uiElement, socket_address){
        this.ws = new ReconnectingWebSocket(socket_address, [], 
            {
                /** debug : true, /**/
                minReconnectionDelay: 100,
                maxReconnectionDelay: 1000,
            }
        );
        this.uiElement = uiElement;
        this.display = uiElement.querySelector("sseq-display")
        this.socket_listener = new SseqSocketListener(this.ws);
        this.socket_listener.attachDisplay(this.display);
        this.popup = uiElement.querySelector("sseq-popup");
        this.sidebar = uiElement.querySelector("sseq-panel");
        this.undoMutex = withTimeout(new Mutex(), 100);
        window.addEventListener("beforeunload", (e) => { 
            this.popup.cancel();
        });
    }

    async start(){
        this.setupUIBindings();
        this.setupSocketMessageBindings();        
        this.socket_listener.start();
        await promiseFromDomEvent(this.uiElement, "started");
        let display_rect = this.uiElement.querySelector("sseq-display").getBoundingClientRect();
        this.popup.left = display_rect.width/2 - 250/2;
    }

    setupSocketMessageBindings(){
        this.socket_listener.add_promise_message_handler("interact.product_info");
        this.socket_listener.add_promise_message_handler("interact.validate.name");
        this.socket_listener.add_promise_message_handler("interact.validate.matrix");
        this.socket_listener.add_promise_message_handler("interact.action_info");
    }

    async updateProductInfo(){
        if(!this.selected_bidegree){
            return;
        }
        let [curX, curY] = this.selected_bidegree;
        this.socket_listener.send("interact.select_bidegree", {"bidegree" : this.selected_bidegree});
        let [_cmd, _args, kwargs] = await this.socket_listener.new_message_promise("interact.product_info");
        let sseq = this.uiElement.querySelector("sseq-chart");
        this.namedVecs = namedVecsListToObj(kwargs.named_vecs);
        this.names = [];
        this.nameMonos = [];
        for(let [name, mono] of kwargs.names){
            this.names.push(name);
            this.nameMonos.push(mono);
        }
        let selectedIndex = undefined;
        if(document.activeElement && document.activeElement.closest("sseq-panel")){
            selectedIndex = this.sidebar.querySelectorAll("[tabindex='0']");
            selectedIndex = selectedIndex && Array.from(selectedIndex);
            selectedIndex = selectedIndex && selectedIndex.indexOf(document.activeElement);
        }
        let name_group = 1;
        let group = name_group;
        this.product_info = kwargs.product_info;
        this.matrix = kwargs.matrix;
        let processed_product_info = [];
        let out_vecs = {};
        for(let { left : [in1,name1, ], right : [in2, name2, ], out_name, out_res_basis} of this.product_info){
            let name_str = "";
            if(out_name){
                name_str = `{}= ${out_name}`
            }
            if(!(JSON.stringify(out_res_basis) in out_vecs)){
                group ++;
                out_vecs[JSON.stringify(out_res_basis)] = 1;
            }
            processed_product_info.push(
                [`${name1} \\cdot ${name2} = ${JSON.stringify(out_res_basis)}`, name_str, group]
            );
        }
        let matrix_group = group + 1;
        let bidegree_html = `<h4>Bidegree (${this.selected_bidegree.join(", ")})</h4>`;
        let class_html = "";
        let product_html = "";
        let matrix_html = "";
        class_html = `
            <h5>Classes</h5>
            <p style="align-self: center;">
                ${this.names.map(e => `<katex-expr class="name" group="${name_group}">${e}</katex-expr>`)
                        .join(`, <span style="padding-right:6pt; display:inline-block;"></span>`)}
            </p>
        `;        
        if(processed_product_info.length > 0){
            product_html = `
                <h5 style="">
                    Products
                </h5>
                <div class="product-list" style="align-self: center; width: max-content; overflow: hidden;">
                    <table><tbody>
                        ${processed_product_info.map(([e, n, group]) => `
                            <tr class="product-item" group="${group}" ${!n ? "focus" : ""}>
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
        this.sidebar.querySelector("#product-info-bidegree").innerHTML = bidegree_html;
        this.sidebar.querySelector("#product-info-classes").innerHTML = class_html;
        this.sidebar.querySelector("#product-info-products").innerHTML = product_html;
        this.sidebar.querySelector("#product-info-matrix").innerHTML = matrix_html;
        let matrixElt = this.sidebar.querySelector("sseq-matrix");
        matrixElt.value = this.matrix;
        matrixElt.labels = this.names.map((n, idx) => this.nameMonos[idx] ? n : "");
        matrixElt.tabIndex = '0';
        matrixElt.addEventListener("click",  () => {
            this.handleMatrixEditClick();
        });
        this.sidebar.displayChildren("#product-info");

        this.sidebar.querySelectorAll(".name").forEach((e, idx) => {
            e.tabIndex = 0;
            e.addEventListener("click",  () => {
                this.handleNameItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });
        Array.from(this.sidebar.querySelectorAll(".product-item"))
             .map((e,idx) => [e,idx])
             .filter((_, idx) => 
             this.product_info[idx].left[2] && this.product_info[idx].left[2].length > 0
                && this.product_info[idx].right[2] && this.product_info[idx].right[2].length > 0
            )
        .forEach(([e, idx]) => {
            e.tabIndex = 0;
            e.addEventListener("click",  () => {
                this.handleProductItemClick(idx);
                // socket_listener.send("interact.click_product", {"bidegree" : sseq._selected_bidegree, "idx" : idx});
            });
        });

        await sleep(0);
        let [selectedX, selectedY] = this.selected_bidegree;
        if(selectedX !== curX || selectedY !== curY){
            return;
        }
        if(!this.sidebar.querySelector("[tabindex='0'][focus]")){
            let nameElts = this.sidebar.querySelectorAll(".name");
            this.nameMonos.forEach((n, idx) => {
                if(!n){
                    nameElts[idx].setAttribute("focus", "");
                }
            });
        }
        if(!this.sidebar.querySelector("[tabindex='0'][focus]")){
            this.sidebar.querySelector("[tabindex='0']").setAttribute("focus", "");
        }

        await this.updateFocus(selectedIndex);

        // div.style.display = "flex";
        // div.style.flexDirection = "column";
        // div.style.height = "90%";
    }

    async updateFocus(selectedIndex){
        if(selectedIndex){
            let focusElt = this.sidebar.querySelectorAll("[tabindex='0']")[selectedIndex];
            focusElt.focus();
            await sleep(50);
            focusElt.focus();
        } else {
            this.sidebar.focus();
            await sleep(50);
            this.sidebar.focus();
        }
    }

    async setError(type, message){
        if(type !== undefined){
            let input = this.popup.querySelector("input");
            if(input){
                input.setAttribute("transition", "show");
                input.setCustomValidity(message);
            }
            let errorElt = this.popup.querySelector(".error");
            errorElt.setAttribute("transition", "show");
            errorElt.classList.add("active");
            errorElt.error_type = type;
            errorElt.innerHTML = message;
        } else {
            clearError();
        }
    }

    clearError(type){
        let input = this.popup.querySelector("input");
        let errorElt = this.popup.querySelector(".error");
        if(type !== undefined && errorElt.error_type !== type){
            return;
        }
        errorElt.setAttribute("transition", "hide");
        if(input){
            input.setAttribute("transition", "hide");
            input.setCustomValidity("");
        }
        errorElt.classList.remove("active");
    }

    async handleNameItemClick(item_idx){
        let selected_bidegree = this.selected_bidegree;
        let popup_header = this.popup.querySelector("[slot=header]");
        let popup_body = this.popup.querySelector("[slot=body]");
        let hasName = this.nameMonos[item_idx] !== null;
        let name = this.names[item_idx];
        let nameWord = hasName ? "Rename" : "Name";
        popup_header.innerText = `${nameWord} class?`;
        let tuple = [...this.selected_bidegree, item_idx];
        popup_body.innerHTML =`   
            Input ${hasName ? "new " : ""}name for class (${tuple.join(", ")}):
            <input type="text" focus style="width : 100%; margin-top : 0.6rem;">
            <span class="error"></span>
        `;
        let input = popup_body.querySelector("input");
        input.addEventListener("focus", () => {
            input.select();
            this.clearError("UnexpectedEOF");
        });
        input.addEventListener("input", async () => {
            let {validated, error} = await this.validateName(input.value);
            this.popup.okEnabled = validated;
            validated = validated || error.name === "UnexpectedEOF";
            if(validated){
                this.clearError();
            } else {
                let input_value = input.value;
                await sleep(1000);
                if(input.value !== input_value){
                    return;
                }
                this.setError(error.name, `${error.name} column: ${error.column}`);
            }
        });
        input.addEventListener("blur", async () => {
            let {validated, error} = await this.validateName(input.value);
            this.popup.okEnabled = validated;
            if(validated){
                this.clearError();
            } else {
                this.setError(error.name, error.name);
            }
        });
        if(hasName){
            input.value = name;
        }
        this.popup.show();
        let ok_q = await this.popup.submited();
        if(!ok_q){
            return;
        }
        if(hasName && input.value === name || !hasName && input.value === ""){
            return;
        }
        let vec = this.matrix[item_idx];
        let our_basis_vec = Array(this.matrix.length).fill(0);
        our_basis_vec[item_idx] = 1;
        let named_class = [...selected_bidegree, item_idx];
        let description;
        if(input.value !== ""){
            description = `Named class (${named_class.join(", ")}) as <katex-expr>${input.value}</katex-expr>.`;
        } else {
            description = `Removed the name ${name} of class (${named_class.join(", ")})`;
        }
        this.socket_listener.send("interact.action", {
            action : { 
                root_bidegree : selected_bidegree,
                cmd_list : [setNameCommand(selected_bidegree, vec, our_basis_vec, input.value)] ,
                description : description
            }
        });
        this.updateProductInfo();
        this.waitForActionInfoAndDisplayIt("Completed action:");
    }

    async handleProductItemClick(item_idx){
        let sseq = display.querySelector("sseq-chart").sseq;
        let jsoned_matrix = this.matrix.map(JSON.stringify);
        let product_data = this.product_info[item_idx];
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
        let class_highlighter = this.uiElement.querySelector("sseq-class-highlighter");
        class_highlighter.clear();
        class_highlighter.fire(highlightClasses, 0.8);
        // .then(() => {
        //     class_highlighter.hideClasses(highlightClasses);
        // });
        if(inbasis){
            let out_tuple = [...out_deg, one_entries[0]];
            await this.handleProductItemClick_inBasis(product_data, out_tuple );
        } else {
            await this.handleProductItemClick_changeBasis(product_data, out_our_basis);
        }
    }

    async handleProductItemClick_inBasis(product_data, out_tuple){
        let { left : [, name1,], right : [, name2,], out_name, out_res_basis, out_our_basis } = product_data;
        let selected_bidegree = this.selected_bidegree;
        this.popup.okEnabled = true;
        let popup_header = this.popup.querySelector("[slot=header]");
        let popup_body = this.popup.querySelector("[slot=body]");
        let nameWord = out_name ? "Rename" : "Name";
        popup_header.innerText = `${nameWord} class?`;
        popup_body.innerHTML = `
            <p>${nameWord} class (${out_tuple.join(", ")}) as <katex-expr>${name1}\\cdot ${name2}</katex-expr>?</p>
            ${out_name ? `<p>Current name is <katex-expr>${out_name}</katex-expr>.</p>` : ``}
        `;
        this.popup.show();
        let ok_q = await this.popup.submited();
        if(!ok_q){
            return;
        }
        this.socket_listener.send("interact.action", {
            action : {
                root_bidegree : selected_bidegree,
                cmd_list : [setNameCommand(selected_bidegree, out_res_basis, out_our_basis, `${name1} ${name2}`)],
                description : `Named class (${out_tuple.join(", ")}) as the product "<katex-expr>${name1}\\cdot ${name2}</katex-expr>."`
            }
        });    
        this.updateProductInfo();
        this.waitForActionInfoAndDisplayIt("Completed action:");
    }
    
    async handleProductItemClick_changeBasis(product_data, out_our_basis){
        this.popup.okEnabled = false;
        let selected_bidegree = this.selected_bidegree;
        let {left : [, name1, ], right :[, name2, ], out_res_basis} = product_data;
        let popup_header = this.popup.querySelector("[slot=header]");
        let popup_body = this.popup.querySelector("[slot=body]");
        popup_header.innerText = "Update basis?";
        let new_body = document.createElement("div");
        new_body.innerHTML = `
            Select a basis vector in bidegree (${selected_bidegree.join(",")}) to replace with 
            <p><katex-expr>${name1} \\cdot ${name2} = ${JSON.stringify(out_res_basis)}</katex-expr>:</p>
            <p><sseq-matrix focus type=select-row></sseq-matrix></p>
        `;
        await sleep(0); // Allow matrix to render
        popup_body.innerHTML = "";
        popup_body.appendChild(new_body);
        let matrix_elt = new_body.querySelector("sseq-matrix");
        matrix_elt.value = this.matrix;
        matrix_elt.labels = this.names;
        matrix_elt.enabledRows = out_our_basis.map(e => e !== 0);
        this.popup.okEnabled = false;
        matrix_elt.addEventListener("matrix-select", (e) => {
            this.popup.okEnabled = matrix_elt.selectedRows.length > 0;
            if(matrix_elt.selectedRows.length > 0){
                let matrix_clone = this.matrix.map(r => r.slice());
                let replace_row = matrix_elt.selectedRows[0];
                matrix_clone[replace_row] = out_res_basis;
                this.validateMatrix(selected_bidegree, matrix_clone);
            } else {
                this.socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            }
        });
        // TODO: disable rows of matrix not set in one_entries.
        this.popup.show();
        let ok_q = await this.popup.submited();
        if(!ok_q){
            this.socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            return;
        }
        let result_matrix = matrix_elt.value;
        let replace_row = matrix_elt.selectedRows[0];
        result_matrix[replace_row] = out_res_basis;
        let updated_basis_vec = Array(result_matrix.length).fill(0);
        updated_basis_vec[replace_row] = 1;
        let class_index = [...selected_bidegree, replace_row];
        this.socket_listener.send("interact.action", {
            action : {
                root_bidegree : selected_bidegree,
                cmd_list : [
                    setNameCommand(selected_bidegree, out_res_basis, updated_basis_vec, `${name1} ${name2}`),
                    setMatrixCommand(selected_bidegree, result_matrix, [replace_row])
                ],
                description : `Replaced the basis vector for (${class_index.join(", ")}) with the product "<katex-expr>${name1}\\cdot ${name2}</katex-expr>."`
            }
        });
        this.updateProductInfo();
        this.waitForActionInfoAndDisplayIt("Completed action:");
    }

    async handleMatrixEditClick(){
        let popup_header = this.popup.querySelector("[slot=header]");
        let popup_body = this.popup.querySelector("[slot=body]");        
        popup_header.innerText = "Update basis?";
        let selected_bidegree = this.selected_bidegree;
        let new_body = document.createElement("div");
        new_body.innerHTML = `
            <p> Input new basis for bidegree (${selected_bidegree.join(",")}):
            <p><sseq-matrix focus type=input></sseq-matrix></p>
            <div class="error" style="width:fit-content; padding : 5px; padding-right : 8px;">Matrix is singular</div>
        `;
        await sleep(0); // Allow matrix to render
        popup_body.innerHTML = "";
        popup_body.appendChild(new_body);
        let matrix_elt = new_body.querySelector("sseq-matrix");
        matrix_elt.value = this.matrix.map(e => e.slice());
        matrix_elt.labels = this.names;
        matrix_elt.addEventListener("change", async (e) => {
            let { singular, row_labels } = await this.validateMatrix(selected_bidegree, matrix_elt.value);
            this.popup.okEnabled = !singular;
            matrix_elt.labels = row_labels;
            if(JSON.stringify(matrix_elt.value) === JSON.stringify(this.matrix)){
                this.socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            }
            let errorElt = this.popup.querySelector(".error");
            if(singular){
                let cur_matrix = JSON.stringify(matrix_elt.value);
                await sleep(1000);
                if(cur_matrix === JSON.stringify(matrix_elt.value)){
                    errorElt.setAttribute("transition", "show");
                    errorElt.classList.add("active");
                }
            } else {
                errorElt.setAttribute("transition", "hide");
                errorElt.classList.remove("active");
            }
        });
        // matrix_elt.addEventListener("blur", (e) => console.log("blurred", e));
        this.popup.show();
        let ok_q = await this.popup.submited();
        if(!ok_q){
            this.socket_listener.send("interact.revert_preview", { bidegree : selected_bidegree });
            return;
        }
        let changedRows = [];
        let originalRows = this.matrix.map((r) => r.join(","));
        let newRows = matrix_elt.value.map((r) => r.join(","));
        for(let r = 0; r < newRows.length; r++){
            if(newRows[r] !== originalRows[r]){
                changedRows.push(r);
            }
        }
        this.socket_listener.send("interact.action", {
            action : {
                root_bidegree : selected_bidegree,
                cmd_list : [ setMatrixCommand(selected_bidegree, matrix_elt.value, changedRows) ],
                description : `Changed the basis in bidegree (${selected_bidegree.join(", ")})`
            }
        });        
        this.updateProductInfo();
        this.waitForActionInfoAndDisplayIt("Did:");
    }

    handleChartClick(e){
        let sseq = display.querySelector("sseq-chart").sseq;
        let new_bidegree = e.detail[0].mouseover_bidegree;
        if(!new_bidegree){
            return;
        }
        let classes = sseq.classes_in_bidegree(...new_bidegree);
        if(classes.length == 0){
            return;
        }
        this.select_bidegree(...new_bidegree);
        display.update();
    }

    async validateName(name){
        this.socket_listener.send("interact.validate.name", {"name" : name});
        let result = await this.socket_listener.new_message_promise("interact.validate.name");
        return result[2];
    }

    async validateMatrix(bidegree, matrix){
        this.socket_listener.send("interact.validate.matrix", {"bidegree": bidegree, "matrix" : matrix});
        let result = await this.socket_listener.new_message_promise("interact.validate.matrix");
        return result[2];
    }

    async select_bidegree(x, y){
        let sseq = this.uiElement.querySelector("sseq-chart").sseq;
        this.selected_bidegree = [x, y];
        // this.popup.cancel();
        let bidegree_highlighter = this.uiElement.querySelector("sseq-bidegree-highlighter");
        let classes = sseq.classes_in_bidegree(...this.selected_bidegree);
        let class_highlighter = this.uiElement.querySelector("sseq-class-highlighter");
        class_highlighter.clear();
        class_highlighter.highlight(classes);
        
        bidegree_highlighter.clear();
        bidegree_highlighter.highlight([this.selected_bidegree]);
            
        let bidegree_html = `<h4>Bidegree (${this.selected_bidegree.join(", ")})</h4>`;
        let class_html = "";
        let product_html = "";
        let matrix_html = "";
        this.sidebar.querySelector("#product-info-bidegree").innerHTML = bidegree_html;
        this.sidebar.querySelector("#product-info-classes").innerHTML = class_html;
        this.sidebar.querySelector("#product-info-products").innerHTML = product_html;
        this.sidebar.querySelector("#product-info-matrix").innerHTML = matrix_html;
        await Promise.all([display.seek(x,y)]); //handleKeyDown.stoppedPromise, 
        if(classes.length == 0){
            this.uiElement.focus();
            return;
        }
        await this.updateProductInfo();
    }

    async showHelpWindow() {
        this.resizeHelpWindow();
        let help_popup = this.uiElement.querySelector(".help");        
        help_popup.show();
        help_popup.focus();
    }

    resizeHelpWindow(){
        let help_popup = this.uiElement.querySelector(".help");
        let display_rect = this.uiElement.querySelector("sseq-display").getBoundingClientRect();
        help_popup.left = 0.2  * display_rect.width;
        help_popup.top = 0.1 * display_rect.height;
        help_popup.width = `${0.6 * display_rect.width}px`;
        help_popup.height = "70vh";//`${0.6 * display_rect.height}px`;
    }


    setupUIBindings(){
        this.uiElement.mousetrap.bind("t", () => {
            this.socket_listener.send("console.take", {});
        });

        this.uiElement.mousetrap.bind("z", () => {
            this.undo();
        });
    
        this.uiElement.mousetrap.bind("Z", () => {
            this.redo();
        });
        this.uiElement.mousetrap.bind("h", this.showHelpWindow.bind(this));
        this.uiElement.querySelector(".help-btn").addEventListener("click", this.showHelpWindow.bind(this))
        let resizeObserver = new ResizeObserver(entries => {
            this.resizeHelpWindow();
        });
        resizeObserver.observe(this.uiElement);

        this.uiElement.mousetrap.bind("home", async () => {
            this.select_bidegree(0, 0);
        });
        this.uiElement.mousetrap.bind("n", async () => {
            let curClass;
            for(let c of this.sortedClasses()){
                if(
                    !c.monomial_name && c.indec
                    && (
                        !this.selected_bidegree 
                        || c.x > this.selected_bidegree[0] 
                        || c.x == this.selected_bidegree[0] && c.y > this.selected_bidegree[1]
                    )
                ){
                    curClass = c;
                    break;
                }
            }
            if(curClass){
                await this.select_bidegree(curClass.x, curClass.y);
                let [selectedX, selectedY] = this.selected_bidegree;
                if(selectedX === curClass.x && selectedY === curClass.y){
                    this.sidebar.querySelectorAll(".name")[curClass.idx].focus();
                }
            }
        });

        this.uiElement.mousetrap.bind("m", () => {
            let curClass;
            for(let c of this.sortedClasses()){
                if(
                    !c.monomial_name && c.hi_indec && !c.indec
                    && (
                        !this.selected_bidegree 
                        || c.x > this.selected_bidegree[0] 
                        || c.x == this.selected_bidegree[0] && c.y > this.selected_bidegree[1]
                    )
                ){
                    curClass = c;
                    break;
                }
            }
            if(curClass){
                this.select_bidegree(curClass.x, curClass.y);
            }
        });        
        
        this.sidebar.addEventListener("interact-toggle", () => {
            document.activeElement.click();
        });
        this.sidebar.addEventListener("interact-submit", () => {
            document.activeElement.click();
        });
        this.display.addEventListener("click", this.handleChartClick.bind(this))
        this.uiElement.addEventListener("keydown-arrow",
            throttle(75, { trailing : false })(this.handleArrow.bind(this)));
        this.uiElement.addEventListener("keypress-wasd", throttle(5)(this.handleWASD.bind(this)));
        this.uiElement.addEventListener("keypress-pm",
            throttle(150, { trailing : false })(this.handlePM.bind(this)));
        this.uiElement.addEventListener("keypress-digit",
            throttle(150, { trailing : false })(this.handleDigit.bind(this)));
        
    }

    handleArrow(e){
        if(!this.selected_bidegree){
            return;
        }
        let [x, y] = this.selected_bidegree;
        let [dx, dy] = e.detail.direction;
        x += dx;
        y += dy;
        let [minX, maxX] = display.xRange;
        let [minY, maxY] = display.yRange;
        x = Math.min(Math.max(x, minX), maxX);
        y = Math.min(Math.max(y, minY), maxY);

        this.select_bidegree(x, y);
    }
    
    async handleWASD(e){
        await animationFrame();
        let [dx, dy] = e.detail.direction;
        let s = 20;
        display.translateBy( - dx * s, dy * s);
    }

    handlePM(e){
        let d = e.detail.direction;
        let zoomCenter = undefined;
        if(this.selected_bidegree){
            let [x, y] = this.selected_bidegree;
            zoomCenter = [display.xScale(x), display.yScale(y)];
        }
        display.zoomBy(d, zoomCenter);
    }

    handleDigit(e) {
        let digit = e.detail.digit;
        if(digit === 0){
            this.popup.focus();
            e.detail.originalEvent.preventDefault();
            return;
        }
        let focusElt;
        for(let n = digit; n < 10; n++){  
            focusElt = this.sidebar.querySelector(`[group='${n}'][tabindex='0']`);
            if(focusElt){
                break;
            }
        }
        if(!focusElt){
            let panel_focuses = this.sidebar.querySelectorAll(`[tabindex='0']`);
            if(panel_focuses){
                focusElt = panel_focuses[panel_focuses.length - 2];
            }
        }
        if(focusElt){
            focusElt.focus();
        }
    }

    async undo() {
        this.undoMutex.runExclusive(async () => {
            this.popup.cancel();
            this.socket_listener.send("interact.undo", {});
            this.updateProductInfo();
            await Promise.all([this.waitForActionInfoAndDisplayIt("Undid action:"), sleep(500)]);
        }).catch(e => {
            if(e.message === "timeout"){
                // console.log("undo timed out");
                return;
            }            
            throw e;
        });
    }

    async redo() {
        await this.undoMutex.runExclusive(async () => {
            this.popup.cancel();
            this.socket_listener.send("interact.redo", {});
            this.updateProductInfo();
            await Promise.all([this.waitForActionInfoAndDisplayIt("Redid action:"), sleep(500)]);
        }).catch(e => {
            if(e.message === "timeout"){
                // console.log("Redo timed out");
                return;
            }            
            throw e;
        });
    }    
    

    async waitForActionInfoAndDisplayIt(action_type){
        let [_cmd, _args, kwargs] = await this.socket_listener.new_message_promise("interact.action_info");
        await this.displayActionInfo(action_type, kwargs.action);
        return true;
    }

    async displayActionInfo(ty, action) {
        if(action === null){
            return;
        }
        let updateID = Math.random();
        let status = this.uiElement.querySelector(".status-indicator");
        let highlightClasses = this.getHighlightClasses(action);
        let class_highlighter = this.uiElement.querySelector("sseq-class-highlighter");
        await class_highlighter.clear();
        class_highlighter.fire(highlightClasses).then(() => {
            class_highlighter.hideClasses(highlightClasses);
        });
        status.updateID = updateID;
        status.innerHTML = `${ty} ${this.getActionDescription(action)}`;
        status.setAttribute("transition", "show");
        status.setAttribute("shown", "");
        sleep(2000).then( () => {
            if(status.updateID === updateID){            
                status.setAttribute("transition", "hide");
                status.removeAttribute("shown", "");
            }
        });
    }

    getActionDescription(action){
        return action.description;
    }

    getHighlightClasses(action){
        let highlightClasses = {};
        for(let cmd of action.cmd_list){
            let [x, y] = cmd.bidegree;
            switch(cmd.type) {
                case "set_name":
                    cmd.our_basis_vec.forEach((v, idx) => {
                        if(v !== 0){
                            let class_index = [x, y, idx];
                            highlightClasses[class_index.join(",")] = class_index;
                        }
                    });
                    break;

                case "set_matrix":
                    cmd.changedRows.forEach((idx) => {
                        let class_index = [x, y, idx];
                        highlightClasses[class_index.join(",")] = class_index;
                    });
                    break;

                default:
                    throw Error(`Unknown command type ${cmd.type}`);
                    break;
            }
        }
        let sseq = display.querySelector("sseq-chart").sseq;
        return Object.values(highlightClasses).map( idx => sseq.class_by_index(...idx));
    }

    sortedClasses(){
        return Object.values(
            this.uiElement.querySelector("sseq-chart").sseq.classes
        )
        .sort((a,b) => (a.x - b.x)*10 + Math.sign(a.y - b.y));
    }
}

window.TableUI = TableUI;
