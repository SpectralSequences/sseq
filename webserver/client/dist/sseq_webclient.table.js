/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		var threw = true;
/******/ 		try {
/******/ 			modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/ 			threw = false;
/******/ 		} finally {
/******/ 			if(threw) delete installedModules[moduleId];
/******/ 		}
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, {
/******/ 				configurable: false,
/******/ 				enumerable: true,
/******/ 				get: getter
/******/ 			});
/******/ 		}
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = 9);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* unused harmony export ensureMath */
/* harmony export (immutable) */ __webpack_exports__["a"] = renderLatex;
/* harmony export (immutable) */ __webpack_exports__["b"] = renderMath;
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_katex__ = __webpack_require__(7);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_katex___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_0_katex__);


function ensureMath(str){
    if(str.startsWith("\\(") || str.startsWith("$")){
        return str;
    }
    if(!str){
        return "";
    }
    return "$" + str + "$";
}

function renderLatex(html) {
    html = html.replace(/\n/g, "\n<hr>\n")
    let html_list = html.split(/(?:\\\[)|(?:\\\()|(?:\\\))|(?:\\\])|(?:\$)/);
    for(let i = 1; i < html_list.length; i+=2){
        html_list[i] = Object(__WEBPACK_IMPORTED_MODULE_0_katex__["renderToString"])(html_list[i]);
    }
    return html_list.join("\n")
}
function renderMath(x) {
    return renderLatex(ensureMath(x));
} 

/***/ }),
/* 1 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return _Panel; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_events__ = __webpack_require__(6);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_events___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_0_events__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1_mousetrap__ = __webpack_require__(5);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1_mousetrap___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_1_mousetrap__);






/**
 * A panel is a collection of objects (button etc.) to be displayed in a
 * sidepanel. The main function to implement is show(), which is called
 * whenever the panel is to be displayed.
 *
 * The standard way to deal with panels is that all children of the panel are
 * created when the panel is initialized, and all callbacks are appropriately
 * set up. When show() is called, we decide which elements to display by
 * setting the display property, and then initialize the values of the elements
 * accordingly.
 *
 * Panels and its children are expected to properly track mutations and write
 * them to this.display.sseq.undo upon each change.
 *
 * There are a few helper functions that add elements to the panel, such as
 * addButton.
 *
 * @property {Node} container - Top node of the panel, to which we add all
 * children. This is a plain div element that is not styled. All styling should
 * be applied to children of this container.
 * @property {Node} currentGroup - This is the DOM element that the helper
 * functions will add the buttons/fields to. This defaults to this.container
 * but is modified by newGroup() and endGroup(). It can also be manually
 * modified as desired.
 *
 * @fires Panel#show
 * @extends EventEmitter
 */
class Panel extends __WEBPACK_IMPORTED_MODULE_0_events___default.a {
    /**
     * Constructs a panel.
     *
     * @param {Node} parentContainer - The node to add the panel to
     * @param {Display:Display} - The Display object the panel is about.
     * This is used by the helper functions to know where to track mutations,
     * update the display when properties change, etc.
     */
    constructor (parentContainer, display) {
        super();

        this.display = display;
        this.container = document.createElement("div");
        parentContainer.appendChild(this.container);
        this.links = [];

        this.currentGroup = this.container;
    }

    /**
     * This hides the panel. It does nothing but set the display property to
     * none.
     */
    hide() {
        this.container.style.display = "none";
    }

    /**
     * This clears everything in the panel. This currently does not unbind the
     * shortcuts.
     */
    clear() {
        while (this.container.firstChild){
            this.container.removeChild(this.container.firstChild);
        }

        this.links = [];
    }

    /**
     * This shows the panel, and populates the values of the children.  This
     * correctly populates the children added by the helper functions, and no
     * extra work has to be done for them. If custom children are added, one
     * will want to customize the show() function to ensure the children are
     * correctly displayed. This can be done by overwriting the show() function
     * or by listening to the Panel#show event.
     *
     * This function may be called when the panel is already shown. In this
     * case, the correct behaviour is to refresh the display (e.g. update the
     * values of the fields)
     */
    show() {
        this.container.style.removeProperty("display");

        for (let link of this.links) {
            let t = this.display;
            for (let attr of link[0].split(".")) {
                t = t[attr];
                if (t === undefined || t === null) {
                    return;
                }
            }
            link[1].value = t;
        }
        /**
         * Show event. This is emitted when show() is called. One may opt to
         * listen and respond to the show event instead of overwriting show()
         * when designing custom panels, c.f. DifferentialPanel.
         *
         * @event Panel#show
         */
        this.emit("show");
    }

    /**
     * This creates a new div and adds it to the container. This new div is
     * then set as currentGroup and has class card-body.
     *
     * This should be used if one wishes to add a collection of children that
     * are to be grouped together. The procedure for using this is as follows:
     * (1) Run Panel#newGroup
     * (2) Add the children using the helper functions (addButton, addObject, etc.)
     * (3) Run Panel#endGroup to set currentGroup back to this.container.
     */
    newGroup() {
        this.currentGroup = document.createElement("div");
        this.currentGroup.className = "card-body";
        this.container.appendChild(this.currentGroup);
    }
    /**
     * See newGroup().
     */
    endGroup() {
        this.currentGroup = this.container;
    }

    /**
     * Does nothing but this.currentGroup.appendChild(obj);
     *
     * @param {Node} obj - The object to be added.
     */
    addObject(obj) {
        this.currentGroup.appendChild(obj);
    }

    /**
     * This adds a button to currentGroup.
     *
     * @param {string} text - Text to appear on the button.
     * @param {function} callback - Function to call when button is clicked.
     * @param {Object} extra - Extra (optional) properties to supply.
     * @param {string} extra.tooltip - Tooltip text to display
     * @param {string[]} shortcuts - A list of shortcuts that will be bound to callback
     */
    addButton(text, callback, extra = {}) {
        let o = document.createElement("button");
        if (extra.style)
            o.className = `btn btn-${extra.style} mb-2`;
        else
            o.className = "btn btn-primary mb-2";

        o.style.width = "100%";
        o.innerHTML = text;
        o.addEventListener("click", callback);

        if (extra.tooltip){
            o.setAttribute("title", extra.tooltip);
        }
        
        if (extra.shortcuts){
            for (let k of extra.shortcuts){
                __WEBPACK_IMPORTED_MODULE_1_mousetrap__["bind"](k, callback);
            }
        }

        this.currentGroup.appendChild(o);
    }

    /**
     * This adds several buttons placed side-by-side on a row.
     *
     * @param {Array[]} buttons - An array of arguments specifying the buttons
     * to be added. Each entry in the array should itself be an array, which
     * consists of the arguments to Panel#addButton for the corresponding
     * button.
     */
    addButtonRow(buttons){
        let group = this.currentGroup;
        let o = document.createElement("div");
        o.className = "form-row";
        for (let button of buttons) {
            let c = document.createElement("div");
            c.className = "col";
            this.currentGroup = c;
            this.addButton(...button);
            o.appendChild(c);
        }
        this.currentGroup = group;
        this.currentGroup.appendChild(o);
    }

    /**
     * This adds a header.
     * @param {String} header - The header text.
     */
    addHeader(header) {
        let node = document.createElement("h5");
        node.className = "card-title";
        node.innerHTML = header;
        this.addObject(node);
    }

    /**
     * This adds a linked input. A linked input is an entry that looks like
     *
     *       +-----+
     * Label |     |
     *       +-----+
     *
     * The input field is linked to a certain property of display. When the
     * panel is shown, the initial value of the input field is set to the value
     * of the corresponding property, and when the input field is changed, the
     * property is changed accordingly.
     *
     * @param {string} label - The label displayed next to the input field
     * @param {string} target - The property the input field is linked to.
     * This is specified by a string of the from "foo.bar.xyz", which says the
     * field is linked to this.display.foo.bar.xyz.
     * @param {string} type - The type of the input field. This is "text" or
     * "number" would usually be sensible choices.
     * @param {Object=} mementoObject - By default, the undo/redo functions
     * will simply set the value of target to what it was. Here the target
     * is remembered as an *object*, not as a property of this.display via
     * target (for example, if the input is about the currently active node
     * (this.display.selected), the undo function should undo the change on the
     * node that was affected, not the node that is active when the undo button
     * is pressed). It turns out this is problematic when dealing with nodes of
     * classes, since when classes are restored via undo/redo, the set of nodes
     * is copied and all references are lost.
     *
     * If mementoObject is defined, then instead of tracking individual changes
     * of the properties, the mutation tracker remembers the previous and after
     * states of mementoObject and writes that into the undo stack instead.
     * c.f. the node color/size inputs in EditorDisplay.
     */
    addLinkedInput(label, target, type, mementoObject) {
        let o = document.createElement("div");
        o.className = "form-row mb-2";
        o.style.width = "100%";
        this.currentGroup.appendChild(o);

        let l = document.createElement("label");
        l.className = "col-form-label mr-sm-2";
        l.innerHTML = label;
        o.appendChild(l);

        let i = document.createElement("input");
        i.style["flex-grow"] = 1;
        i.setAttribute("type", type);
        o.appendChild(i);

        switch (type) {
            case "text":
                i.setAttribute("size", "1");
                break;
            default:
                i.style.width = "1px";
                break;
        }

        this.links.push([target, i]);

        i.addEventListener("change", (e) => {
            let target_pre;
            if (mementoObject) {
                mementoObject = Panel.unwrapProperty(this.display, mementoObject.split("."))
                target_pre = mementoObject.getMemento();
            }

            let l = target.split(".");
            let prop = l.pop();
            let t = Panel.unwrapProperty(this.display, l);

            let old_val = t[prop];
            let new_val = e.target.value;
            t[prop] = new_val;

            if (this.display.sseq.undo) {
                if (mementoObject) {
                    this.display.sseq.undo.startMutationTracking()
                    this.display.sseq.undo.addMutation(mementoObject, target_pre, mementoObject.getMemento())
                    this.display.sseq.undo.addMutationsToUndoStack();
                } else {
                    this.display.sseq.undo.addValueChange(t, prop, old_val, new_val, () => this.display.sidebar.showPanel());
                }
            }

            this.display.sseq.emit("update");
        });
    }

    static unwrapProperty(start, list) {
        let t = start;
        for (let i of list){
            t = t[i];
        }
        return t;
    }
}

const _Panel = Panel;


/***/ }),
/* 2 */
/***/ (function(module, exports, __webpack_require__) {

"use strict";


const MARGIN = 10;

class Tooltip {
    constructor(display) {
        this.display = display;

        this.div = document.createElement("div");
        this.div.style.opacity = 0;
        this.div.style.position = "absolute";
        this.div.style["z-index"] = 999999;
        this.div.className = "tooltip";

        document.body.appendChild(this.div);
    }

    setHTML(html) {
        this.div.innerHTML = html;
    }

    show(x, y) {
        /**
         * Reset the tooltip position. This prevents a bug that occurs when the
         * previously displayed tooltip is positioned near the edge (but still
         * positioned to the right of the node), and the new tooltip text is
         * longer than the previous tooltip text. This may cause the new
         * (undisplayed) tooltip text to wrap, which gives an incorrect value
         * of rect.width and rect.height. The bug also occurs after resizing,
         * where the location of the previous tooltip is now outside of the
         * window.
         */
        this.div.style.left = "0px";
        this.div.style.top = "0px";

        let rect = this.div.getBoundingClientRect();
        let canvasRect = this.display.canvas.getBoundingClientRect();

        x = x + canvasRect.x;
        y = y + canvasRect.y;

        /**
         * By default, show the tooltip to the top and right of (x, y), offset
         * by MARGIN. If this cuases the tooltip to leave the window, position
         * it to the bottom/left accordingly.
         */
        if (x + MARGIN + rect.width < window.innerWidth)
            x = x + MARGIN;
        else
            x = x - rect.width - MARGIN;

        if (y - rect.height - MARGIN > 0)
            y = y - rect.height - MARGIN;
        else
            y = y + MARGIN;

        this.div.style.left = `${x}px`;
        this.div.style.top = `${y}px`;

        this.div.style.transition = "opacity 200ms";
        this.div.style.opacity = 0.9;
    }

    hide () {
        this.div.style.transition = "opacity 500ms";
        this.div.style.opacity = 0;
    }
}

exports.Tooltip = Tooltip;


/***/ }),
/* 3 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Panel__ = __webpack_require__(1);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "b", function() { return __WEBPACK_IMPORTED_MODULE_0__Panel__["a"]; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__DifferentialPanel__ = __webpack_require__(17);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return __WEBPACK_IMPORTED_MODULE_1__DifferentialPanel__["a"]; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__StructlinePanel__ = __webpack_require__(18);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "c", function() { return __WEBPACK_IMPORTED_MODULE_2__StructlinePanel__["a"]; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3__TabbedPanel__ = __webpack_require__(19);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "d", function() { return __WEBPACK_IMPORTED_MODULE_3__TabbedPanel__["a"]; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_4__TablePanel__ = __webpack_require__(20);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "e", function() { return __WEBPACK_IMPORTED_MODULE_4__TablePanel__["a"]; });






/***/ }),
/* 4 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_events__ = __webpack_require__(6);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_events___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_0_events__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1_d3__ = __webpack_require__(14);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1_d3___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_1_d3__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__infinity_js__ = __webpack_require__(15);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__infinity_js___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_2__infinity_js__);






const GridEnum = Object.freeze({ go : 1, chess : 2 });

class Display extends __WEBPACK_IMPORTED_MODULE_0_events__ {
    // container is either an id (e.g. "#main") or a DOM object
    constructor(container, sseq) {
        super();

        this.leftMargin = 40;
        this.rightMargin = 5;
        this.topMargin = 45;
        this.bottomMargin = 50;
        this.domainOffset = 1 / 2;

        this.gridStyle = GridEnum.go;
        this.gridColor = "#c6c6c6";
        this.background_color = "#FFFFFF";
        this.gridStrokeWidth = 0.3;
        this.TICK_STEP_LOG_BASE = 1.1; // Used for deciding when to change tick step.
        this.bidegreeDistanceThreshold = 15;

        this.hiddenStructlines = new Set();
        this.updateQueue = 0;

        this.container = __WEBPACK_IMPORTED_MODULE_1_d3__["select"](container);
        this.container_DOM = this.container.node();

        this.container.selectAll().remove();

        this.xScaleInit = __WEBPACK_IMPORTED_MODULE_1_d3__["scaleLinear"]();
        this.yScaleInit = __WEBPACK_IMPORTED_MODULE_1_d3__["scaleLinear"]();

        this.canvas = document.createElement("canvas");
        this.canvas.style.padding = "0px";
        this.canvas.style.position = "absolute";
        this.canvas.style.top = "0";
        this.canvas.style.left = "0";

        this.container_DOM.appendChild(this.canvas);

        this.context = this.canvas.getContext("2d");

        this.updateBatch = this.updateBatch.bind(this);
        this.nextPage = this.nextPage.bind(this);
        this.previousPage = this.previousPage.bind(this);
        this._emitMouseover = this._emitMouseover.bind(this);
        this._emitClick = this._emitClick.bind(this);

        this.zoom = __WEBPACK_IMPORTED_MODULE_1_d3__["zoom"]().scaleExtent([0, 4]);
        this.zoom.on("zoom", this.updateBatch);
        this.zoomD3Element = __WEBPACK_IMPORTED_MODULE_1_d3__["select"](this.canvas);
        this.zoomD3Element.call(this.zoom).on("dblclick.zoom", null);

        this.canvas.addEventListener("mousemove", this._emitMouseover);
        this.canvas.addEventListener("click", this._emitClick);

        // TODO: improve window resize handling. Currently the way that the domain changes is suboptimal.
        // I think the best would be to maintain the x and y range by scaling.
        window.addEventListener("resize",  () => this.resize());

        if(sseq) {
            this.setSseq(sseq);
        }
    }

    setBackgroundColor(color) {
        this.background_color = color;
        this.container_DOM.style["background"] = color;
        this.update();
    }

    /**
     *
     * @param width Optional width. Default to 97% of width of bounding element.
     * @param height Optional height. Default to 97% of height of bounding element.
     */
    resize(width, height){
        if(!this.sseq) {
            return;
        }

        let oldxmin = this.xminFloat;
        let oldymin = this.yminFloat;
        // This fixes the scale, but leaves a
        this._initializeCanvas(width, height);
        this._updateScale();
        let dx = this.xminFloat - oldxmin;
        let dy = this.yminFloat - oldymin;
        this.zoom.on("zoom", null);
        this.zoom.translateBy(this.zoomD3Element, this.dxScale(dx), this.dyScale(dy));
        this.zoom.on("zoom", this.updateBatch);
        this.updateBatch();
    }

    /**
     * Initialization method called in constructor.
     * @private
     */
    _initializeCanvas(width, height){
        const boundingRectangle = this.container_DOM.getBoundingClientRect();
        const canvasWidth = width || 0.99*boundingRectangle.width;
        const canvasHeight = height || 0.97*boundingRectangle.height;

        this.canvasWidth = canvasWidth;
        this.canvasHeight = canvasHeight;

        this.canvas.width = canvasWidth;
        this.canvas.height = canvasHeight;

        this.clipWidth = this.canvasWidth - this.rightMargin;
        this.clipHeight = this.canvasHeight - this.bottomMargin;

        this.plotWidth = this.canvasWidth - this.leftMargin - this.rightMargin;
        this.plotHeight = this.canvasHeight - this.bottomMargin - this.topMargin;

        this.xScaleInit = this.xScaleInit.range([this.leftMargin, this.clipWidth]);
        this.yScaleInit = this.yScaleInit.range([this.clipHeight, this.topMargin]);
    }


    /**
     * Set the spectral sequence to display.
     * @param ss
     */
    setSseq(sseq){
        if(this.sseq) {
            this.sseq.removeListener("update", this.updateBatch);
        }
        this.sseq = sseq;
        // The sseq object contains the list of valid pages. Always includes at least 0 and infinity.
        if(this.sseq.initial_page_idx){
            this.page_idx = this.sseq.initial_page_idx;
        } else {
            this.page_idx = this.sseq.min_page_idx;
        }
        if(this.page_idx >= this.sseq.page_list.length){
            console.warn(`Warning: min_page_idx ${this.sseq.min_page_idx} greater than page list length ${this.sseq.page_list.length}. Using 0 for min_page_idx instead.`);
            this.page_idx = 0;
            this.min_page_idx = 0;
        }
        this.setPage();

        this._initializeScale();
        this._initializeCanvas();

        if(sseq.gridStyle){
            this.gridStyle = sseq.gridStyle;
        }

        this.sseq.on('update',this.updateBatch);
        this.update();
    }

    _initializeScale(){
        this.xScaleInit.domain([this.sseq.initial_x_range[0] - this.domainOffset, this.sseq.initial_x_range[1] + this.domainOffset]);
        this.yScaleInit.domain([this.sseq.initial_y_range[0] - this.domainOffset, this.sseq.initial_y_range[1] + this.domainOffset]);
    }

    nextPage(){
        if (this.page_idx < this.sseq.page_list.length - 1) {
            this.setPage(this.page_idx + 1);
            this.update();
        }
    }

    previousPage(){
        if (this.page_idx > this.sseq.min_page_idx) {
            this.setPage(this.page_idx - 1);
            this.update();
        }
    }

    /**
     * Update this.page and this.pageRange to reflect the value of page_idx.
     * Eventually I should make a display that indicates the current page again, then this can also say what that is.
     */
    setPage(idx){
        if (!this.sseq) return;

        if(idx !== undefined){
            this.page_idx = idx;
        }
        this.pageRange = this.sseq.page_list[this.page_idx];

        if(Array.isArray(this.pageRange)){
            this.page = this.pageRange[0];
        } else {
            this.page = this.pageRange;
        }
        this.emit("page-change", this.pageRange, this.page_idx);
    }

    /**
     * The main updateAll routine.
     */
    updateBatch(){
        this.update(true);
    }

    update(batch = false) {
        if (!this.sseq) return;

        this.updateQueue ++;

        let drawFunc = () => {
            this.updateQueue --;
            if (this.updateQueue != 0) return;

            this._drawSseq(this.context);
            if (__WEBPACK_IMPORTED_MODULE_1_d3__["event"]) {
                // d3 zoom doesn't allow the events it handles to bubble, so we
                // fails to track pointer position.
                this._emitMouseover(__WEBPACK_IMPORTED_MODULE_1_d3__["event"]);
            } else {
                this._emitMouseover();
            }
        };
        if(batch){
            requestAnimationFrame(drawFunc);
        } else {
            drawFunc();
        }
    }

    clipContext(ctx) {
        ctx.beginPath();
        let y_clip_offset = this.y_clip_offset || 0;
        ctx.globalAlpha = 0; // C2S does not correctly clip unless the clip is stroked.
        ctx.rect(this.leftMargin, this.topMargin + y_clip_offset, this.plotWidth, this.plotHeight - y_clip_offset);
        ctx.stroke();
        ctx.clip();
        ctx.globalAlpha = 1;
    }

    _drawSseq(ctx = this.context) {
        if (!this.sseq) return;

        this._updateScale();
        this._updateGridAndTickStep();

        ctx.clearRect(0, 0, this.canvasWidth, this.canvasHeight);

        this._drawTicks(ctx);
        this._drawAxes(ctx);

        ctx.save();

        this.clipContext(ctx);

        let [nodes, edges] = this.sseq.getElementsToDraw(
            this.pageRange, 
            this.xmin - 1, this.xmax + 1, this.ymin - 1, this.ymax + 1
        );

        this._drawGrid(ctx);
        this.emit("draw_background");
        this._updateNodes(nodes);
        this._hightlightClasses(ctx);
        this._drawEdges(ctx, edges);
        this._drawClasses(ctx);

        if (this.sseq.edgeLayerSVG)
            this.drawSVG(ctx, this.sseq.edgeLayerSVG);

        if(this.svg) {
            if(this.svg_unclipped){
                ctx.restore();
                ctx.save();
            }
            let x_scale = this.svg_x_scale || this.svg_scale || 1;
            let y_scale = this.svg_y_scale || this.svg_scale || 1;
            let x_offset = this.svg_x_offset || 0;
            let y_offset = this.svg_y_offset || 0;
            let default_width = 
                this.canvasWidth / (this.xmaxFloat - this.xminFloat) * (this.sseq.x_range[1] - this.sseq.x_range[0] + 1);
            let default_height = 
                this.canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.sseq.y_range[1] - this.sseq.y_range[0] + 1);
            let width = default_width * x_scale;
            let height = default_height * y_scale;
            this.context.drawImage(this.svg,
                this.xScale(this.sseq.x_range[0] + x_offset), //- display.xMinOffset,
                this.yScale(this.sseq.y_range[1] + 1 + y_offset) ,
                width, height
            );
        }
        ctx.restore();


        this.emit("draw");
    }

    /**
     * @private
     */
    _updateScale(){
        let zoomD3Element = this.zoomD3Element;
        let transform = __WEBPACK_IMPORTED_MODULE_1_d3__["zoomTransform"](zoomD3Element.node());
        let scale = transform.k;
        let xScale = transform.rescaleX(this.xScaleInit);
        let yScale = transform.rescaleY(this.yScaleInit);

        // We have to call zoom.translateBy when the user hits the boundary of the pan region
        // to adjust the zoom transform. However, this causes the zoom handler (this function) to be called a second time,
        // which is less intuitive program flow than just continuing on in the current function.
        // In order to prevent this, temporarily unset the zoom handler.
        // TODO: See if we can make the behaviour here less jank.
        this.zoom.on("zoom", null);

        let xScaleMaxed = false, yScaleMaxed = false;
        // Prevent user from panning off the side.
        if (this.sseq.x_range) {
            if (xScale(this.sseq.x_range[1] - this.sseq.x_range[0] + 2 * this.domainOffset) - xScale(0) < this.plotWidth) {
                // We simply record the scale was maxed and handle this later
                // by modifying xScale directly.
                xScaleMaxed = true;
            } else if (xScale(this.sseq.x_range[0] - this.domainOffset) > this.leftMargin) {
                this.zoom.translateBy(zoomD3Element, (this.leftMargin - xScale(this.sseq.x_range[0] - this.domainOffset)) / scale, 0);
            } else if (xScale(this.sseq.x_range[1] + this.domainOffset) < this.clipWidth) {
                this.zoom.translateBy(zoomD3Element, (this.clipWidth - xScale(this.sseq.x_range[1] + this.domainOffset)) / scale, 0);
            }
        }

        if (this.sseq.y_range) {
            if (yScale(0) -yScale(this.sseq.y_range[1] - this.sseq.y_range[0] + 2 * this.domainOffset) < this.plotHeight) {
                yScaleMaxed = true;
            } else if (yScale(this.sseq.y_range[0] - this.domainOffset) < this.clipHeight) {
                this.zoom.translateBy(zoomD3Element, 0, (this.clipHeight - yScale(this.sseq.y_range[0] - this.domainOffset)) / scale);
            } else if (yScale(this.sseq.y_range[1] + this.domainOffset) > this.topMargin) {
                this.zoom.translateBy(zoomD3Element, 0, this.topMargin - yScale(this.sseq.y_range[1] + this.domainOffset) / scale);
            }
        }

        // If both scales are maxed, and the user attempts to zoom out further,
        // d3 registers a zoom, but nothing in the interface changes since we
        // manually override xScale and yScale instead of doing something at
        // the level of the transform (see below). We do *not* want to keep
        // zooming out, or else when the user wants to zoom back in, they will
        // have to zoom in for a while before the interface actually zooms in.
        // Thus, We restore the previous zoom state.
        if (xScaleMaxed && yScaleMaxed) {
            if (this.oldScalesMaxed && scale < this.scale) {
                this.zoom.transform(zoomD3Element, this.transform);
                this.zoom.on("zoom", this.updateBatch);
                return;
            } else {
                this.oldScalesMaxed = true;
            }
        } else {
            this.oldScalesMaxed = false;
        }

        // Get new transform and scale objects after possible translation above
        this.transform = __WEBPACK_IMPORTED_MODULE_1_d3__["zoomTransform"](zoomD3Element.node());
        this.scale = this.transform.k;
        this.xScale = this.transform.rescaleX(this.xScaleInit);
        this.yScale = this.transform.rescaleY(this.yScaleInit);

        // If x or y scale is maxed, we directly override xScale/yScale instead
        // of messing with zoom, since we want to continue allow zooming in the
        // other direction
        if (xScaleMaxed) {
            this.xScale.domain([
                this.sseq.x_range[0] - this.domainOffset,
                this.sseq.x_range[1] + this.domainOffset
            ]);
        }
        if (yScaleMaxed) {
            this.yScale.domain([
                this.sseq.y_range[0] - this.domainOffset,
                this.sseq.y_range[1] + this.domainOffset
            ]);
        }

        this.xminFloat = this.xScale.invert(this.leftMargin);
        this.xmaxFloat = this.xScale.invert(this.clipWidth);
        this.yminFloat = this.yScale.invert(this.clipHeight);
        this.ymaxFloat = this.yScale.invert(this.topMargin);
        this.xmin = Math.ceil(this.xminFloat);
        this.xmax = Math.floor(this.xmaxFloat);
        this.ymin = Math.ceil(this.yminFloat);
        this.ymax = Math.floor(this.ymaxFloat);

        this.zoom.on("zoom", this.updateBatch);
    }

    dxScale(x){
        return this.xScale(x) - this.xScale(0);
    }

    dyScale(x){
        return this.yScale(x) - this.yScale(0);
    }

    _updateGridAndTickStep(){
        // TODO: This 70 is a magic number. Maybe I should give it a name?
        this.xTicks = this.xScale.ticks(this.canvasWidth / 70);
        this.yTicks = this.yScale.ticks(this.canvasHeight / 70);

        this.xTickStep = Math.ceil(this.xTicks[1] - this.xTicks[0]);
        this.yTickStep = Math.ceil(this.yTicks[1] - this.yTicks[0]);
        this.xTicks[0] -= this.xTickStep;
        this.yTicks[0] -= this.yTickStep;
        this.xTicks.push(this.xTicks[this.xTicks.length - 1] + this.xTickStep);
        this.yTicks.push(this.yTicks[this.yTicks.length - 1] + this.yTickStep);

        if(this.manualxGridStep){
            this.xGridStep = this.manualxGridStep;
        } else {
            this.xGridStep = (Math.floor(this.xTickStep / 5) === 0) ? 1 : Math.floor(this.xTickStep / 5);
        }
        if(this.manualyGridStep){
            this.yGridStep = this.manualxGridStep;
        } else {
            this.yGridStep = (Math.floor(this.yTickStep / 5) === 0) ? 1 : Math.floor(this.yTickStep / 5);
        }
        // TODO: This is an ad-hoc modification requested by Danny to ensure that the grid boxes are square.
        // Probably it's a useful thing to be able to have square grid boxes, how do we want to deal with this?
        if(this.sseq.squareAspectRatio){
            this.xGridStep = 1;
            this.yGridStep = this.xGridStep;
        }
    }

    _drawTicks(context) {
        context.save();

        context.textBaseline = "middle";
        context.font = "15px Arial";
        context.textAlign = "center";
        for (let i = Math.floor(this.xTicks[0]); i <= this.xTicks[this.xTicks.length - 1]; i += this.xTickStep) {
            context.fillText(i, this.xScale(i), this.clipHeight + 20);
        }

        context.textAlign = "right";
        for (let i = Math.floor(this.yTicks[0]); i <= this.yTicks[this.yTicks.length - 1]; i += this.yTickStep) {
            context.fillText(i, this.leftMargin - 10, this.yScale(i));
        }
        context.restore();
    }

    _drawGrid(context){
        context.save();

        context.strokeStyle = this.gridColor;
        context.lineWidth = this.gridStrokeWidth;

        switch(this.gridStyle){
            case GridEnum.go:
                this._drawGoGrid(context);
                break;
            case GridEnum.chess:
                this._drawChessGrid(context);
                break;
            default:
                throw Error("Undefined grid type.");
                break;
        }

        context.restore();
    }

    _drawGoGrid(context) {
        this._drawGridWithOffset(context, 0, 0);
    }

    _drawChessGrid(context) {
        this._drawGridWithOffset(context, 0.5, 0.5);
    }

    _drawGridWithOffset(context, xoffset, yoffset){
        context.beginPath();
        for (let col = Math.floor(this.xmin / this.xGridStep) * this.xGridStep - xoffset; col <= this.xmax; col += this.xGridStep) {
            context.moveTo(this.xScale(col), 0);
            context.lineTo(this.xScale(col), this.clipHeight);
        }
        context.stroke();

        context.beginPath();
        for (let row = Math.floor(this.ymin / this.yGridStep) * this.yGridStep - yoffset; row <= this.ymax; row += this.yGridStep) {
            context.moveTo(this.leftMargin, this.yScale(row));
            context.lineTo(this.canvasWidth - this.rightMargin, this.yScale(row));
        }
        context.stroke();
    }

    _drawAxes(context){
        context.save();

        // This makes the white square in the bottom left and top right corners which prevents axes labels from appearing to the left
        // or below the axes intercept.
        context.fillStyle = this.background_color;
        context.rect(0, this.clipHeight, this.leftMargin, this.bottomMargin);
        context.rect(0, 0, this.leftMargin, this.topMargin);
        context.fill();
        context.fillStyle = "#000";

        // Draw the axes.
        context.beginPath();
        context.moveTo(this.leftMargin, this.topMargin);
        context.lineTo(this.leftMargin, this.clipHeight);
        context.lineTo(this.canvasWidth - this.rightMargin, this.clipHeight);
        context.stroke();

        context.restore();
    }

    _updateNodes(classes){
        let size = Math.max(Math.min(this.dxScale(1), -this.dyScale(1), this.sseq.max_class_size), this.sseq.min_class_size) * this.sseq.class_scale;
        this.classes_to_draw = classes;
        for(let c of classes) {
            c.setPosition( 
                this.xScale(c.x) + c.getXOffset(), 
                this.yScale(c.y) + c.getYOffset(), 
                size
            );
        }
    }

    _hightlightClasses(context) {
        for (let c of this.classes_to_draw) {
            if(c._highlight){
                c.drawHighlight(context);
            }
        }
    }

    _drawClasses(context) {
        for (let c of this.classes_to_draw) {
            c.draw(context);
            c.updateTooltipPath();
        }
    }

    _drawEdges(context, edges){        
        for (let e of edges) {
            if(!e) {
                throw ValueError("Undefined edge.");
            }
            if(e.invalid || !e.visible){
                continue;
            }
            if (e.type === "Structline" && this.hiddenStructlines.has(e.mult)) {
                continue;
            }

            let source_node = e._source;
            let target_node = e._target;
            if(!source_node || ! target_node){
                throw ValueError(`Edge ${e} has undefined source or target node`);
            }
            e._sourceOffset = e.sourceOffset || {x: 0, y: 0};
            e._targetOffset = e.targetOffset || {x: 0, y: 0};

            context.save();
            context.strokeStyle = e.color;
            if(e.lineWidth){
                context.lineWidth = e.lineWidth;
            }
            if(e.opacity){
                context.globalAlpha = e.opacity;
            }
            if(e.dash){
                context.setLineDash(e.dash);
            }

            let sourceX = source_node._canvas_x + e._sourceOffset.x;
            let sourceY = source_node._canvas_y + e._sourceOffset.y;
            let targetX = target_node._canvas_x + e._targetOffset.x;
            let targetY = target_node._canvas_y + e._targetOffset.y;

            context.beginPath();
            if(e.bend ){//&& e.bend !== 0
                let distance = Math.sqrt((targetX - sourceX)*(targetX - sourceX) + (targetY - sourceY)*(targetY - sourceY));
                let looseness = 0.4;
                if(e.looseness){
                    looseness = e.looseness;
                }
                let angle = Math.atan((targetY - sourceY)/(targetX - sourceX));
                let bendAngle = - e.bend * Math.PI/180;
                let control1X = sourceX + Math.cos(angle + bendAngle) * looseness * distance;
                let control1Y = sourceY + Math.sin(angle + bendAngle) * looseness * distance;
                let control2X = targetX - Math.cos(angle - bendAngle) * looseness * distance;
                let control2Y = targetY - Math.sin(angle - bendAngle) * looseness * distance;
                context.moveTo(sourceX, sourceY);
                context.bezierCurveTo(control1X, control1Y, control2X, control2Y, targetX, targetY);
            } else {
                context.moveTo(sourceX, sourceY);
                context.lineTo(targetX, targetY);
            }
            context.stroke();
            context.restore();
        }
    }

    prepareMouseEventObject(){
        let o = {};
        o.real_x = this.xScale.invert(this.mousex);
        o.real_y = this.yScale.invert(this.mousey);
        o.x = Math.round(o.real_x);
        o.y = Math.round(o.real_y);
        let dx = o.x - o.real_x;
        let dy = o.y - o.real_y;
        o.distance = Math.sqrt(dx*dx + dy*dy);
        o.mouseover_class = this.mouseover_class;
        o.mouseover_bidegree = this.mouseover_bidegree;
        return o;
    }

    _emitClick(e) {
        let o = this.prepareMouseEventObject();
        o.event = e;
        this.emit("click", o);
    }

    _emitMouseover(e, redraw) {
        // If not yet set up 
        if (!this.classes_to_draw) {
            return;
        }

        // We cannot query for mouse position. We must remember it from
        // previous events. If update() is called, we call _onMousemove without
        // an event.
        // let rect = this.canvas.getBoundingClientRect();
        if(e) {
            this.mousex = e.layerX; //e.clientX - rect.x;
            this.mousey = e.layerY; //e.clientY - rect.y;
        }
        redraw = redraw | false;
        redraw |= this._emitMouseoverClass();
        redraw |= this._emitMouseoverBidegree();

        if (redraw) {
            this._drawSseq(this.context);  
        } 
    }

    _emitMouseoverClass(){
        let redraw = false;
        if (this.mouseover_class) {
            if(
                this.classes_to_draw.includes(this.mouseover_class) 
                && this.context.isPointInPath(this.mouseover_class._path, this.mousex, this.mousey)
            ) {
                return false;
            } else {
                this.emit("mouseout-class", this.mouseover_class);
                this.mouseover_class = null;
                redraw = true;
            }
        }
        let c = this.classes_to_draw.find(c => this.context.isPointInPath(c._path, this.mousex, this.mousey));
        if(c) {
            redraw = true;
            this.mouseover_class = c;
            this.emit("mouseover-class", c);
        }
        return redraw;
    }

    _emitMouseoverBidegree(){
        let x = this.mousex;
        let y = this.mousey;
        let nearest_x = Math.round(this.xScale.invert(x));
        let nearest_y = Math.round(this.yScale.invert(y));
        let redraw = false;
        let threshold = this.bidegreeDistanceThreshold * (this.sseq.bidegreeDistanceScale | 1);
        let xscale = 1;
        let yscale = 1;
        // let x_max_threshold = Math.abs(this.xScale(1) - this.xScale(0)) * 0.4;
        // let y_max_threshold = Math.abs(this.yScale(1) - this.yScale(0)) * 0.4;
        // if(threshold > x_max_threshold) {
        //     xscale = x_max_threshold / threshold;
        // }
        // if(threshold > y_max_threshold) {
        //     yscale = y_max_threshold / threshold;
        // }
        if(this.mouseover_bidegree){
            let bidegree = this.mouseover_bidegree;
            let dx = (x - this.xScale(bidegree[0])) * xscale;
            let dy = (y - this.yScale(bidegree[1])) * yscale;
            let distance = Math.sqrt(dx * dx + dy * dy);
            if(distance < threshold){
                return false;
            } else {
                this.emit("mouseout-bidegree", this.mouseover_bidegree);
                this.mouseover_bidegree = null;
                redraw = true;
            }
        }
        

        let bidegree = [nearest_x, nearest_y];
        let dx = (x - this.xScale(bidegree[0])) * xscale;
        let dy = (y - this.yScale(bidegree[1])) * yscale;
        let distance = Math.sqrt(dx * dx + dy * dy);
        if(distance < threshold){
            redraw = true;
            this.mouseover_bidegree = bidegree;
            this.emit("mouseover-bidegree", bidegree);
        }
        return redraw;
    }

    /**
     * Draw an svg onto the canvas.
     * @param context html5 canvas context
     * @param xml An svg string
     */
    drawSVG(context, xml){
        // make it base64
        let svg64 = btoa(xml);
        let b64Start = 'data:image/svg+xml;base64,';

        // prepend a "header"
        let image64 = b64Start + svg64;

        // set it as the source of the img element
        let img = new Image();
        img.src = image64;

        context.drawImage(img,
            this.xScale(this.sseq.x_range[0]),// - this.xMinOffset,
            this.yScale(this.sseq.y_range[1] + 1),
            this.canvasWidth  / (this.xmaxFloat - this.xminFloat) * (this.sseq.x_range[1] - this.sseq.x_range[0] + 1),
            this.canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.sseq.y_range[1] - this.sseq.y_range[0] + 1)
        );
    }

    toSVG(){
        let ctx = new C2S(this.canvasWidth, this.canvasHeight);
        this._drawSseq(ctx);

        return ctx.getSerializedSvg(true);
    }

    downloadSVG(filename) {
        if(filename === undefined){
            filename = `${this.sseq.name}_x-${this.xmin}-${this.xmax}_y-${this.ymin}-${this.ymax}.svg`
        }
        IO.download(filename, this.toSVG(), "image/svg+xml")
    }

    /**
     * Move the canvas to contain (x,y)
     * TODO: control speed, control acceptable range of target positions, maybe zoom out if display is super zoomed in?
     * @param x
     * @param y
     */
    seek(x, y){
        return new Promise((resolve) => {
            let dx = 0;
            let dy = 0;
            if (x > this.xmaxFloat - 1) {
                dx = this.xmaxFloat - 1 - x;
            } else if (x < this.xminFloat + 1) {
                dx = this.xminFloat + 1 - x;
            }
            if (y > this.ymaxFloat - 1) {
                dy = this.ymaxFloat - 1 - y;
            } else if (y < this.xminFloat + 1) {
                dy = this.yminFloat + 1 - y;
            }
            if (dx === 0 && dy === 0) {
                return;
            }

            let dxActual = this.dxScale(dx);
            let dyActual = this.dyScale(dy);
            let dist = Math.sqrt(dxActual * dxActual + dyActual * dyActual);
            // steps controls the speed -- doubling steps halves the speed.
            // Of course we could maybe set up some fancy algorithm that zooms and pans.
            let steps = Math.ceil(dist / 10);
            let xstep = dxActual / steps;
            let ystep = dyActual / steps;

            let i = 0;
            let t = __WEBPACK_IMPORTED_MODULE_1_d3__["interval"](() => {
                i++;
                this.translateBy(xstep, ystep);
                if (i >= steps) {
                    t.stop();
                    resolve();
                }
            }, 5);
        });
    }

    translateBy(xstep, ystep){
        this.zoom.on("zoom", null);
        this.zoom.translateBy(this.zoomD3Element, xstep / this.scale, ystep / this.scale );
        this.update();
        this.zoom.on("zoom", this.updateBatch);
    }

    getPageDescriptor(pageRange) {
        if (!this.sseq) return;

        let basePage = 2;
        if(this.sseq.page_list.includes(1)){
            basePage = 1;
        }
        if (pageRange[0] === __WEBPACK_IMPORTED_MODULE_2__infinity_js___default.a) {
            return "Page ∞";
        }
        if (pageRange === 0) {
            return `Page ${basePage} with all differentials`;
        }
        if (pageRange === 1 && basePage === 2) {
            return `Page ${basePage} with no differentials`;
        }
        if (pageRange.length) {
            if(pageRange[1] === __WEBPACK_IMPORTED_MODULE_2__infinity_js___default.a){
                return `Page ${pageRange[0]} with all differentials`;
            }
            if(pageRange[1] === -1){
                return `Page ${pageRange[0]} with no differentials`;
            }

            if(pageRange[0] === pageRange[1]){
                return `Page ${pageRange[0]}`;
            }

            return `Pages ${pageRange[0]} – ${pageRange[1]}`.replace(__WEBPACK_IMPORTED_MODULE_2__infinity_js___default.a, "∞");
        }
        return `Page ${pageRange}`;
    }

    // TODO: Fix the selection
    //    /**
    //     * This is a click event handler to update the selected cell when the user clicks.
    //     * @param event A click event.
    //     */
    //    updateSelection(event){
    //        event.mouseover_class = this.mouseover_class;
    //        this.selectedX = Math.floor(display.xScale.invert(event.layerX) + 0.5);
    //        this.selectedY = Math.floor(display.yScale.invert(event.layerY) + 0.5);
    //        this.update();
    //    }
    //
    //    /**
    //     * Enable selection. This changes the grid style to a chess grid and attaches event handlers for clicking
    //     * @param arrowNavigate
    //     */
    //    enableSelection(arrowNavigate){
    //        this.gridStyle = gridChess;
    //        this.addEventHandler("onclick",this.updateSelection.bind(this));
    //        if(arrowNavigate){
    //            this.addEventHandler('left',  () => {
    //                if(this.selectedX !== undefined){
    //                    this.selectedX --;
    //                    this.update();
    //                }
    //            });
    //            this.addEventHandler('right', () => {
    //                if(this.selectedX !== undefined){
    //                    this.selectedX ++;
    //                    this.update();
    //                }
    //            });
    //            this.addEventHandler('down',  () => {
    //                if(this.selectedY !== undefined){
    //                    this.selectedY --;
    //                    this.update();
    //                }
    //            });
    //            this.addEventHandler('up', () => {
    //                if(this.selectedY !== undefined){
    //                    this.selectedY ++;
    //                    this.update();
    //                }
    //            });
    //        }
    //        this.update();
    //    }
    //
    //    disableSelection(){
    //        this.selectedX = undefined;
    //        this.gridStyle = gridGo;
    //        Mousetrap.bind('left',  this.previousPage);
    //        Mousetrap.bind('right', this.nextPage);
    //        this.eventHandlerLayer["onclick"] = (event) => {};
    //        this.update();
    //    }
    //
    //    _drawSelection(context){
    //        let x = this.selectedX;
    //        let y = this.selectedY;
    //        if(x !== undefined && y !== undefined){
    //            context.fillStyle = this.gridColor;
    //            context.rect(
    //                display.xScale(x - 0.5),
    //                display.yScale(y - 0.5),
    //                display.dxScale(1),
    //                display.dyScale(1)
    //            );
    //            context.fill();
    //        }
    //    }

}
/* harmony export (immutable) */ __webpack_exports__["a"] = Display;


/***/ }),
/* 5 */
/***/ (function(module, exports, __webpack_require__) {

var __WEBPACK_AMD_DEFINE_RESULT__;/*global define:false */
/**
 * Copyright 2012-2017 Craig Campbell
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Mousetrap is a simple keyboard shortcut library for Javascript with
 * no external dependencies
 *
 * @version 1.6.5
 * @url craig.is/killing/mice
 */
(function(window, document, undefined) {

    // Check if mousetrap is used inside browser, if not, return
    if (!window) {
        return;
    }

    /**
     * mapping of special keycodes to their corresponding keys
     *
     * everything in this dictionary cannot use keypress events
     * so it has to be here to map to the correct keycodes for
     * keyup/keydown events
     *
     * @type {Object}
     */
    var _MAP = {
        8: 'backspace',
        9: 'tab',
        13: 'enter',
        16: 'shift',
        17: 'ctrl',
        18: 'alt',
        20: 'capslock',
        27: 'esc',
        32: 'space',
        33: 'pageup',
        34: 'pagedown',
        35: 'end',
        36: 'home',
        37: 'left',
        38: 'up',
        39: 'right',
        40: 'down',
        45: 'ins',
        46: 'del',
        91: 'meta',
        93: 'meta',
        224: 'meta'
    };

    /**
     * mapping for special characters so they can support
     *
     * this dictionary is only used incase you want to bind a
     * keyup or keydown event to one of these keys
     *
     * @type {Object}
     */
    var _KEYCODE_MAP = {
        106: '*',
        107: '+',
        109: '-',
        110: '.',
        111 : '/',
        186: ';',
        187: '=',
        188: ',',
        189: '-',
        190: '.',
        191: '/',
        192: '`',
        219: '[',
        220: '\\',
        221: ']',
        222: '\''
    };

    /**
     * this is a mapping of keys that require shift on a US keypad
     * back to the non shift equivelents
     *
     * this is so you can use keyup events with these keys
     *
     * note that this will only work reliably on US keyboards
     *
     * @type {Object}
     */
    var _SHIFT_MAP = {
        '~': '`',
        '!': '1',
        '@': '2',
        '#': '3',
        '$': '4',
        '%': '5',
        '^': '6',
        '&': '7',
        '*': '8',
        '(': '9',
        ')': '0',
        '_': '-',
        '+': '=',
        ':': ';',
        '\"': '\'',
        '<': ',',
        '>': '.',
        '?': '/',
        '|': '\\'
    };

    /**
     * this is a list of special strings you can use to map
     * to modifier keys when you specify your keyboard shortcuts
     *
     * @type {Object}
     */
    var _SPECIAL_ALIASES = {
        'option': 'alt',
        'command': 'meta',
        'return': 'enter',
        'escape': 'esc',
        'plus': '+',
        'mod': /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'meta' : 'ctrl'
    };

    /**
     * variable to store the flipped version of _MAP from above
     * needed to check if we should use keypress or not when no action
     * is specified
     *
     * @type {Object|undefined}
     */
    var _REVERSE_MAP;

    /**
     * loop through the f keys, f1 to f19 and add them to the map
     * programatically
     */
    for (var i = 1; i < 20; ++i) {
        _MAP[111 + i] = 'f' + i;
    }

    /**
     * loop through to map numbers on the numeric keypad
     */
    for (i = 0; i <= 9; ++i) {

        // This needs to use a string cause otherwise since 0 is falsey
        // mousetrap will never fire for numpad 0 pressed as part of a keydown
        // event.
        //
        // @see https://github.com/ccampbell/mousetrap/pull/258
        _MAP[i + 96] = i.toString();
    }

    /**
     * cross browser add event method
     *
     * @param {Element|HTMLDocument} object
     * @param {string} type
     * @param {Function} callback
     * @returns void
     */
    function _addEvent(object, type, callback) {
        if (object.addEventListener) {
            object.addEventListener(type, callback, false);
            return;
        }

        object.attachEvent('on' + type, callback);
    }

    /**
     * takes the event and returns the key character
     *
     * @param {Event} e
     * @return {string}
     */
    function _characterFromEvent(e) {

        // for keypress events we should return the character as is
        if (e.type == 'keypress') {
            var character = String.fromCharCode(e.which);

            // if the shift key is not pressed then it is safe to assume
            // that we want the character to be lowercase.  this means if
            // you accidentally have caps lock on then your key bindings
            // will continue to work
            //
            // the only side effect that might not be desired is if you
            // bind something like 'A' cause you want to trigger an
            // event when capital A is pressed caps lock will no longer
            // trigger the event.  shift+a will though.
            if (!e.shiftKey) {
                character = character.toLowerCase();
            }

            return character;
        }

        // for non keypress events the special maps are needed
        if (_MAP[e.which]) {
            return _MAP[e.which];
        }

        if (_KEYCODE_MAP[e.which]) {
            return _KEYCODE_MAP[e.which];
        }

        // if it is not in the special map

        // with keydown and keyup events the character seems to always
        // come in as an uppercase character whether you are pressing shift
        // or not.  we should make sure it is always lowercase for comparisons
        return String.fromCharCode(e.which).toLowerCase();
    }

    /**
     * checks if two arrays are equal
     *
     * @param {Array} modifiers1
     * @param {Array} modifiers2
     * @returns {boolean}
     */
    function _modifiersMatch(modifiers1, modifiers2) {
        return modifiers1.sort().join(',') === modifiers2.sort().join(',');
    }

    /**
     * takes a key event and figures out what the modifiers are
     *
     * @param {Event} e
     * @returns {Array}
     */
    function _eventModifiers(e) {
        var modifiers = [];

        if (e.shiftKey) {
            modifiers.push('shift');
        }

        if (e.altKey) {
            modifiers.push('alt');
        }

        if (e.ctrlKey) {
            modifiers.push('ctrl');
        }

        if (e.metaKey) {
            modifiers.push('meta');
        }

        return modifiers;
    }

    /**
     * prevents default for this event
     *
     * @param {Event} e
     * @returns void
     */
    function _preventDefault(e) {
        if (e.preventDefault) {
            e.preventDefault();
            return;
        }

        e.returnValue = false;
    }

    /**
     * stops propogation for this event
     *
     * @param {Event} e
     * @returns void
     */
    function _stopPropagation(e) {
        if (e.stopPropagation) {
            e.stopPropagation();
            return;
        }

        e.cancelBubble = true;
    }

    /**
     * determines if the keycode specified is a modifier key or not
     *
     * @param {string} key
     * @returns {boolean}
     */
    function _isModifier(key) {
        return key == 'shift' || key == 'ctrl' || key == 'alt' || key == 'meta';
    }

    /**
     * reverses the map lookup so that we can look for specific keys
     * to see what can and can't use keypress
     *
     * @return {Object}
     */
    function _getReverseMap() {
        if (!_REVERSE_MAP) {
            _REVERSE_MAP = {};
            for (var key in _MAP) {

                // pull out the numeric keypad from here cause keypress should
                // be able to detect the keys from the character
                if (key > 95 && key < 112) {
                    continue;
                }

                if (_MAP.hasOwnProperty(key)) {
                    _REVERSE_MAP[_MAP[key]] = key;
                }
            }
        }
        return _REVERSE_MAP;
    }

    /**
     * picks the best action based on the key combination
     *
     * @param {string} key - character for key
     * @param {Array} modifiers
     * @param {string=} action passed in
     */
    function _pickBestAction(key, modifiers, action) {

        // if no action was picked in we should try to pick the one
        // that we think would work best for this key
        if (!action) {
            action = _getReverseMap()[key] ? 'keydown' : 'keypress';
        }

        // modifier keys don't work as expected with keypress,
        // switch to keydown
        if (action == 'keypress' && modifiers.length) {
            action = 'keydown';
        }

        return action;
    }

    /**
     * Converts from a string key combination to an array
     *
     * @param  {string} combination like "command+shift+l"
     * @return {Array}
     */
    function _keysFromString(combination) {
        if (combination === '+') {
            return ['+'];
        }

        combination = combination.replace(/\+{2}/g, '+plus');
        return combination.split('+');
    }

    /**
     * Gets info for a specific key combination
     *
     * @param  {string} combination key combination ("command+s" or "a" or "*")
     * @param  {string=} action
     * @returns {Object}
     */
    function _getKeyInfo(combination, action) {
        var keys;
        var key;
        var i;
        var modifiers = [];

        // take the keys from this pattern and figure out what the actual
        // pattern is all about
        keys = _keysFromString(combination);

        for (i = 0; i < keys.length; ++i) {
            key = keys[i];

            // normalize key names
            if (_SPECIAL_ALIASES[key]) {
                key = _SPECIAL_ALIASES[key];
            }

            // if this is not a keypress event then we should
            // be smart about using shift keys
            // this will only work for US keyboards however
            if (action && action != 'keypress' && _SHIFT_MAP[key]) {
                key = _SHIFT_MAP[key];
                modifiers.push('shift');
            }

            // if this key is a modifier then add it to the list of modifiers
            if (_isModifier(key)) {
                modifiers.push(key);
            }
        }

        // depending on what the key combination is
        // we will try to pick the best event for it
        action = _pickBestAction(key, modifiers, action);

        return {
            key: key,
            modifiers: modifiers,
            action: action
        };
    }

    function _belongsTo(element, ancestor) {
        if (element === null || element === document) {
            return false;
        }

        if (element === ancestor) {
            return true;
        }

        return _belongsTo(element.parentNode, ancestor);
    }

    function Mousetrap(targetElement) {
        var self = this;

        targetElement = targetElement || document;

        if (!(self instanceof Mousetrap)) {
            return new Mousetrap(targetElement);
        }

        /**
         * element to attach key events to
         *
         * @type {Element}
         */
        self.target = targetElement;

        /**
         * a list of all the callbacks setup via Mousetrap.bind()
         *
         * @type {Object}
         */
        self._callbacks = {};

        /**
         * direct map of string combinations to callbacks used for trigger()
         *
         * @type {Object}
         */
        self._directMap = {};

        /**
         * keeps track of what level each sequence is at since multiple
         * sequences can start out with the same sequence
         *
         * @type {Object}
         */
        var _sequenceLevels = {};

        /**
         * variable to store the setTimeout call
         *
         * @type {null|number}
         */
        var _resetTimer;

        /**
         * temporary state where we will ignore the next keyup
         *
         * @type {boolean|string}
         */
        var _ignoreNextKeyup = false;

        /**
         * temporary state where we will ignore the next keypress
         *
         * @type {boolean}
         */
        var _ignoreNextKeypress = false;

        /**
         * are we currently inside of a sequence?
         * type of action ("keyup" or "keydown" or "keypress") or false
         *
         * @type {boolean|string}
         */
        var _nextExpectedAction = false;

        /**
         * resets all sequence counters except for the ones passed in
         *
         * @param {Object} doNotReset
         * @returns void
         */
        function _resetSequences(doNotReset) {
            doNotReset = doNotReset || {};

            var activeSequences = false,
                key;

            for (key in _sequenceLevels) {
                if (doNotReset[key]) {
                    activeSequences = true;
                    continue;
                }
                _sequenceLevels[key] = 0;
            }

            if (!activeSequences) {
                _nextExpectedAction = false;
            }
        }

        /**
         * finds all callbacks that match based on the keycode, modifiers,
         * and action
         *
         * @param {string} character
         * @param {Array} modifiers
         * @param {Event|Object} e
         * @param {string=} sequenceName - name of the sequence we are looking for
         * @param {string=} combination
         * @param {number=} level
         * @returns {Array}
         */
        function _getMatches(character, modifiers, e, sequenceName, combination, level) {
            var i;
            var callback;
            var matches = [];
            var action = e.type;

            // if there are no events related to this keycode
            if (!self._callbacks[character]) {
                return [];
            }

            // if a modifier key is coming up on its own we should allow it
            if (action == 'keyup' && _isModifier(character)) {
                modifiers = [character];
            }

            // loop through all callbacks for the key that was pressed
            // and see if any of them match
            for (i = 0; i < self._callbacks[character].length; ++i) {
                callback = self._callbacks[character][i];

                // if a sequence name is not specified, but this is a sequence at
                // the wrong level then move onto the next match
                if (!sequenceName && callback.seq && _sequenceLevels[callback.seq] != callback.level) {
                    continue;
                }

                // if the action we are looking for doesn't match the action we got
                // then we should keep going
                if (action != callback.action) {
                    continue;
                }

                // if this is a keypress event and the meta key and control key
                // are not pressed that means that we need to only look at the
                // character, otherwise check the modifiers as well
                //
                // chrome will not fire a keypress if meta or control is down
                // safari will fire a keypress if meta or meta+shift is down
                // firefox will fire a keypress if meta or control is down
                if ((action == 'keypress' && !e.metaKey && !e.ctrlKey) || _modifiersMatch(modifiers, callback.modifiers)) {

                    // when you bind a combination or sequence a second time it
                    // should overwrite the first one.  if a sequenceName or
                    // combination is specified in this call it does just that
                    //
                    // @todo make deleting its own method?
                    var deleteCombo = !sequenceName && callback.combo == combination;
                    var deleteSequence = sequenceName && callback.seq == sequenceName && callback.level == level;
                    if (deleteCombo || deleteSequence) {
                        self._callbacks[character].splice(i, 1);
                    }

                    matches.push(callback);
                }
            }

            return matches;
        }

        /**
         * actually calls the callback function
         *
         * if your callback function returns false this will use the jquery
         * convention - prevent default and stop propogation on the event
         *
         * @param {Function} callback
         * @param {Event} e
         * @returns void
         */
        function _fireCallback(callback, e, combo, sequence) {

            // if this event should not happen stop here
            if (self.stopCallback(e, e.target || e.srcElement, combo, sequence)) {
                return;
            }

            if (callback(e, combo) === false) {
                _preventDefault(e);
                _stopPropagation(e);
            }
        }

        /**
         * handles a character key event
         *
         * @param {string} character
         * @param {Array} modifiers
         * @param {Event} e
         * @returns void
         */
        self._handleKey = function(character, modifiers, e) {
            var callbacks = _getMatches(character, modifiers, e);
            var i;
            var doNotReset = {};
            var maxLevel = 0;
            var processedSequenceCallback = false;

            // Calculate the maxLevel for sequences so we can only execute the longest callback sequence
            for (i = 0; i < callbacks.length; ++i) {
                if (callbacks[i].seq) {
                    maxLevel = Math.max(maxLevel, callbacks[i].level);
                }
            }

            // loop through matching callbacks for this key event
            for (i = 0; i < callbacks.length; ++i) {

                // fire for all sequence callbacks
                // this is because if for example you have multiple sequences
                // bound such as "g i" and "g t" they both need to fire the
                // callback for matching g cause otherwise you can only ever
                // match the first one
                if (callbacks[i].seq) {

                    // only fire callbacks for the maxLevel to prevent
                    // subsequences from also firing
                    //
                    // for example 'a option b' should not cause 'option b' to fire
                    // even though 'option b' is part of the other sequence
                    //
                    // any sequences that do not match here will be discarded
                    // below by the _resetSequences call
                    if (callbacks[i].level != maxLevel) {
                        continue;
                    }

                    processedSequenceCallback = true;

                    // keep a list of which sequences were matches for later
                    doNotReset[callbacks[i].seq] = 1;
                    _fireCallback(callbacks[i].callback, e, callbacks[i].combo, callbacks[i].seq);
                    continue;
                }

                // if there were no sequence matches but we are still here
                // that means this is a regular match so we should fire that
                if (!processedSequenceCallback) {
                    _fireCallback(callbacks[i].callback, e, callbacks[i].combo);
                }
            }

            // if the key you pressed matches the type of sequence without
            // being a modifier (ie "keyup" or "keypress") then we should
            // reset all sequences that were not matched by this event
            //
            // this is so, for example, if you have the sequence "h a t" and you
            // type "h e a r t" it does not match.  in this case the "e" will
            // cause the sequence to reset
            //
            // modifier keys are ignored because you can have a sequence
            // that contains modifiers such as "enter ctrl+space" and in most
            // cases the modifier key will be pressed before the next key
            //
            // also if you have a sequence such as "ctrl+b a" then pressing the
            // "b" key will trigger a "keypress" and a "keydown"
            //
            // the "keydown" is expected when there is a modifier, but the
            // "keypress" ends up matching the _nextExpectedAction since it occurs
            // after and that causes the sequence to reset
            //
            // we ignore keypresses in a sequence that directly follow a keydown
            // for the same character
            var ignoreThisKeypress = e.type == 'keypress' && _ignoreNextKeypress;
            if (e.type == _nextExpectedAction && !_isModifier(character) && !ignoreThisKeypress) {
                _resetSequences(doNotReset);
            }

            _ignoreNextKeypress = processedSequenceCallback && e.type == 'keydown';
        };

        /**
         * handles a keydown event
         *
         * @param {Event} e
         * @returns void
         */
        function _handleKeyEvent(e) {

            // normalize e.which for key events
            // @see http://stackoverflow.com/questions/4285627/javascript-keycode-vs-charcode-utter-confusion
            if (typeof e.which !== 'number') {
                e.which = e.keyCode;
            }

            var character = _characterFromEvent(e);

            // no character found then stop
            if (!character) {
                return;
            }

            // need to use === for the character check because the character can be 0
            if (e.type == 'keyup' && _ignoreNextKeyup === character) {
                _ignoreNextKeyup = false;
                return;
            }

            self.handleKey(character, _eventModifiers(e), e);
        }

        /**
         * called to set a 1 second timeout on the specified sequence
         *
         * this is so after each key press in the sequence you have 1 second
         * to press the next key before you have to start over
         *
         * @returns void
         */
        function _resetSequenceTimer() {
            clearTimeout(_resetTimer);
            _resetTimer = setTimeout(_resetSequences, 1000);
        }

        /**
         * binds a key sequence to an event
         *
         * @param {string} combo - combo specified in bind call
         * @param {Array} keys
         * @param {Function} callback
         * @param {string=} action
         * @returns void
         */
        function _bindSequence(combo, keys, callback, action) {

            // start off by adding a sequence level record for this combination
            // and setting the level to 0
            _sequenceLevels[combo] = 0;

            /**
             * callback to increase the sequence level for this sequence and reset
             * all other sequences that were active
             *
             * @param {string} nextAction
             * @returns {Function}
             */
            function _increaseSequence(nextAction) {
                return function() {
                    _nextExpectedAction = nextAction;
                    ++_sequenceLevels[combo];
                    _resetSequenceTimer();
                };
            }

            /**
             * wraps the specified callback inside of another function in order
             * to reset all sequence counters as soon as this sequence is done
             *
             * @param {Event} e
             * @returns void
             */
            function _callbackAndReset(e) {
                _fireCallback(callback, e, combo);

                // we should ignore the next key up if the action is key down
                // or keypress.  this is so if you finish a sequence and
                // release the key the final key will not trigger a keyup
                if (action !== 'keyup') {
                    _ignoreNextKeyup = _characterFromEvent(e);
                }

                // weird race condition if a sequence ends with the key
                // another sequence begins with
                setTimeout(_resetSequences, 10);
            }

            // loop through keys one at a time and bind the appropriate callback
            // function.  for any key leading up to the final one it should
            // increase the sequence. after the final, it should reset all sequences
            //
            // if an action is specified in the original bind call then that will
            // be used throughout.  otherwise we will pass the action that the
            // next key in the sequence should match.  this allows a sequence
            // to mix and match keypress and keydown events depending on which
            // ones are better suited to the key provided
            for (var i = 0; i < keys.length; ++i) {
                var isFinal = i + 1 === keys.length;
                var wrappedCallback = isFinal ? _callbackAndReset : _increaseSequence(action || _getKeyInfo(keys[i + 1]).action);
                _bindSingle(keys[i], wrappedCallback, action, combo, i);
            }
        }

        /**
         * binds a single keyboard combination
         *
         * @param {string} combination
         * @param {Function} callback
         * @param {string=} action
         * @param {string=} sequenceName - name of sequence if part of sequence
         * @param {number=} level - what part of the sequence the command is
         * @returns void
         */
        function _bindSingle(combination, callback, action, sequenceName, level) {

            // store a direct mapped reference for use with Mousetrap.trigger
            self._directMap[combination + ':' + action] = callback;

            // make sure multiple spaces in a row become a single space
            combination = combination.replace(/\s+/g, ' ');

            var sequence = combination.split(' ');
            var info;

            // if this pattern is a sequence of keys then run through this method
            // to reprocess each pattern one key at a time
            if (sequence.length > 1) {
                _bindSequence(combination, sequence, callback, action);
                return;
            }

            info = _getKeyInfo(combination, action);

            // make sure to initialize array if this is the first time
            // a callback is added for this key
            self._callbacks[info.key] = self._callbacks[info.key] || [];

            // remove an existing match if there is one
            _getMatches(info.key, info.modifiers, {type: info.action}, sequenceName, combination, level);

            // add this call back to the array
            // if it is a sequence put it at the beginning
            // if not put it at the end
            //
            // this is important because the way these are processed expects
            // the sequence ones to come first
            self._callbacks[info.key][sequenceName ? 'unshift' : 'push']({
                callback: callback,
                modifiers: info.modifiers,
                action: info.action,
                seq: sequenceName,
                level: level,
                combo: combination
            });
        }

        /**
         * binds multiple combinations to the same callback
         *
         * @param {Array} combinations
         * @param {Function} callback
         * @param {string|undefined} action
         * @returns void
         */
        self._bindMultiple = function(combinations, callback, action) {
            for (var i = 0; i < combinations.length; ++i) {
                _bindSingle(combinations[i], callback, action);
            }
        };

        // start!
        _addEvent(targetElement, 'keypress', _handleKeyEvent);
        _addEvent(targetElement, 'keydown', _handleKeyEvent);
        _addEvent(targetElement, 'keyup', _handleKeyEvent);
    }

    /**
     * binds an event to mousetrap
     *
     * can be a single key, a combination of keys separated with +,
     * an array of keys, or a sequence of keys separated by spaces
     *
     * be sure to list the modifier keys first to make sure that the
     * correct key ends up getting bound (the last key in the pattern)
     *
     * @param {string|Array} keys
     * @param {Function} callback
     * @param {string=} action - 'keypress', 'keydown', or 'keyup'
     * @returns void
     */
    Mousetrap.prototype.bind = function(keys, callback, action) {
        var self = this;
        keys = keys instanceof Array ? keys : [keys];
        self._bindMultiple.call(self, keys, callback, action);
        return self;
    };

    /**
     * unbinds an event to mousetrap
     *
     * the unbinding sets the callback function of the specified key combo
     * to an empty function and deletes the corresponding key in the
     * _directMap dict.
     *
     * TODO: actually remove this from the _callbacks dictionary instead
     * of binding an empty function
     *
     * the keycombo+action has to be exactly the same as
     * it was defined in the bind method
     *
     * @param {string|Array} keys
     * @param {string} action
     * @returns void
     */
    Mousetrap.prototype.unbind = function(keys, action) {
        var self = this;
        return self.bind.call(self, keys, function() {}, action);
    };

    /**
     * triggers an event that has already been bound
     *
     * @param {string} keys
     * @param {string=} action
     * @returns void
     */
    Mousetrap.prototype.trigger = function(keys, action) {
        var self = this;
        if (self._directMap[keys + ':' + action]) {
            self._directMap[keys + ':' + action]({}, keys);
        }
        return self;
    };

    /**
     * resets the library back to its initial state.  this is useful
     * if you want to clear out the current keyboard shortcuts and bind
     * new ones - for example if you switch to another page
     *
     * @returns void
     */
    Mousetrap.prototype.reset = function() {
        var self = this;
        self._callbacks = {};
        self._directMap = {};
        return self;
    };

    /**
     * should we stop this event before firing off callbacks
     *
     * @param {Event} e
     * @param {Element} element
     * @return {boolean}
     */
    Mousetrap.prototype.stopCallback = function(e, element) {
        var self = this;

        // if the element has the class "mousetrap" then no need to stop
        if ((' ' + element.className + ' ').indexOf(' mousetrap ') > -1) {
            return false;
        }

        if (_belongsTo(element, self.target)) {
            return false;
        }

        // Events originating from a shadow DOM are re-targetted and `e.target` is the shadow host,
        // not the initial event target in the shadow tree. Note that not all events cross the
        // shadow boundary.
        // For shadow trees with `mode: 'open'`, the initial event target is the first element in
        // the event’s composed path. For shadow trees with `mode: 'closed'`, the initial event
        // target cannot be obtained.
        if ('composedPath' in e && typeof e.composedPath === 'function') {
            // For open shadow trees, update `element` so that the following check works.
            var initialEventTarget = e.composedPath()[0];
            if (initialEventTarget !== e.target) {
                element = initialEventTarget;
            }
        }

        // stop for input, select, and textarea
        return element.tagName == 'INPUT' || element.tagName == 'SELECT' || element.tagName == 'TEXTAREA' || element.isContentEditable;
    };

    /**
     * exposes _handleKey publicly so it can be overwritten by extensions
     */
    Mousetrap.prototype.handleKey = function() {
        var self = this;
        return self._handleKey.apply(self, arguments);
    };

    /**
     * allow custom key mappings
     */
    Mousetrap.addKeycodes = function(object) {
        for (var key in object) {
            if (object.hasOwnProperty(key)) {
                _MAP[key] = object[key];
            }
        }
        _REVERSE_MAP = null;
    };

    /**
     * Init the global mousetrap functions
     *
     * This method is needed to allow the global mousetrap functions to work
     * now that mousetrap is a constructor function.
     */
    Mousetrap.init = function() {
        var documentMousetrap = Mousetrap(document);
        for (var method in documentMousetrap) {
            if (method.charAt(0) !== '_') {
                Mousetrap[method] = (function(method) {
                    return function() {
                        return documentMousetrap[method].apply(documentMousetrap, arguments);
                    };
                } (method));
            }
        }
    };

    Mousetrap.init();

    // expose mousetrap to the global object
    window.Mousetrap = Mousetrap;

    // expose as a common js module
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = Mousetrap;
    }

    // expose mousetrap as an AMD module
    if (true) {
        !(__WEBPACK_AMD_DEFINE_RESULT__ = function() {
            return Mousetrap;
        }.call(exports, __webpack_require__, exports, module),
				__WEBPACK_AMD_DEFINE_RESULT__ !== undefined && (module.exports = __WEBPACK_AMD_DEFINE_RESULT__));
    }
}) (typeof window !== 'undefined' ? window : null, typeof  window !== 'undefined' ? document : null);


/***/ }),
/* 6 */
/***/ (function(module, exports) {

function EventEmitter(){this._events=this._events||{};this._maxListeners=this._maxListeners||undefined}module.exports=EventEmitter;EventEmitter.EventEmitter=EventEmitter;EventEmitter.prototype._events=undefined;EventEmitter.prototype._maxListeners=undefined;EventEmitter.defaultMaxListeners=10;EventEmitter.prototype.setMaxListeners=function(n){if(!isNumber(n)||n<0||isNaN(n))throw TypeError("n must be a positive number");this._maxListeners=n;return this};EventEmitter.prototype.emit=function(type){var er,handler,len,args,i,listeners;if(!this._events)this._events={};if(type==="error"){if(!this._events.error||isObject(this._events.error)&&!this._events.error.length){er=arguments[1];if(er instanceof Error){throw er}throw TypeError('Uncaught, unspecified "error" event.')}}handler=this._events[type];if(isUndefined(handler))return false;if(isFunction(handler)){switch(arguments.length){case 1:handler.call(this);break;case 2:handler.call(this,arguments[1]);break;case 3:handler.call(this,arguments[1],arguments[2]);break;default:len=arguments.length;args=new Array(len-1);for(i=1;i<len;i++)args[i-1]=arguments[i];handler.apply(this,args)}}else if(isObject(handler)){len=arguments.length;args=new Array(len-1);for(i=1;i<len;i++)args[i-1]=arguments[i];listeners=handler.slice();len=listeners.length;for(i=0;i<len;i++)listeners[i].apply(this,args)}return true};EventEmitter.prototype.addListener=function(type,listener){var m;if(!isFunction(listener))throw TypeError("listener must be a function");if(!this._events)this._events={};if(this._events.newListener)this.emit("newListener",type,isFunction(listener.listener)?listener.listener:listener);if(!this._events[type])this._events[type]=listener;else if(isObject(this._events[type]))this._events[type].push(listener);else this._events[type]=[this._events[type],listener];if(isObject(this._events[type])&&!this._events[type].warned){var m;if(!isUndefined(this._maxListeners)){m=this._maxListeners}else{m=EventEmitter.defaultMaxListeners}if(m&&m>0&&this._events[type].length>m){this._events[type].warned=true;console.error("(node) warning: possible EventEmitter memory "+"leak detected. %d listeners added. "+"Use emitter.setMaxListeners() to increase limit.",this._events[type].length);if(typeof console.trace==="function"){console.trace()}}}return this};EventEmitter.prototype.on=EventEmitter.prototype.addListener;EventEmitter.prototype.once=function(type,listener){if(!isFunction(listener))throw TypeError("listener must be a function");var fired=false;function g(){this.removeListener(type,g);if(!fired){fired=true;listener.apply(this,arguments)}}g.listener=listener;this.on(type,g);return this};EventEmitter.prototype.removeListener=function(type,listener){var list,position,length,i;if(!isFunction(listener))throw TypeError("listener must be a function");if(!this._events||!this._events[type])return this;list=this._events[type];length=list.length;position=-1;if(list===listener||isFunction(list.listener)&&list.listener===listener){delete this._events[type];if(this._events.removeListener)this.emit("removeListener",type,listener)}else if(isObject(list)){for(i=length;i-->0;){if(list[i]===listener||list[i].listener&&list[i].listener===listener){position=i;break}}if(position<0)return this;if(list.length===1){list.length=0;delete this._events[type]}else{list.splice(position,1)}if(this._events.removeListener)this.emit("removeListener",type,listener)}return this};EventEmitter.prototype.removeAllListeners=function(type){var key,listeners;if(!this._events)return this;if(!this._events.removeListener){if(arguments.length===0)this._events={};else if(this._events[type])delete this._events[type];return this}if(arguments.length===0){for(key in this._events){if(key==="removeListener")continue;this.removeAllListeners(key)}this.removeAllListeners("removeListener");this._events={};return this}listeners=this._events[type];if(isFunction(listeners)){this.removeListener(type,listeners)}else{while(listeners.length)this.removeListener(type,listeners[listeners.length-1])}delete this._events[type];return this};EventEmitter.prototype.listeners=function(type){var ret;if(!this._events||!this._events[type])ret=[];else if(isFunction(this._events[type]))ret=[this._events[type]];else ret=this._events[type].slice();return ret};EventEmitter.listenerCount=function(emitter,type){var ret;if(!emitter._events||!emitter._events[type])ret=0;else if(isFunction(emitter._events[type]))ret=1;else ret=emitter._events[type].length;return ret};function isFunction(arg){return typeof arg==="function"}function isNumber(arg){return typeof arg==="number"}function isObject(arg){return typeof arg==="object"&&arg!==null}function isUndefined(arg){return arg===void 0}

/***/ }),
/* 7 */
/***/ (function(module, exports, __webpack_require__) {

(function webpackUniversalModuleDefinition(root, factory) {
	if(true)
		module.exports = factory();
	else if(typeof define === 'function' && define.amd)
		define([], factory);
	else if(typeof exports === 'object')
		exports["katex"] = factory();
	else
		root["katex"] = factory();
})((typeof self !== 'undefined' ? self : this), function() {
return /******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, { enumerable: true, get: getter });
/******/ 		}
/******/ 	};
/******/
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = function(exports) {
/******/ 		if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 			Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 		}
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/
/******/ 	// create a fake namespace object
/******/ 	// mode & 1: value is a module id, require it
/******/ 	// mode & 2: merge all properties of value into the ns
/******/ 	// mode & 4: return value when already ns object
/******/ 	// mode & 8|1: behave like require
/******/ 	__webpack_require__.t = function(value, mode) {
/******/ 		if(mode & 1) value = __webpack_require__(value);
/******/ 		if(mode & 8) return value;
/******/ 		if((mode & 4) && typeof value === 'object' && value && value.__esModule) return value;
/******/ 		var ns = Object.create(null);
/******/ 		__webpack_require__.r(ns);
/******/ 		Object.defineProperty(ns, 'default', { enumerable: true, value: value });
/******/ 		if(mode & 2 && typeof value != 'string') for(var key in value) __webpack_require__.d(ns, key, function(key) { return value[key]; }.bind(null, key));
/******/ 		return ns;
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = 1);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ (function(module, exports, __webpack_require__) {

// extracted by mini-css-extract-plugin

/***/ }),
/* 1 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
__webpack_require__.r(__webpack_exports__);

// EXTERNAL MODULE: ./src/katex.less
var katex = __webpack_require__(0);

// CONCATENATED MODULE: ./src/SourceLocation.js
/**
 * Lexing or parsing positional information for error reporting.
 * This object is immutable.
 */
var SourceLocation =
/*#__PURE__*/
function () {
  // The + prefix indicates that these fields aren't writeable
  // Lexer holding the input string.
  // Start offset, zero-based inclusive.
  // End offset, zero-based exclusive.
  function SourceLocation(lexer, start, end) {
    this.lexer = void 0;
    this.start = void 0;
    this.end = void 0;
    this.lexer = lexer;
    this.start = start;
    this.end = end;
  }
  /**
   * Merges two `SourceLocation`s from location providers, given they are
   * provided in order of appearance.
   * - Returns the first one's location if only the first is provided.
   * - Returns a merged range of the first and the last if both are provided
   *   and their lexers match.
   * - Otherwise, returns null.
   */


  SourceLocation.range = function range(first, second) {
    if (!second) {
      return first && first.loc;
    } else if (!first || !first.loc || !second.loc || first.loc.lexer !== second.loc.lexer) {
      return null;
    } else {
      return new SourceLocation(first.loc.lexer, first.loc.start, second.loc.end);
    }
  };

  return SourceLocation;
}();


// CONCATENATED MODULE: ./src/Token.js

/**
 * Interface required to break circular dependency between Token, Lexer, and
 * ParseError.
 */

/**
 * The resulting token returned from `lex`.
 *
 * It consists of the token text plus some position information.
 * The position information is essentially a range in an input string,
 * but instead of referencing the bare input string, we refer to the lexer.
 * That way it is possible to attach extra metadata to the input string,
 * like for example a file name or similar.
 *
 * The position information is optional, so it is OK to construct synthetic
 * tokens if appropriate. Not providing available position information may
 * lead to degraded error reporting, though.
 */
var Token_Token =
/*#__PURE__*/
function () {
  function Token(text, // the text of this token
  loc) {
    this.text = void 0;
    this.loc = void 0;
    this.text = text;
    this.loc = loc;
  }
  /**
   * Given a pair of tokens (this and endToken), compute a `Token` encompassing
   * the whole input range enclosed by these two.
   */


  var _proto = Token.prototype;

  _proto.range = function range(endToken, // last token of the range, inclusive
  text) // the text of the newly constructed token
  {
    return new Token(text, SourceLocation.range(this, endToken));
  };

  return Token;
}();
// CONCATENATED MODULE: ./src/ParseError.js


/**
 * This is the ParseError class, which is the main error thrown by KaTeX
 * functions when something has gone wrong. This is used to distinguish internal
 * errors from errors in the expression that the user provided.
 *
 * If possible, a caller should provide a Token or ParseNode with information
 * about where in the source string the problem occurred.
 */
var ParseError = // Error position based on passed-in Token or ParseNode.
function ParseError(message, // The error message
token) // An object providing position information
{
  this.position = void 0;
  var error = "KaTeX parse error: " + message;
  var start;
  var loc = token && token.loc;

  if (loc && loc.start <= loc.end) {
    // If we have the input and a position, make the error a bit fancier
    // Get the input
    var input = loc.lexer.input; // Prepend some information

    start = loc.start;
    var end = loc.end;

    if (start === input.length) {
      error += " at end of input: ";
    } else {
      error += " at position " + (start + 1) + ": ";
    } // Underline token in question using combining underscores


    var underlined = input.slice(start, end).replace(/[^]/g, "$&\u0332"); // Extract some context from the input and add it to the error

    var left;

    if (start > 15) {
      left = "…" + input.slice(start - 15, start);
    } else {
      left = input.slice(0, start);
    }

    var right;

    if (end + 15 < input.length) {
      right = input.slice(end, end + 15) + "…";
    } else {
      right = input.slice(end);
    }

    error += left + underlined + right;
  } // Some hackery to make ParseError a prototype of Error
  // See http://stackoverflow.com/a/8460753


  var self = new Error(error);
  self.name = "ParseError"; // $FlowFixMe

  self.__proto__ = ParseError.prototype; // $FlowFixMe

  self.position = start;
  return self;
}; // $FlowFixMe More hackery


ParseError.prototype.__proto__ = Error.prototype;
/* harmony default export */ var src_ParseError = (ParseError);
// CONCATENATED MODULE: ./src/utils.js
/**
 * This file contains a list of utility functions which are useful in other
 * files.
 */

/**
 * Return whether an element is contained in a list
 */
var contains = function contains(list, elem) {
  return list.indexOf(elem) !== -1;
};
/**
 * Provide a default value if a setting is undefined
 * NOTE: Couldn't use `T` as the output type due to facebook/flow#5022.
 */


var deflt = function deflt(setting, defaultIfUndefined) {
  return setting === undefined ? defaultIfUndefined : setting;
}; // hyphenate and escape adapted from Facebook's React under Apache 2 license


var uppercase = /([A-Z])/g;

var hyphenate = function hyphenate(str) {
  return str.replace(uppercase, "-$1").toLowerCase();
};

var ESCAPE_LOOKUP = {
  "&": "&amp;",
  ">": "&gt;",
  "<": "&lt;",
  "\"": "&quot;",
  "'": "&#x27;"
};
var ESCAPE_REGEX = /[&><"']/g;
/**
 * Escapes text to prevent scripting attacks.
 */

function utils_escape(text) {
  return String(text).replace(ESCAPE_REGEX, function (match) {
    return ESCAPE_LOOKUP[match];
  });
}
/**
 * Sometimes we want to pull out the innermost element of a group. In most
 * cases, this will just be the group itself, but when ordgroups and colors have
 * a single element, we want to pull that out.
 */


var getBaseElem = function getBaseElem(group) {
  if (group.type === "ordgroup") {
    if (group.body.length === 1) {
      return getBaseElem(group.body[0]);
    } else {
      return group;
    }
  } else if (group.type === "color") {
    if (group.body.length === 1) {
      return getBaseElem(group.body[0]);
    } else {
      return group;
    }
  } else if (group.type === "font") {
    return getBaseElem(group.body);
  } else {
    return group;
  }
};
/**
 * TeXbook algorithms often reference "character boxes", which are simply groups
 * with a single character in them. To decide if something is a character box,
 * we find its innermost group, and see if it is a single character.
 */


var utils_isCharacterBox = function isCharacterBox(group) {
  var baseElem = getBaseElem(group); // These are all they types of groups which hold single characters

  return baseElem.type === "mathord" || baseElem.type === "textord" || baseElem.type === "atom";
};

var assert = function assert(value) {
  if (!value) {
    throw new Error('Expected non-null, but got ' + String(value));
  }

  return value;
};
/* harmony default export */ var utils = ({
  contains: contains,
  deflt: deflt,
  escape: utils_escape,
  hyphenate: hyphenate,
  getBaseElem: getBaseElem,
  isCharacterBox: utils_isCharacterBox
});
// CONCATENATED MODULE: ./src/Settings.js
/* eslint no-console:0 */

/**
 * This is a module for storing settings passed into KaTeX. It correctly handles
 * default settings.
 */




/**
 * The main Settings object
 *
 * The current options stored are:
 *  - displayMode: Whether the expression should be typeset as inline math
 *                 (false, the default), meaning that the math starts in
 *                 \textstyle and is placed in an inline-block); or as display
 *                 math (true), meaning that the math starts in \displaystyle
 *                 and is placed in a block with vertical margin.
 */
var Settings_Settings =
/*#__PURE__*/
function () {
  function Settings(options) {
    this.displayMode = void 0;
    this.leqno = void 0;
    this.fleqn = void 0;
    this.throwOnError = void 0;
    this.errorColor = void 0;
    this.macros = void 0;
    this.colorIsTextColor = void 0;
    this.strict = void 0;
    this.maxSize = void 0;
    this.maxExpand = void 0;
    this.allowedProtocols = void 0;
    // allow null options
    options = options || {};
    this.displayMode = utils.deflt(options.displayMode, false);
    this.leqno = utils.deflt(options.leqno, false);
    this.fleqn = utils.deflt(options.fleqn, false);
    this.throwOnError = utils.deflt(options.throwOnError, true);
    this.errorColor = utils.deflt(options.errorColor, "#cc0000");
    this.macros = options.macros || {};
    this.colorIsTextColor = utils.deflt(options.colorIsTextColor, false);
    this.strict = utils.deflt(options.strict, "warn");
    this.maxSize = Math.max(0, utils.deflt(options.maxSize, Infinity));
    this.maxExpand = Math.max(0, utils.deflt(options.maxExpand, 1000));
    this.allowedProtocols = utils.deflt(options.allowedProtocols, ["http", "https", "mailto", "_relative"]);
  }
  /**
   * Report nonstrict (non-LaTeX-compatible) input.
   * Can safely not be called if `this.strict` is false in JavaScript.
   */


  var _proto = Settings.prototype;

  _proto.reportNonstrict = function reportNonstrict(errorCode, errorMsg, token) {
    var strict = this.strict;

    if (typeof strict === "function") {
      // Allow return value of strict function to be boolean or string
      // (or null/undefined, meaning no further processing).
      strict = strict(errorCode, errorMsg, token);
    }

    if (!strict || strict === "ignore") {
      return;
    } else if (strict === true || strict === "error") {
      throw new src_ParseError("LaTeX-incompatible input and strict mode is set to 'error': " + (errorMsg + " [" + errorCode + "]"), token);
    } else if (strict === "warn") {
      typeof console !== "undefined" && console.warn("LaTeX-incompatible input and strict mode is set to 'warn': " + (errorMsg + " [" + errorCode + "]"));
    } else {
      // won't happen in type-safe code
      typeof console !== "undefined" && console.warn("LaTeX-incompatible input and strict mode is set to " + ("unrecognized '" + strict + "': " + errorMsg + " [" + errorCode + "]"));
    }
  }
  /**
   * Check whether to apply strict (LaTeX-adhering) behavior for unusual
   * input (like `\\`).  Unlike `nonstrict`, will not throw an error;
   * instead, "error" translates to a return value of `true`, while "ignore"
   * translates to a return value of `false`.  May still print a warning:
   * "warn" prints a warning and returns `false`.
   * This is for the second category of `errorCode`s listed in the README.
   */
  ;

  _proto.useStrictBehavior = function useStrictBehavior(errorCode, errorMsg, token) {
    var strict = this.strict;

    if (typeof strict === "function") {
      // Allow return value of strict function to be boolean or string
      // (or null/undefined, meaning no further processing).
      // But catch any exceptions thrown by function, treating them
      // like "error".
      try {
        strict = strict(errorCode, errorMsg, token);
      } catch (error) {
        strict = "error";
      }
    }

    if (!strict || strict === "ignore") {
      return false;
    } else if (strict === true || strict === "error") {
      return true;
    } else if (strict === "warn") {
      typeof console !== "undefined" && console.warn("LaTeX-incompatible input and strict mode is set to 'warn': " + (errorMsg + " [" + errorCode + "]"));
      return false;
    } else {
      // won't happen in type-safe code
      typeof console !== "undefined" && console.warn("LaTeX-incompatible input and strict mode is set to " + ("unrecognized '" + strict + "': " + errorMsg + " [" + errorCode + "]"));
      return false;
    }
  };

  return Settings;
}();

/* harmony default export */ var src_Settings = (Settings_Settings);
// CONCATENATED MODULE: ./src/Style.js
/**
 * This file contains information and classes for the various kinds of styles
 * used in TeX. It provides a generic `Style` class, which holds information
 * about a specific style. It then provides instances of all the different kinds
 * of styles possible, and provides functions to move between them and get
 * information about them.
 */

/**
 * The main style class. Contains a unique id for the style, a size (which is
 * the same for cramped and uncramped version of a style), and a cramped flag.
 */
var Style =
/*#__PURE__*/
function () {
  function Style(id, size, cramped) {
    this.id = void 0;
    this.size = void 0;
    this.cramped = void 0;
    this.id = id;
    this.size = size;
    this.cramped = cramped;
  }
  /**
   * Get the style of a superscript given a base in the current style.
   */


  var _proto = Style.prototype;

  _proto.sup = function sup() {
    return Style_styles[_sup[this.id]];
  }
  /**
   * Get the style of a subscript given a base in the current style.
   */
  ;

  _proto.sub = function sub() {
    return Style_styles[_sub[this.id]];
  }
  /**
   * Get the style of a fraction numerator given the fraction in the current
   * style.
   */
  ;

  _proto.fracNum = function fracNum() {
    return Style_styles[_fracNum[this.id]];
  }
  /**
   * Get the style of a fraction denominator given the fraction in the current
   * style.
   */
  ;

  _proto.fracDen = function fracDen() {
    return Style_styles[_fracDen[this.id]];
  }
  /**
   * Get the cramped version of a style (in particular, cramping a cramped style
   * doesn't change the style).
   */
  ;

  _proto.cramp = function cramp() {
    return Style_styles[_cramp[this.id]];
  }
  /**
   * Get a text or display version of this style.
   */
  ;

  _proto.text = function text() {
    return Style_styles[_text[this.id]];
  }
  /**
   * Return true if this style is tightly spaced (scriptstyle/scriptscriptstyle)
   */
  ;

  _proto.isTight = function isTight() {
    return this.size >= 2;
  };

  return Style;
}(); // Export an interface for type checking, but don't expose the implementation.
// This way, no more styles can be generated.


// IDs of the different styles
var D = 0;
var Dc = 1;
var T = 2;
var Tc = 3;
var S = 4;
var Sc = 5;
var SS = 6;
var SSc = 7; // Instances of the different styles

var Style_styles = [new Style(D, 0, false), new Style(Dc, 0, true), new Style(T, 1, false), new Style(Tc, 1, true), new Style(S, 2, false), new Style(Sc, 2, true), new Style(SS, 3, false), new Style(SSc, 3, true)]; // Lookup tables for switching from one style to another

var _sup = [S, Sc, S, Sc, SS, SSc, SS, SSc];
var _sub = [Sc, Sc, Sc, Sc, SSc, SSc, SSc, SSc];
var _fracNum = [T, Tc, S, Sc, SS, SSc, SS, SSc];
var _fracDen = [Tc, Tc, Sc, Sc, SSc, SSc, SSc, SSc];
var _cramp = [Dc, Dc, Tc, Tc, Sc, Sc, SSc, SSc];
var _text = [D, Dc, T, Tc, T, Tc, T, Tc]; // We only export some of the styles.

/* harmony default export */ var src_Style = ({
  DISPLAY: Style_styles[D],
  TEXT: Style_styles[T],
  SCRIPT: Style_styles[S],
  SCRIPTSCRIPT: Style_styles[SS]
});
// CONCATENATED MODULE: ./src/unicodeScripts.js
/*
 * This file defines the Unicode scripts and script families that we
 * support. To add new scripts or families, just add a new entry to the
 * scriptData array below. Adding scripts to the scriptData array allows
 * characters from that script to appear in \text{} environments.
 */

/**
 * Each script or script family has a name and an array of blocks.
 * Each block is an array of two numbers which specify the start and
 * end points (inclusive) of a block of Unicode codepoints.
 */

/**
 * Unicode block data for the families of scripts we support in \text{}.
 * Scripts only need to appear here if they do not have font metrics.
 */
var scriptData = [{
  // Latin characters beyond the Latin-1 characters we have metrics for.
  // Needed for Czech, Hungarian and Turkish text, for example.
  name: 'latin',
  blocks: [[0x0100, 0x024f], // Latin Extended-A and Latin Extended-B
  [0x0300, 0x036f]]
}, {
  // The Cyrillic script used by Russian and related languages.
  // A Cyrillic subset used to be supported as explicitly defined
  // symbols in symbols.js
  name: 'cyrillic',
  blocks: [[0x0400, 0x04ff]]
}, {
  // The Brahmic scripts of South and Southeast Asia
  // Devanagari (0900–097F)
  // Bengali (0980–09FF)
  // Gurmukhi (0A00–0A7F)
  // Gujarati (0A80–0AFF)
  // Oriya (0B00–0B7F)
  // Tamil (0B80–0BFF)
  // Telugu (0C00–0C7F)
  // Kannada (0C80–0CFF)
  // Malayalam (0D00–0D7F)
  // Sinhala (0D80–0DFF)
  // Thai (0E00–0E7F)
  // Lao (0E80–0EFF)
  // Tibetan (0F00–0FFF)
  // Myanmar (1000–109F)
  name: 'brahmic',
  blocks: [[0x0900, 0x109F]]
}, {
  name: 'georgian',
  blocks: [[0x10A0, 0x10ff]]
}, {
  // Chinese and Japanese.
  // The "k" in cjk is for Korean, but we've separated Korean out
  name: "cjk",
  blocks: [[0x3000, 0x30FF], // CJK symbols and punctuation, Hiragana, Katakana
  [0x4E00, 0x9FAF], // CJK ideograms
  [0xFF00, 0xFF60]]
}, {
  // Korean
  name: 'hangul',
  blocks: [[0xAC00, 0xD7AF]]
}];
/**
 * Given a codepoint, return the name of the script or script family
 * it is from, or null if it is not part of a known block
 */

function scriptFromCodepoint(codepoint) {
  for (var i = 0; i < scriptData.length; i++) {
    var script = scriptData[i];

    for (var _i = 0; _i < script.blocks.length; _i++) {
      var block = script.blocks[_i];

      if (codepoint >= block[0] && codepoint <= block[1]) {
        return script.name;
      }
    }
  }

  return null;
}
/**
 * A flattened version of all the supported blocks in a single array.
 * This is an optimization to make supportedCodepoint() fast.
 */

var allBlocks = [];
scriptData.forEach(function (s) {
  return s.blocks.forEach(function (b) {
    return allBlocks.push.apply(allBlocks, b);
  });
});
/**
 * Given a codepoint, return true if it falls within one of the
 * scripts or script families defined above and false otherwise.
 *
 * Micro benchmarks shows that this is faster than
 * /[\u3000-\u30FF\u4E00-\u9FAF\uFF00-\uFF60\uAC00-\uD7AF\u0900-\u109F]/.test()
 * in Firefox, Chrome and Node.
 */

function supportedCodepoint(codepoint) {
  for (var i = 0; i < allBlocks.length; i += 2) {
    if (codepoint >= allBlocks[i] && codepoint <= allBlocks[i + 1]) {
      return true;
    }
  }

  return false;
}
// CONCATENATED MODULE: ./src/svgGeometry.js
/**
 * This file provides support to domTree.js
 * It's a storehouse of path geometry for SVG images.
 */
// In all paths below, the viewBox-to-em scale is 1000:1.
var hLinePad = 80; // padding above a sqrt viniculum.

var svgGeometry_path = {
  // sqrtMain path geometry is from glyph U221A in the font KaTeX Main
  // All surds have 80 units padding above the viniculumn.
  sqrtMain: "M95," + (622 + hLinePad) + "c-2.7,0,-7.17,-2.7,-13.5,-8c-5.8,-5.3,-9.5,\n-10,-9.5,-14c0,-2,0.3,-3.3,1,-4c1.3,-2.7,23.83,-20.7,67.5,-54c44.2,-33.3,65.8,\n-50.3,66.5,-51c1.3,-1.3,3,-2,5,-2c4.7,0,8.7,3.3,12,10s173,378,173,378c0.7,0,\n35.3,-71,104,-213c68.7,-142,137.5,-285,206.5,-429c69,-144,104.5,-217.7,106.5,\n-221c5.3,-9.3,12,-14,20,-14H400000v40H845.2724s-225.272,467,-225.272,467\ns-235,486,-235,486c-2.7,4.7,-9,7,-19,7c-6,0,-10,-1,-12,-3s-194,-422,-194,-422\ns-65,47,-65,47z M834 " + hLinePad + "H400000v40H845z",
  // size1 is from glyph U221A in the font KaTeX_Size1-Regular
  sqrtSize1: "M263," + (601 + hLinePad) + "c0.7,0,18,39.7,52,119c34,79.3,68.167,\n158.7,102.5,238c34.3,79.3,51.8,119.3,52.5,120c340,-704.7,510.7,-1060.3,512,-1067\nc4.7,-7.3,11,-11,19,-11H40000v40H1012.3s-271.3,567,-271.3,567c-38.7,80.7,-84,\n175,-136,283c-52,108,-89.167,185.3,-111.5,232c-22.3,46.7,-33.8,70.3,-34.5,71\nc-4.7,4.7,-12.3,7,-23,7s-12,-1,-12,-1s-109,-253,-109,-253c-72.7,-168,-109.3,\n-252,-110,-252c-10.7,8,-22,16.7,-34,26c-22,17.3,-33.3,26,-34,26s-26,-26,-26,-26\ns76,-59,76,-59s76,-60,76,-60z M1001 " + hLinePad + "H40000v40H1012z",
  // size2 is from glyph U221A in the font KaTeX_Size2-Regular
  // The 80 units padding is most obvious here. Note start node at M1001 80.
  sqrtSize2: "M1001," + hLinePad + "H400000v40H1013.1s-83.4,268,-264.1,840c-180.7,\n572,-277,876.3,-289,913c-4.7,4.7,-12.7,7,-24,7s-12,0,-12,0c-1.3,-3.3,-3.7,-11.7,\n-7,-25c-35.3,-125.3,-106.7,-373.3,-214,-744c-10,12,-21,25,-33,39s-32,39,-32,39\nc-6,-5.3,-15,-14,-27,-26s25,-30,25,-30c26.7,-32.7,52,-63,76,-91s52,-60,52,-60\ns208,722,208,722c56,-175.3,126.3,-397.3,211,-666c84.7,-268.7,153.8,-488.2,207.5,\n-658.5c53.7,-170.3,84.5,-266.8,92.5,-289.5c4,-6.7,10,-10,18,-10z\nM1001 " + hLinePad + "H400000v40H1013z",
  // size3 is from glyph U221A in the font KaTeX_Size3-Regular
  sqrtSize3: "M424," + (2398 + hLinePad) + "c-1.3,-0.7,-38.5,-172,-111.5,-514c-73,\n-342,-109.8,-513.3,-110.5,-514c0,-2,-10.7,14.3,-32,49c-4.7,7.3,-9.8,15.7,-15.5,\n25c-5.7,9.3,-9.8,16,-12.5,20s-5,7,-5,7c-4,-3.3,-8.3,-7.7,-13,-13s-13,-13,-13,\n-13s76,-122,76,-122s77,-121,77,-121s209,968,209,968c0,-2,84.7,-361.7,254,-1079\nc169.3,-717.3,254.7,-1077.7,256,-1081c4,-6.7,10,-10,18,-10H400000v40H1014.6\ns-87.3,378.7,-272.6,1166c-185.3,787.3,-279.3,1182.3,-282,1185c-2,6,-10,9,-24,9\nc-8,0,-12,-0.7,-12,-2z M1001 " + hLinePad + "H400000v40H1014z",
  // size4 is from glyph U221A in the font KaTeX_Size4-Regular
  sqrtSize4: "M473," + (2713 + hLinePad) + "c339.3,-1799.3,509.3,-2700,510,-2702\nc3.3,-7.3,9.3,-11,18,-11H400000v40H1017.7s-90.5,478,-276.2,1466c-185.7,988,\n-279.5,1483,-281.5,1485c-2,6,-10,9,-24,9c-8,0,-12,-0.7,-12,-2c0,-1.3,-5.3,-32,\n-16,-92c-50.7,-293.3,-119.7,-693.3,-207,-1200c0,-1.3,-5.3,8.7,-16,30c-10.7,\n21.3,-21.3,42.7,-32,64s-16,33,-16,33s-26,-26,-26,-26s76,-153,76,-153s77,-151,\n77,-151c0.7,0.7,35.7,202,105,604c67.3,400.7,102,602.7,104,606z\nM1001 " + hLinePad + "H400000v40H1017z",
  // The doubleleftarrow geometry is from glyph U+21D0 in the font KaTeX Main
  doubleleftarrow: "M262 157\nl10-10c34-36 62.7-77 86-123 3.3-8 5-13.3 5-16 0-5.3-6.7-8-20-8-7.3\n 0-12.2.5-14.5 1.5-2.3 1-4.8 4.5-7.5 10.5-49.3 97.3-121.7 169.3-217 216-28\n 14-57.3 25-88 33-6.7 2-11 3.8-13 5.5-2 1.7-3 4.2-3 7.5s1 5.8 3 7.5\nc2 1.7 6.3 3.5 13 5.5 68 17.3 128.2 47.8 180.5 91.5 52.3 43.7 93.8 96.2 124.5\n 157.5 9.3 8 15.3 12.3 18 13h6c12-.7 18-4 18-10 0-2-1.7-7-5-15-23.3-46-52-87\n-86-123l-10-10h399738v-40H218c328 0 0 0 0 0l-10-8c-26.7-20-65.7-43-117-69 2.7\n-2 6-3.7 10-5 36.7-16 72.3-37.3 107-64l10-8h399782v-40z\nm8 0v40h399730v-40zm0 194v40h399730v-40z",
  // doublerightarrow is from glyph U+21D2 in font KaTeX Main
  doublerightarrow: "M399738 392l\n-10 10c-34 36-62.7 77-86 123-3.3 8-5 13.3-5 16 0 5.3 6.7 8 20 8 7.3 0 12.2-.5\n 14.5-1.5 2.3-1 4.8-4.5 7.5-10.5 49.3-97.3 121.7-169.3 217-216 28-14 57.3-25 88\n-33 6.7-2 11-3.8 13-5.5 2-1.7 3-4.2 3-7.5s-1-5.8-3-7.5c-2-1.7-6.3-3.5-13-5.5-68\n-17.3-128.2-47.8-180.5-91.5-52.3-43.7-93.8-96.2-124.5-157.5-9.3-8-15.3-12.3-18\n-13h-6c-12 .7-18 4-18 10 0 2 1.7 7 5 15 23.3 46 52 87 86 123l10 10H0v40h399782\nc-328 0 0 0 0 0l10 8c26.7 20 65.7 43 117 69-2.7 2-6 3.7-10 5-36.7 16-72.3 37.3\n-107 64l-10 8H0v40zM0 157v40h399730v-40zm0 194v40h399730v-40z",
  // leftarrow is from glyph U+2190 in font KaTeX Main
  leftarrow: "M400000 241H110l3-3c68.7-52.7 113.7-120\n 135-202 4-14.7 6-23 6-25 0-7.3-7-11-21-11-8 0-13.2.8-15.5 2.5-2.3 1.7-4.2 5.8\n-5.5 12.5-1.3 4.7-2.7 10.3-4 17-12 48.7-34.8 92-68.5 130S65.3 228.3 18 247\nc-10 4-16 7.7-18 11 0 8.7 6 14.3 18 17 47.3 18.7 87.8 47 121.5 85S196 441.3 208\n 490c.7 2 1.3 5 2 9s1.2 6.7 1.5 8c.3 1.3 1 3.3 2 6s2.2 4.5 3.5 5.5c1.3 1 3.3\n 1.8 6 2.5s6 1 10 1c14 0 21-3.7 21-11 0-2-2-10.3-6-25-20-79.3-65-146.7-135-202\n l-3-3h399890zM100 241v40h399900v-40z",
  // overbrace is from glyphs U+23A9/23A8/23A7 in font KaTeX_Size4-Regular
  leftbrace: "M6 548l-6-6v-35l6-11c56-104 135.3-181.3 238-232 57.3-28.7 117\n-45 179-50h399577v120H403c-43.3 7-81 15-113 26-100.7 33-179.7 91-237 174-2.7\n 5-6 9-10 13-.7 1-7.3 1-20 1H6z",
  leftbraceunder: "M0 6l6-6h17c12.688 0 19.313.3 20 1 4 4 7.313 8.3 10 13\n 35.313 51.3 80.813 93.8 136.5 127.5 55.688 33.7 117.188 55.8 184.5 66.5.688\n 0 2 .3 4 1 18.688 2.7 76 4.3 172 5h399450v120H429l-6-1c-124.688-8-235-61.7\n-331-161C60.687 138.7 32.312 99.3 7 54L0 41V6z",
  // overgroup is from the MnSymbol package (public domain)
  leftgroup: "M400000 80\nH435C64 80 168.3 229.4 21 260c-5.9 1.2-18 0-18 0-2 0-3-1-3-3v-38C76 61 257 0\n 435 0h399565z",
  leftgroupunder: "M400000 262\nH435C64 262 168.3 112.6 21 82c-5.9-1.2-18 0-18 0-2 0-3 1-3 3v38c76 158 257 219\n 435 219h399565z",
  // Harpoons are from glyph U+21BD in font KaTeX Main
  leftharpoon: "M0 267c.7 5.3 3 10 7 14h399993v-40H93c3.3\n-3.3 10.2-9.5 20.5-18.5s17.8-15.8 22.5-20.5c50.7-52 88-110.3 112-175 4-11.3 5\n-18.3 3-21-1.3-4-7.3-6-18-6-8 0-13 .7-15 2s-4.7 6.7-8 16c-42 98.7-107.3 174.7\n-196 228-6.7 4.7-10.7 8-12 10-1.3 2-2 5.7-2 11zm100-26v40h399900v-40z",
  leftharpoonplus: "M0 267c.7 5.3 3 10 7 14h399993v-40H93c3.3-3.3 10.2-9.5\n 20.5-18.5s17.8-15.8 22.5-20.5c50.7-52 88-110.3 112-175 4-11.3 5-18.3 3-21-1.3\n-4-7.3-6-18-6-8 0-13 .7-15 2s-4.7 6.7-8 16c-42 98.7-107.3 174.7-196 228-6.7 4.7\n-10.7 8-12 10-1.3 2-2 5.7-2 11zm100-26v40h399900v-40zM0 435v40h400000v-40z\nm0 0v40h400000v-40z",
  leftharpoondown: "M7 241c-4 4-6.333 8.667-7 14 0 5.333.667 9 2 11s5.333\n 5.333 12 10c90.667 54 156 130 196 228 3.333 10.667 6.333 16.333 9 17 2 .667 5\n 1 9 1h5c10.667 0 16.667-2 18-6 2-2.667 1-9.667-3-21-32-87.333-82.667-157.667\n-152-211l-3-3h399907v-40zM93 281 H400000 v-40L7 241z",
  leftharpoondownplus: "M7 435c-4 4-6.3 8.7-7 14 0 5.3.7 9 2 11s5.3 5.3 12\n 10c90.7 54 156 130 196 228 3.3 10.7 6.3 16.3 9 17 2 .7 5 1 9 1h5c10.7 0 16.7\n-2 18-6 2-2.7 1-9.7-3-21-32-87.3-82.7-157.7-152-211l-3-3h399907v-40H7zm93 0\nv40h399900v-40zM0 241v40h399900v-40zm0 0v40h399900v-40z",
  // hook is from glyph U+21A9 in font KaTeX Main
  lefthook: "M400000 281 H103s-33-11.2-61-33.5S0 197.3 0 164s14.2-61.2 42.5\n-83.5C70.8 58.2 104 47 142 47 c16.7 0 25 6.7 25 20 0 12-8.7 18.7-26 20-40 3.3\n-68.7 15.7-86 37-10 12-15 25.3-15 40 0 22.7 9.8 40.7 29.5 54 19.7 13.3 43.5 21\n 71.5 23h399859zM103 281v-40h399897v40z",
  leftlinesegment: "M40 281 V428 H0 V94 H40 V241 H400000 v40z\nM40 281 V428 H0 V94 H40 V241 H400000 v40z",
  leftmapsto: "M40 281 V448H0V74H40V241H400000v40z\nM40 281 V448H0V74H40V241H400000v40z",
  // tofrom is from glyph U+21C4 in font KaTeX AMS Regular
  leftToFrom: "M0 147h400000v40H0zm0 214c68 40 115.7 95.7 143 167h22c15.3 0 23\n-.3 23-1 0-1.3-5.3-13.7-16-37-18-35.3-41.3-69-70-101l-7-8h399905v-40H95l7-8\nc28.7-32 52-65.7 70-101 10.7-23.3 16-35.7 16-37 0-.7-7.7-1-23-1h-22C115.7 265.3\n 68 321 0 361zm0-174v-40h399900v40zm100 154v40h399900v-40z",
  longequal: "M0 50 h400000 v40H0z m0 194h40000v40H0z\nM0 50 h400000 v40H0z m0 194h40000v40H0z",
  midbrace: "M200428 334\nc-100.7-8.3-195.3-44-280-108-55.3-42-101.7-93-139-153l-9-14c-2.7 4-5.7 8.7-9 14\n-53.3 86.7-123.7 153-211 199-66.7 36-137.3 56.3-212 62H0V214h199568c178.3-11.7\n 311.7-78.3 403-201 6-8 9.7-12 11-12 .7-.7 6.7-1 18-1s17.3.3 18 1c1.3 0 5 4 11\n 12 44.7 59.3 101.3 106.3 170 141s145.3 54.3 229 60h199572v120z",
  midbraceunder: "M199572 214\nc100.7 8.3 195.3 44 280 108 55.3 42 101.7 93 139 153l9 14c2.7-4 5.7-8.7 9-14\n 53.3-86.7 123.7-153 211-199 66.7-36 137.3-56.3 212-62h199568v120H200432c-178.3\n 11.7-311.7 78.3-403 201-6 8-9.7 12-11 12-.7.7-6.7 1-18 1s-17.3-.3-18-1c-1.3 0\n-5-4-11-12-44.7-59.3-101.3-106.3-170-141s-145.3-54.3-229-60H0V214z",
  oiintSize1: "M512.6 71.6c272.6 0 320.3 106.8 320.3 178.2 0 70.8-47.7 177.6\n-320.3 177.6S193.1 320.6 193.1 249.8c0-71.4 46.9-178.2 319.5-178.2z\nm368.1 178.2c0-86.4-60.9-215.4-368.1-215.4-306.4 0-367.3 129-367.3 215.4 0 85.8\n60.9 214.8 367.3 214.8 307.2 0 368.1-129 368.1-214.8z",
  oiintSize2: "M757.8 100.1c384.7 0 451.1 137.6 451.1 230 0 91.3-66.4 228.8\n-451.1 228.8-386.3 0-452.7-137.5-452.7-228.8 0-92.4 66.4-230 452.7-230z\nm502.4 230c0-111.2-82.4-277.2-502.4-277.2s-504 166-504 277.2\nc0 110 84 276 504 276s502.4-166 502.4-276z",
  oiiintSize1: "M681.4 71.6c408.9 0 480.5 106.8 480.5 178.2 0 70.8-71.6 177.6\n-480.5 177.6S202.1 320.6 202.1 249.8c0-71.4 70.5-178.2 479.3-178.2z\nm525.8 178.2c0-86.4-86.8-215.4-525.7-215.4-437.9 0-524.7 129-524.7 215.4 0\n85.8 86.8 214.8 524.7 214.8 438.9 0 525.7-129 525.7-214.8z",
  oiiintSize2: "M1021.2 53c603.6 0 707.8 165.8 707.8 277.2 0 110-104.2 275.8\n-707.8 275.8-606 0-710.2-165.8-710.2-275.8C311 218.8 415.2 53 1021.2 53z\nm770.4 277.1c0-131.2-126.4-327.6-770.5-327.6S248.4 198.9 248.4 330.1\nc0 130 128.8 326.4 772.7 326.4s770.5-196.4 770.5-326.4z",
  rightarrow: "M0 241v40h399891c-47.3 35.3-84 78-110 128\n-16.7 32-27.7 63.7-33 95 0 1.3-.2 2.7-.5 4-.3 1.3-.5 2.3-.5 3 0 7.3 6.7 11 20\n 11 8 0 13.2-.8 15.5-2.5 2.3-1.7 4.2-5.5 5.5-11.5 2-13.3 5.7-27 11-41 14.7-44.7\n 39-84.5 73-119.5s73.7-60.2 119-75.5c6-2 9-5.7 9-11s-3-9-9-11c-45.3-15.3-85\n-40.5-119-75.5s-58.3-74.8-73-119.5c-4.7-14-8.3-27.3-11-40-1.3-6.7-3.2-10.8-5.5\n-12.5-2.3-1.7-7.5-2.5-15.5-2.5-14 0-21 3.7-21 11 0 2 2 10.3 6 25 20.7 83.3 67\n 151.7 139 205zm0 0v40h399900v-40z",
  rightbrace: "M400000 542l\n-6 6h-17c-12.7 0-19.3-.3-20-1-4-4-7.3-8.3-10-13-35.3-51.3-80.8-93.8-136.5-127.5\ns-117.2-55.8-184.5-66.5c-.7 0-2-.3-4-1-18.7-2.7-76-4.3-172-5H0V214h399571l6 1\nc124.7 8 235 61.7 331 161 31.3 33.3 59.7 72.7 85 118l7 13v35z",
  rightbraceunder: "M399994 0l6 6v35l-6 11c-56 104-135.3 181.3-238 232-57.3\n 28.7-117 45-179 50H-300V214h399897c43.3-7 81-15 113-26 100.7-33 179.7-91 237\n-174 2.7-5 6-9 10-13 .7-1 7.3-1 20-1h17z",
  rightgroup: "M0 80h399565c371 0 266.7 149.4 414 180 5.9 1.2 18 0 18 0 2 0\n 3-1 3-3v-38c-76-158-257-219-435-219H0z",
  rightgroupunder: "M0 262h399565c371 0 266.7-149.4 414-180 5.9-1.2 18 0 18\n 0 2 0 3 1 3 3v38c-76 158-257 219-435 219H0z",
  rightharpoon: "M0 241v40h399993c4.7-4.7 7-9.3 7-14 0-9.3\n-3.7-15.3-11-18-92.7-56.7-159-133.7-199-231-3.3-9.3-6-14.7-8-16-2-1.3-7-2-15-2\n-10.7 0-16.7 2-18 6-2 2.7-1 9.7 3 21 15.3 42 36.7 81.8 64 119.5 27.3 37.7 58\n 69.2 92 94.5zm0 0v40h399900v-40z",
  rightharpoonplus: "M0 241v40h399993c4.7-4.7 7-9.3 7-14 0-9.3-3.7-15.3-11\n-18-92.7-56.7-159-133.7-199-231-3.3-9.3-6-14.7-8-16-2-1.3-7-2-15-2-10.7 0-16.7\n 2-18 6-2 2.7-1 9.7 3 21 15.3 42 36.7 81.8 64 119.5 27.3 37.7 58 69.2 92 94.5z\nm0 0v40h399900v-40z m100 194v40h399900v-40zm0 0v40h399900v-40z",
  rightharpoondown: "M399747 511c0 7.3 6.7 11 20 11 8 0 13-.8 15-2.5s4.7-6.8\n 8-15.5c40-94 99.3-166.3 178-217 13.3-8 20.3-12.3 21-13 5.3-3.3 8.5-5.8 9.5\n-7.5 1-1.7 1.5-5.2 1.5-10.5s-2.3-10.3-7-15H0v40h399908c-34 25.3-64.7 57-92 95\n-27.3 38-48.7 77.7-64 119-3.3 8.7-5 14-5 16zM0 241v40h399900v-40z",
  rightharpoondownplus: "M399747 705c0 7.3 6.7 11 20 11 8 0 13-.8\n 15-2.5s4.7-6.8 8-15.5c40-94 99.3-166.3 178-217 13.3-8 20.3-12.3 21-13 5.3-3.3\n 8.5-5.8 9.5-7.5 1-1.7 1.5-5.2 1.5-10.5s-2.3-10.3-7-15H0v40h399908c-34 25.3\n-64.7 57-92 95-27.3 38-48.7 77.7-64 119-3.3 8.7-5 14-5 16zM0 435v40h399900v-40z\nm0-194v40h400000v-40zm0 0v40h400000v-40z",
  righthook: "M399859 241c-764 0 0 0 0 0 40-3.3 68.7-15.7 86-37 10-12 15-25.3\n 15-40 0-22.7-9.8-40.7-29.5-54-19.7-13.3-43.5-21-71.5-23-17.3-1.3-26-8-26-20 0\n-13.3 8.7-20 26-20 38 0 71 11.2 99 33.5 0 0 7 5.6 21 16.7 14 11.2 21 33.5 21\n 66.8s-14 61.2-42 83.5c-28 22.3-61 33.5-99 33.5L0 241z M0 281v-40h399859v40z",
  rightlinesegment: "M399960 241 V94 h40 V428 h-40 V281 H0 v-40z\nM399960 241 V94 h40 V428 h-40 V281 H0 v-40z",
  rightToFrom: "M400000 167c-70.7-42-118-97.7-142-167h-23c-15.3 0-23 .3-23\n 1 0 1.3 5.3 13.7 16 37 18 35.3 41.3 69 70 101l7 8H0v40h399905l-7 8c-28.7 32\n-52 65.7-70 101-10.7 23.3-16 35.7-16 37 0 .7 7.7 1 23 1h23c24-69.3 71.3-125 142\n-167z M100 147v40h399900v-40zM0 341v40h399900v-40z",
  // twoheadleftarrow is from glyph U+219E in font KaTeX AMS Regular
  twoheadleftarrow: "M0 167c68 40\n 115.7 95.7 143 167h22c15.3 0 23-.3 23-1 0-1.3-5.3-13.7-16-37-18-35.3-41.3-69\n-70-101l-7-8h125l9 7c50.7 39.3 85 86 103 140h46c0-4.7-6.3-18.7-19-42-18-35.3\n-40-67.3-66-96l-9-9h399716v-40H284l9-9c26-28.7 48-60.7 66-96 12.7-23.333 19\n-37.333 19-42h-46c-18 54-52.3 100.7-103 140l-9 7H95l7-8c28.7-32 52-65.7 70-101\n 10.7-23.333 16-35.7 16-37 0-.7-7.7-1-23-1h-22C115.7 71.3 68 127 0 167z",
  twoheadrightarrow: "M400000 167\nc-68-40-115.7-95.7-143-167h-22c-15.3 0-23 .3-23 1 0 1.3 5.3 13.7 16 37 18 35.3\n 41.3 69 70 101l7 8h-125l-9-7c-50.7-39.3-85-86-103-140h-46c0 4.7 6.3 18.7 19 42\n 18 35.3 40 67.3 66 96l9 9H0v40h399716l-9 9c-26 28.7-48 60.7-66 96-12.7 23.333\n-19 37.333-19 42h46c18-54 52.3-100.7 103-140l9-7h125l-7 8c-28.7 32-52 65.7-70\n 101-10.7 23.333-16 35.7-16 37 0 .7 7.7 1 23 1h22c27.3-71.3 75-127 143-167z",
  // tilde1 is a modified version of a glyph from the MnSymbol package
  tilde1: "M200 55.538c-77 0-168 73.953-177 73.953-3 0-7\n-2.175-9-5.437L2 97c-1-2-2-4-2-6 0-4 2-7 5-9l20-12C116 12 171 0 207 0c86 0\n 114 68 191 68 78 0 168-68 177-68 4 0 7 2 9 5l12 19c1 2.175 2 4.35 2 6.525 0\n 4.35-2 7.613-5 9.788l-19 13.05c-92 63.077-116.937 75.308-183 76.128\n-68.267.847-113-73.952-191-73.952z",
  // ditto tilde2, tilde3, & tilde4
  tilde2: "M344 55.266c-142 0-300.638 81.316-311.5 86.418\n-8.01 3.762-22.5 10.91-23.5 5.562L1 120c-1-2-1-3-1-4 0-5 3-9 8-10l18.4-9C160.9\n 31.9 283 0 358 0c148 0 188 122 331 122s314-97 326-97c4 0 8 2 10 7l7 21.114\nc1 2.14 1 3.21 1 4.28 0 5.347-3 9.626-7 10.696l-22.3 12.622C852.6 158.372 751\n 181.476 676 181.476c-149 0-189-126.21-332-126.21z",
  tilde3: "M786 59C457 59 32 175.242 13 175.242c-6 0-10-3.457\n-11-10.37L.15 138c-1-7 3-12 10-13l19.2-6.4C378.4 40.7 634.3 0 804.3 0c337 0\n 411.8 157 746.8 157 328 0 754-112 773-112 5 0 10 3 11 9l1 14.075c1 8.066-.697\n 16.595-6.697 17.492l-21.052 7.31c-367.9 98.146-609.15 122.696-778.15 122.696\n -338 0-409-156.573-744-156.573z",
  tilde4: "M786 58C457 58 32 177.487 13 177.487c-6 0-10-3.345\n-11-10.035L.15 143c-1-7 3-12 10-13l22-6.7C381.2 35 637.15 0 807.15 0c337 0 409\n 177 744 177 328 0 754-127 773-127 5 0 10 3 11 9l1 14.794c1 7.805-3 13.38-9\n 14.495l-20.7 5.574c-366.85 99.79-607.3 139.372-776.3 139.372-338 0-409\n -175.236-744-175.236z",
  // vec is from glyph U+20D7 in font KaTeX Main
  vec: "M377 20c0-5.333 1.833-10 5.5-14S391 0 397 0c4.667 0 8.667 1.667 12 5\n3.333 2.667 6.667 9 10 19 6.667 24.667 20.333 43.667 41 57 7.333 4.667 11\n10.667 11 18 0 6-1 10-3 12s-6.667 5-14 9c-28.667 14.667-53.667 35.667-75 63\n-1.333 1.333-3.167 3.5-5.5 6.5s-4 4.833-5 5.5c-1 .667-2.5 1.333-4.5 2s-4.333 1\n-7 1c-4.667 0-9.167-1.833-13.5-5.5S337 184 337 178c0-12.667 15.667-32.333 47-59\nH213l-171-1c-8.667-6-13-12.333-13-19 0-4.667 4.333-11.333 13-20h359\nc-16-25.333-24-45-24-59z",
  // widehat1 is a modified version of a glyph from the MnSymbol package
  widehat1: "M529 0h5l519 115c5 1 9 5 9 10 0 1-1 2-1 3l-4 22\nc-1 5-5 9-11 9h-2L532 67 19 159h-2c-5 0-9-4-11-9l-5-22c-1-6 2-12 8-13z",
  // ditto widehat2, widehat3, & widehat4
  widehat2: "M1181 0h2l1171 176c6 0 10 5 10 11l-2 23c-1 6-5 10\n-11 10h-1L1182 67 15 220h-1c-6 0-10-4-11-10l-2-23c-1-6 4-11 10-11z",
  widehat3: "M1181 0h2l1171 236c6 0 10 5 10 11l-2 23c-1 6-5 10\n-11 10h-1L1182 67 15 280h-1c-6 0-10-4-11-10l-2-23c-1-6 4-11 10-11z",
  widehat4: "M1181 0h2l1171 296c6 0 10 5 10 11l-2 23c-1 6-5 10\n-11 10h-1L1182 67 15 340h-1c-6 0-10-4-11-10l-2-23c-1-6 4-11 10-11z",
  // widecheck paths are all inverted versions of widehat
  widecheck1: "M529,159h5l519,-115c5,-1,9,-5,9,-10c0,-1,-1,-2,-1,-3l-4,-22c-1,\n-5,-5,-9,-11,-9h-2l-512,92l-513,-92h-2c-5,0,-9,4,-11,9l-5,22c-1,6,2,12,8,13z",
  widecheck2: "M1181,220h2l1171,-176c6,0,10,-5,10,-11l-2,-23c-1,-6,-5,-10,\n-11,-10h-1l-1168,153l-1167,-153h-1c-6,0,-10,4,-11,10l-2,23c-1,6,4,11,10,11z",
  widecheck3: "M1181,280h2l1171,-236c6,0,10,-5,10,-11l-2,-23c-1,-6,-5,-10,\n-11,-10h-1l-1168,213l-1167,-213h-1c-6,0,-10,4,-11,10l-2,23c-1,6,4,11,10,11z",
  widecheck4: "M1181,340h2l1171,-296c6,0,10,-5,10,-11l-2,-23c-1,-6,-5,-10,\n-11,-10h-1l-1168,273l-1167,-273h-1c-6,0,-10,4,-11,10l-2,23c-1,6,4,11,10,11z",
  // The next ten paths support reaction arrows from the mhchem package.
  // Arrows for \ce{<-->} are offset from xAxis by 0.22ex, per mhchem in LaTeX
  // baraboveleftarrow is mostly from from glyph U+2190 in font KaTeX Main
  baraboveleftarrow: "M400000 620h-399890l3 -3c68.7 -52.7 113.7 -120 135 -202\nc4 -14.7 6 -23 6 -25c0 -7.3 -7 -11 -21 -11c-8 0 -13.2 0.8 -15.5 2.5\nc-2.3 1.7 -4.2 5.8 -5.5 12.5c-1.3 4.7 -2.7 10.3 -4 17c-12 48.7 -34.8 92 -68.5 130\ns-74.2 66.3 -121.5 85c-10 4 -16 7.7 -18 11c0 8.7 6 14.3 18 17c47.3 18.7 87.8 47\n121.5 85s56.5 81.3 68.5 130c0.7 2 1.3 5 2 9s1.2 6.7 1.5 8c0.3 1.3 1 3.3 2 6\ns2.2 4.5 3.5 5.5c1.3 1 3.3 1.8 6 2.5s6 1 10 1c14 0 21 -3.7 21 -11\nc0 -2 -2 -10.3 -6 -25c-20 -79.3 -65 -146.7 -135 -202l-3 -3h399890z\nM100 620v40h399900v-40z M0 241v40h399900v-40zM0 241v40h399900v-40z",
  // rightarrowabovebar is mostly from glyph U+2192, KaTeX Main
  rightarrowabovebar: "M0 241v40h399891c-47.3 35.3-84 78-110 128-16.7 32\n-27.7 63.7-33 95 0 1.3-.2 2.7-.5 4-.3 1.3-.5 2.3-.5 3 0 7.3 6.7 11 20 11 8 0\n13.2-.8 15.5-2.5 2.3-1.7 4.2-5.5 5.5-11.5 2-13.3 5.7-27 11-41 14.7-44.7 39\n-84.5 73-119.5s73.7-60.2 119-75.5c6-2 9-5.7 9-11s-3-9-9-11c-45.3-15.3-85-40.5\n-119-75.5s-58.3-74.8-73-119.5c-4.7-14-8.3-27.3-11-40-1.3-6.7-3.2-10.8-5.5\n-12.5-2.3-1.7-7.5-2.5-15.5-2.5-14 0-21 3.7-21 11 0 2 2 10.3 6 25 20.7 83.3 67\n151.7 139 205zm96 379h399894v40H0zm0 0h399904v40H0z",
  // The short left harpoon has 0.5em (i.e. 500 units) kern on the left end.
  // Ref from mhchem.sty: \rlap{\raisebox{-.22ex}{$\kern0.5em
  baraboveshortleftharpoon: "M507,435c-4,4,-6.3,8.7,-7,14c0,5.3,0.7,9,2,11\nc1.3,2,5.3,5.3,12,10c90.7,54,156,130,196,228c3.3,10.7,6.3,16.3,9,17\nc2,0.7,5,1,9,1c0,0,5,0,5,0c10.7,0,16.7,-2,18,-6c2,-2.7,1,-9.7,-3,-21\nc-32,-87.3,-82.7,-157.7,-152,-211c0,0,-3,-3,-3,-3l399351,0l0,-40\nc-398570,0,-399437,0,-399437,0z M593 435 v40 H399500 v-40z\nM0 281 v-40 H399908 v40z M0 281 v-40 H399908 v40z",
  rightharpoonaboveshortbar: "M0,241 l0,40c399126,0,399993,0,399993,0\nc4.7,-4.7,7,-9.3,7,-14c0,-9.3,-3.7,-15.3,-11,-18c-92.7,-56.7,-159,-133.7,-199,\n-231c-3.3,-9.3,-6,-14.7,-8,-16c-2,-1.3,-7,-2,-15,-2c-10.7,0,-16.7,2,-18,6\nc-2,2.7,-1,9.7,3,21c15.3,42,36.7,81.8,64,119.5c27.3,37.7,58,69.2,92,94.5z\nM0 241 v40 H399908 v-40z M0 475 v-40 H399500 v40z M0 475 v-40 H399500 v40z",
  shortbaraboveleftharpoon: "M7,435c-4,4,-6.3,8.7,-7,14c0,5.3,0.7,9,2,11\nc1.3,2,5.3,5.3,12,10c90.7,54,156,130,196,228c3.3,10.7,6.3,16.3,9,17c2,0.7,5,1,9,\n1c0,0,5,0,5,0c10.7,0,16.7,-2,18,-6c2,-2.7,1,-9.7,-3,-21c-32,-87.3,-82.7,-157.7,\n-152,-211c0,0,-3,-3,-3,-3l399907,0l0,-40c-399126,0,-399993,0,-399993,0z\nM93 435 v40 H400000 v-40z M500 241 v40 H400000 v-40z M500 241 v40 H400000 v-40z",
  shortrightharpoonabovebar: "M53,241l0,40c398570,0,399437,0,399437,0\nc4.7,-4.7,7,-9.3,7,-14c0,-9.3,-3.7,-15.3,-11,-18c-92.7,-56.7,-159,-133.7,-199,\n-231c-3.3,-9.3,-6,-14.7,-8,-16c-2,-1.3,-7,-2,-15,-2c-10.7,0,-16.7,2,-18,6\nc-2,2.7,-1,9.7,3,21c15.3,42,36.7,81.8,64,119.5c27.3,37.7,58,69.2,92,94.5z\nM500 241 v40 H399408 v-40z M500 435 v40 H400000 v-40z"
};
/* harmony default export */ var svgGeometry = ({
  path: svgGeometry_path
});
// CONCATENATED MODULE: ./src/tree.js


/**
 * This node represents a document fragment, which contains elements, but when
 * placed into the DOM doesn't have any representation itself. It only contains
 * children and doesn't have any DOM node properties.
 */
var tree_DocumentFragment =
/*#__PURE__*/
function () {
  // HtmlDomNode
  // Never used; needed for satisfying interface.
  function DocumentFragment(children) {
    this.children = void 0;
    this.classes = void 0;
    this.height = void 0;
    this.depth = void 0;
    this.maxFontSize = void 0;
    this.style = void 0;
    this.children = children;
    this.classes = [];
    this.height = 0;
    this.depth = 0;
    this.maxFontSize = 0;
    this.style = {};
  }

  var _proto = DocumentFragment.prototype;

  _proto.hasClass = function hasClass(className) {
    return utils.contains(this.classes, className);
  }
  /** Convert the fragment into a node. */
  ;

  _proto.toNode = function toNode() {
    var frag = document.createDocumentFragment();

    for (var i = 0; i < this.children.length; i++) {
      frag.appendChild(this.children[i].toNode());
    }

    return frag;
  }
  /** Convert the fragment into HTML markup. */
  ;

  _proto.toMarkup = function toMarkup() {
    var markup = ""; // Simply concatenate the markup for the children together.

    for (var i = 0; i < this.children.length; i++) {
      markup += this.children[i].toMarkup();
    }

    return markup;
  }
  /**
   * Converts the math node into a string, similar to innerText. Applies to
   * MathDomNode's only.
   */
  ;

  _proto.toText = function toText() {
    // To avoid this, we would subclass documentFragment separately for
    // MathML, but polyfills for subclassing is expensive per PR 1469.
    // $FlowFixMe: Only works for ChildType = MathDomNode.
    var toText = function toText(child) {
      return child.toText();
    };

    return this.children.map(toText).join("");
  };

  return DocumentFragment;
}();
// CONCATENATED MODULE: ./src/domTree.js
/**
 * These objects store the data about the DOM nodes we create, as well as some
 * extra data. They can then be transformed into real DOM nodes with the
 * `toNode` function or HTML markup using `toMarkup`. They are useful for both
 * storing extra properties on the nodes, as well as providing a way to easily
 * work with the DOM.
 *
 * Similar functions for working with MathML nodes exist in mathMLTree.js.
 *
 * TODO: refactor `span` and `anchor` into common superclass when
 * target environments support class inheritance
 */





/**
 * Create an HTML className based on a list of classes. In addition to joining
 * with spaces, we also remove empty classes.
 */
var createClass = function createClass(classes) {
  return classes.filter(function (cls) {
    return cls;
  }).join(" ");
};

var initNode = function initNode(classes, options, style) {
  this.classes = classes || [];
  this.attributes = {};
  this.height = 0;
  this.depth = 0;
  this.maxFontSize = 0;
  this.style = style || {};

  if (options) {
    if (options.style.isTight()) {
      this.classes.push("mtight");
    }

    var color = options.getColor();

    if (color) {
      this.style.color = color;
    }
  }
};
/**
 * Convert into an HTML node
 */


var _toNode = function toNode(tagName) {
  var node = document.createElement(tagName); // Apply the class

  node.className = createClass(this.classes); // Apply inline styles

  for (var style in this.style) {
    if (this.style.hasOwnProperty(style)) {
      // $FlowFixMe Flow doesn't seem to understand span.style's type.
      node.style[style] = this.style[style];
    }
  } // Apply attributes


  for (var attr in this.attributes) {
    if (this.attributes.hasOwnProperty(attr)) {
      node.setAttribute(attr, this.attributes[attr]);
    }
  } // Append the children, also as HTML nodes


  for (var i = 0; i < this.children.length; i++) {
    node.appendChild(this.children[i].toNode());
  }

  return node;
};
/**
 * Convert into an HTML markup string
 */


var _toMarkup = function toMarkup(tagName) {
  var markup = "<" + tagName; // Add the class

  if (this.classes.length) {
    markup += " class=\"" + utils.escape(createClass(this.classes)) + "\"";
  }

  var styles = ""; // Add the styles, after hyphenation

  for (var style in this.style) {
    if (this.style.hasOwnProperty(style)) {
      styles += utils.hyphenate(style) + ":" + this.style[style] + ";";
    }
  }

  if (styles) {
    markup += " style=\"" + utils.escape(styles) + "\"";
  } // Add the attributes


  for (var attr in this.attributes) {
    if (this.attributes.hasOwnProperty(attr)) {
      markup += " " + attr + "=\"" + utils.escape(this.attributes[attr]) + "\"";
    }
  }

  markup += ">"; // Add the markup of the children, also as markup

  for (var i = 0; i < this.children.length; i++) {
    markup += this.children[i].toMarkup();
  }

  markup += "</" + tagName + ">";
  return markup;
}; // Making the type below exact with all optional fields doesn't work due to
// - https://github.com/facebook/flow/issues/4582
// - https://github.com/facebook/flow/issues/5688
// However, since *all* fields are optional, $Shape<> works as suggested in 5688
// above.
// This type does not include all CSS properties. Additional properties should
// be added as needed.


/**
 * This node represents a span node, with a className, a list of children, and
 * an inline style. It also contains information about its height, depth, and
 * maxFontSize.
 *
 * Represents two types with different uses: SvgSpan to wrap an SVG and DomSpan
 * otherwise. This typesafety is important when HTML builders access a span's
 * children.
 */
var domTree_Span =
/*#__PURE__*/
function () {
  function Span(classes, children, options, style) {
    this.children = void 0;
    this.attributes = void 0;
    this.classes = void 0;
    this.height = void 0;
    this.depth = void 0;
    this.width = void 0;
    this.maxFontSize = void 0;
    this.style = void 0;
    initNode.call(this, classes, options, style);
    this.children = children || [];
  }
  /**
   * Sets an arbitrary attribute on the span. Warning: use this wisely. Not
   * all browsers support attributes the same, and having too many custom
   * attributes is probably bad.
   */


  var _proto = Span.prototype;

  _proto.setAttribute = function setAttribute(attribute, value) {
    this.attributes[attribute] = value;
  };

  _proto.hasClass = function hasClass(className) {
    return utils.contains(this.classes, className);
  };

  _proto.toNode = function toNode() {
    return _toNode.call(this, "span");
  };

  _proto.toMarkup = function toMarkup() {
    return _toMarkup.call(this, "span");
  };

  return Span;
}();
/**
 * This node represents an anchor (<a>) element with a hyperlink.  See `span`
 * for further details.
 */

var domTree_Anchor =
/*#__PURE__*/
function () {
  function Anchor(href, classes, children, options) {
    this.children = void 0;
    this.attributes = void 0;
    this.classes = void 0;
    this.height = void 0;
    this.depth = void 0;
    this.maxFontSize = void 0;
    this.style = void 0;
    initNode.call(this, classes, options);
    this.children = children || [];
    this.setAttribute('href', href);
  }

  var _proto2 = Anchor.prototype;

  _proto2.setAttribute = function setAttribute(attribute, value) {
    this.attributes[attribute] = value;
  };

  _proto2.hasClass = function hasClass(className) {
    return utils.contains(this.classes, className);
  };

  _proto2.toNode = function toNode() {
    return _toNode.call(this, "a");
  };

  _proto2.toMarkup = function toMarkup() {
    return _toMarkup.call(this, "a");
  };

  return Anchor;
}();
/**
 * This node represents an image embed (<img>) element.
 */

var domTree_Img =
/*#__PURE__*/
function () {
  function Img(src, alt, style) {
    this.src = void 0;
    this.alt = void 0;
    this.classes = void 0;
    this.height = void 0;
    this.depth = void 0;
    this.maxFontSize = void 0;
    this.style = void 0;
    this.alt = alt;
    this.src = src;
    this.classes = ["mord"];
    this.style = style;
  }

  var _proto3 = Img.prototype;

  _proto3.hasClass = function hasClass(className) {
    return utils.contains(this.classes, className);
  };

  _proto3.toNode = function toNode() {
    var node = document.createElement("img");
    node.src = this.src;
    node.alt = this.alt;
    node.className = "mord"; // Apply inline styles

    for (var style in this.style) {
      if (this.style.hasOwnProperty(style)) {
        // $FlowFixMe
        node.style[style] = this.style[style];
      }
    }

    return node;
  };

  _proto3.toMarkup = function toMarkup() {
    var markup = "<img  src='" + this.src + " 'alt='" + this.alt + "' "; // Add the styles, after hyphenation

    var styles = "";

    for (var style in this.style) {
      if (this.style.hasOwnProperty(style)) {
        styles += utils.hyphenate(style) + ":" + this.style[style] + ";";
      }
    }

    if (styles) {
      markup += " style=\"" + utils.escape(styles) + "\"";
    }

    markup += "'/>";
    return markup;
  };

  return Img;
}();
var iCombinations = {
  'î': "\u0131\u0302",
  'ï': "\u0131\u0308",
  'í': "\u0131\u0301",
  // 'ī': '\u0131\u0304', // enable when we add Extended Latin
  'ì': "\u0131\u0300"
};
/**
 * A symbol node contains information about a single symbol. It either renders
 * to a single text node, or a span with a single text node in it, depending on
 * whether it has CSS classes, styles, or needs italic correction.
 */

var domTree_SymbolNode =
/*#__PURE__*/
function () {
  function SymbolNode(text, height, depth, italic, skew, width, classes, style) {
    this.text = void 0;
    this.height = void 0;
    this.depth = void 0;
    this.italic = void 0;
    this.skew = void 0;
    this.width = void 0;
    this.maxFontSize = void 0;
    this.classes = void 0;
    this.style = void 0;
    this.text = text;
    this.height = height || 0;
    this.depth = depth || 0;
    this.italic = italic || 0;
    this.skew = skew || 0;
    this.width = width || 0;
    this.classes = classes || [];
    this.style = style || {};
    this.maxFontSize = 0; // Mark text from non-Latin scripts with specific classes so that we
    // can specify which fonts to use.  This allows us to render these
    // characters with a serif font in situations where the browser would
    // either default to a sans serif or render a placeholder character.
    // We use CSS class names like cjk_fallback, hangul_fallback and
    // brahmic_fallback. See ./unicodeScripts.js for the set of possible
    // script names

    var script = scriptFromCodepoint(this.text.charCodeAt(0));

    if (script) {
      this.classes.push(script + "_fallback");
    }

    if (/[îïíì]/.test(this.text)) {
      // add ī when we add Extended Latin
      this.text = iCombinations[this.text];
    }
  }

  var _proto4 = SymbolNode.prototype;

  _proto4.hasClass = function hasClass(className) {
    return utils.contains(this.classes, className);
  }
  /**
   * Creates a text node or span from a symbol node. Note that a span is only
   * created if it is needed.
   */
  ;

  _proto4.toNode = function toNode() {
    var node = document.createTextNode(this.text);
    var span = null;

    if (this.italic > 0) {
      span = document.createElement("span");
      span.style.marginRight = this.italic + "em";
    }

    if (this.classes.length > 0) {
      span = span || document.createElement("span");
      span.className = createClass(this.classes);
    }

    for (var style in this.style) {
      if (this.style.hasOwnProperty(style)) {
        span = span || document.createElement("span"); // $FlowFixMe Flow doesn't seem to understand span.style's type.

        span.style[style] = this.style[style];
      }
    }

    if (span) {
      span.appendChild(node);
      return span;
    } else {
      return node;
    }
  }
  /**
   * Creates markup for a symbol node.
   */
  ;

  _proto4.toMarkup = function toMarkup() {
    // TODO(alpert): More duplication than I'd like from
    // span.prototype.toMarkup and symbolNode.prototype.toNode...
    var needsSpan = false;
    var markup = "<span";

    if (this.classes.length) {
      needsSpan = true;
      markup += " class=\"";
      markup += utils.escape(createClass(this.classes));
      markup += "\"";
    }

    var styles = "";

    if (this.italic > 0) {
      styles += "margin-right:" + this.italic + "em;";
    }

    for (var style in this.style) {
      if (this.style.hasOwnProperty(style)) {
        styles += utils.hyphenate(style) + ":" + this.style[style] + ";";
      }
    }

    if (styles) {
      needsSpan = true;
      markup += " style=\"" + utils.escape(styles) + "\"";
    }

    var escaped = utils.escape(this.text);

    if (needsSpan) {
      markup += ">";
      markup += escaped;
      markup += "</span>";
      return markup;
    } else {
      return escaped;
    }
  };

  return SymbolNode;
}();
/**
 * SVG nodes are used to render stretchy wide elements.
 */

var SvgNode =
/*#__PURE__*/
function () {
  function SvgNode(children, attributes) {
    this.children = void 0;
    this.attributes = void 0;
    this.children = children || [];
    this.attributes = attributes || {};
  }

  var _proto5 = SvgNode.prototype;

  _proto5.toNode = function toNode() {
    var svgNS = "http://www.w3.org/2000/svg";
    var node = document.createElementNS(svgNS, "svg"); // Apply attributes

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        node.setAttribute(attr, this.attributes[attr]);
      }
    }

    for (var i = 0; i < this.children.length; i++) {
      node.appendChild(this.children[i].toNode());
    }

    return node;
  };

  _proto5.toMarkup = function toMarkup() {
    var markup = "<svg"; // Apply attributes

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        markup += " " + attr + "='" + this.attributes[attr] + "'";
      }
    }

    markup += ">";

    for (var i = 0; i < this.children.length; i++) {
      markup += this.children[i].toMarkup();
    }

    markup += "</svg>";
    return markup;
  };

  return SvgNode;
}();
var domTree_PathNode =
/*#__PURE__*/
function () {
  function PathNode(pathName, alternate) {
    this.pathName = void 0;
    this.alternate = void 0;
    this.pathName = pathName;
    this.alternate = alternate; // Used only for tall \sqrt
  }

  var _proto6 = PathNode.prototype;

  _proto6.toNode = function toNode() {
    var svgNS = "http://www.w3.org/2000/svg";
    var node = document.createElementNS(svgNS, "path");

    if (this.alternate) {
      node.setAttribute("d", this.alternate);
    } else {
      node.setAttribute("d", svgGeometry.path[this.pathName]);
    }

    return node;
  };

  _proto6.toMarkup = function toMarkup() {
    if (this.alternate) {
      return "<path d='" + this.alternate + "'/>";
    } else {
      return "<path d='" + svgGeometry.path[this.pathName] + "'/>";
    }
  };

  return PathNode;
}();
var LineNode =
/*#__PURE__*/
function () {
  function LineNode(attributes) {
    this.attributes = void 0;
    this.attributes = attributes || {};
  }

  var _proto7 = LineNode.prototype;

  _proto7.toNode = function toNode() {
    var svgNS = "http://www.w3.org/2000/svg";
    var node = document.createElementNS(svgNS, "line"); // Apply attributes

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        node.setAttribute(attr, this.attributes[attr]);
      }
    }

    return node;
  };

  _proto7.toMarkup = function toMarkup() {
    var markup = "<line";

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        markup += " " + attr + "='" + this.attributes[attr] + "'";
      }
    }

    markup += "/>";
    return markup;
  };

  return LineNode;
}();
function assertSymbolDomNode(group) {
  if (group instanceof domTree_SymbolNode) {
    return group;
  } else {
    throw new Error("Expected symbolNode but got " + String(group) + ".");
  }
}
function assertSpan(group) {
  if (group instanceof domTree_Span) {
    return group;
  } else {
    throw new Error("Expected span<HtmlDomNode> but got " + String(group) + ".");
  }
}
// CONCATENATED MODULE: ./submodules/katex-fonts/fontMetricsData.js
// This file is GENERATED by buildMetrics.sh. DO NOT MODIFY.
/* harmony default export */ var fontMetricsData = ({
  "AMS-Regular": {
    "65": [0, 0.68889, 0, 0, 0.72222],
    "66": [0, 0.68889, 0, 0, 0.66667],
    "67": [0, 0.68889, 0, 0, 0.72222],
    "68": [0, 0.68889, 0, 0, 0.72222],
    "69": [0, 0.68889, 0, 0, 0.66667],
    "70": [0, 0.68889, 0, 0, 0.61111],
    "71": [0, 0.68889, 0, 0, 0.77778],
    "72": [0, 0.68889, 0, 0, 0.77778],
    "73": [0, 0.68889, 0, 0, 0.38889],
    "74": [0.16667, 0.68889, 0, 0, 0.5],
    "75": [0, 0.68889, 0, 0, 0.77778],
    "76": [0, 0.68889, 0, 0, 0.66667],
    "77": [0, 0.68889, 0, 0, 0.94445],
    "78": [0, 0.68889, 0, 0, 0.72222],
    "79": [0.16667, 0.68889, 0, 0, 0.77778],
    "80": [0, 0.68889, 0, 0, 0.61111],
    "81": [0.16667, 0.68889, 0, 0, 0.77778],
    "82": [0, 0.68889, 0, 0, 0.72222],
    "83": [0, 0.68889, 0, 0, 0.55556],
    "84": [0, 0.68889, 0, 0, 0.66667],
    "85": [0, 0.68889, 0, 0, 0.72222],
    "86": [0, 0.68889, 0, 0, 0.72222],
    "87": [0, 0.68889, 0, 0, 1.0],
    "88": [0, 0.68889, 0, 0, 0.72222],
    "89": [0, 0.68889, 0, 0, 0.72222],
    "90": [0, 0.68889, 0, 0, 0.66667],
    "107": [0, 0.68889, 0, 0, 0.55556],
    "165": [0, 0.675, 0.025, 0, 0.75],
    "174": [0.15559, 0.69224, 0, 0, 0.94666],
    "240": [0, 0.68889, 0, 0, 0.55556],
    "295": [0, 0.68889, 0, 0, 0.54028],
    "710": [0, 0.825, 0, 0, 2.33334],
    "732": [0, 0.9, 0, 0, 2.33334],
    "770": [0, 0.825, 0, 0, 2.33334],
    "771": [0, 0.9, 0, 0, 2.33334],
    "989": [0.08167, 0.58167, 0, 0, 0.77778],
    "1008": [0, 0.43056, 0.04028, 0, 0.66667],
    "8245": [0, 0.54986, 0, 0, 0.275],
    "8463": [0, 0.68889, 0, 0, 0.54028],
    "8487": [0, 0.68889, 0, 0, 0.72222],
    "8498": [0, 0.68889, 0, 0, 0.55556],
    "8502": [0, 0.68889, 0, 0, 0.66667],
    "8503": [0, 0.68889, 0, 0, 0.44445],
    "8504": [0, 0.68889, 0, 0, 0.66667],
    "8513": [0, 0.68889, 0, 0, 0.63889],
    "8592": [-0.03598, 0.46402, 0, 0, 0.5],
    "8594": [-0.03598, 0.46402, 0, 0, 0.5],
    "8602": [-0.13313, 0.36687, 0, 0, 1.0],
    "8603": [-0.13313, 0.36687, 0, 0, 1.0],
    "8606": [0.01354, 0.52239, 0, 0, 1.0],
    "8608": [0.01354, 0.52239, 0, 0, 1.0],
    "8610": [0.01354, 0.52239, 0, 0, 1.11111],
    "8611": [0.01354, 0.52239, 0, 0, 1.11111],
    "8619": [0, 0.54986, 0, 0, 1.0],
    "8620": [0, 0.54986, 0, 0, 1.0],
    "8621": [-0.13313, 0.37788, 0, 0, 1.38889],
    "8622": [-0.13313, 0.36687, 0, 0, 1.0],
    "8624": [0, 0.69224, 0, 0, 0.5],
    "8625": [0, 0.69224, 0, 0, 0.5],
    "8630": [0, 0.43056, 0, 0, 1.0],
    "8631": [0, 0.43056, 0, 0, 1.0],
    "8634": [0.08198, 0.58198, 0, 0, 0.77778],
    "8635": [0.08198, 0.58198, 0, 0, 0.77778],
    "8638": [0.19444, 0.69224, 0, 0, 0.41667],
    "8639": [0.19444, 0.69224, 0, 0, 0.41667],
    "8642": [0.19444, 0.69224, 0, 0, 0.41667],
    "8643": [0.19444, 0.69224, 0, 0, 0.41667],
    "8644": [0.1808, 0.675, 0, 0, 1.0],
    "8646": [0.1808, 0.675, 0, 0, 1.0],
    "8647": [0.1808, 0.675, 0, 0, 1.0],
    "8648": [0.19444, 0.69224, 0, 0, 0.83334],
    "8649": [0.1808, 0.675, 0, 0, 1.0],
    "8650": [0.19444, 0.69224, 0, 0, 0.83334],
    "8651": [0.01354, 0.52239, 0, 0, 1.0],
    "8652": [0.01354, 0.52239, 0, 0, 1.0],
    "8653": [-0.13313, 0.36687, 0, 0, 1.0],
    "8654": [-0.13313, 0.36687, 0, 0, 1.0],
    "8655": [-0.13313, 0.36687, 0, 0, 1.0],
    "8666": [0.13667, 0.63667, 0, 0, 1.0],
    "8667": [0.13667, 0.63667, 0, 0, 1.0],
    "8669": [-0.13313, 0.37788, 0, 0, 1.0],
    "8672": [-0.064, 0.437, 0, 0, 1.334],
    "8674": [-0.064, 0.437, 0, 0, 1.334],
    "8705": [0, 0.825, 0, 0, 0.5],
    "8708": [0, 0.68889, 0, 0, 0.55556],
    "8709": [0.08167, 0.58167, 0, 0, 0.77778],
    "8717": [0, 0.43056, 0, 0, 0.42917],
    "8722": [-0.03598, 0.46402, 0, 0, 0.5],
    "8724": [0.08198, 0.69224, 0, 0, 0.77778],
    "8726": [0.08167, 0.58167, 0, 0, 0.77778],
    "8733": [0, 0.69224, 0, 0, 0.77778],
    "8736": [0, 0.69224, 0, 0, 0.72222],
    "8737": [0, 0.69224, 0, 0, 0.72222],
    "8738": [0.03517, 0.52239, 0, 0, 0.72222],
    "8739": [0.08167, 0.58167, 0, 0, 0.22222],
    "8740": [0.25142, 0.74111, 0, 0, 0.27778],
    "8741": [0.08167, 0.58167, 0, 0, 0.38889],
    "8742": [0.25142, 0.74111, 0, 0, 0.5],
    "8756": [0, 0.69224, 0, 0, 0.66667],
    "8757": [0, 0.69224, 0, 0, 0.66667],
    "8764": [-0.13313, 0.36687, 0, 0, 0.77778],
    "8765": [-0.13313, 0.37788, 0, 0, 0.77778],
    "8769": [-0.13313, 0.36687, 0, 0, 0.77778],
    "8770": [-0.03625, 0.46375, 0, 0, 0.77778],
    "8774": [0.30274, 0.79383, 0, 0, 0.77778],
    "8776": [-0.01688, 0.48312, 0, 0, 0.77778],
    "8778": [0.08167, 0.58167, 0, 0, 0.77778],
    "8782": [0.06062, 0.54986, 0, 0, 0.77778],
    "8783": [0.06062, 0.54986, 0, 0, 0.77778],
    "8785": [0.08198, 0.58198, 0, 0, 0.77778],
    "8786": [0.08198, 0.58198, 0, 0, 0.77778],
    "8787": [0.08198, 0.58198, 0, 0, 0.77778],
    "8790": [0, 0.69224, 0, 0, 0.77778],
    "8791": [0.22958, 0.72958, 0, 0, 0.77778],
    "8796": [0.08198, 0.91667, 0, 0, 0.77778],
    "8806": [0.25583, 0.75583, 0, 0, 0.77778],
    "8807": [0.25583, 0.75583, 0, 0, 0.77778],
    "8808": [0.25142, 0.75726, 0, 0, 0.77778],
    "8809": [0.25142, 0.75726, 0, 0, 0.77778],
    "8812": [0.25583, 0.75583, 0, 0, 0.5],
    "8814": [0.20576, 0.70576, 0, 0, 0.77778],
    "8815": [0.20576, 0.70576, 0, 0, 0.77778],
    "8816": [0.30274, 0.79383, 0, 0, 0.77778],
    "8817": [0.30274, 0.79383, 0, 0, 0.77778],
    "8818": [0.22958, 0.72958, 0, 0, 0.77778],
    "8819": [0.22958, 0.72958, 0, 0, 0.77778],
    "8822": [0.1808, 0.675, 0, 0, 0.77778],
    "8823": [0.1808, 0.675, 0, 0, 0.77778],
    "8828": [0.13667, 0.63667, 0, 0, 0.77778],
    "8829": [0.13667, 0.63667, 0, 0, 0.77778],
    "8830": [0.22958, 0.72958, 0, 0, 0.77778],
    "8831": [0.22958, 0.72958, 0, 0, 0.77778],
    "8832": [0.20576, 0.70576, 0, 0, 0.77778],
    "8833": [0.20576, 0.70576, 0, 0, 0.77778],
    "8840": [0.30274, 0.79383, 0, 0, 0.77778],
    "8841": [0.30274, 0.79383, 0, 0, 0.77778],
    "8842": [0.13597, 0.63597, 0, 0, 0.77778],
    "8843": [0.13597, 0.63597, 0, 0, 0.77778],
    "8847": [0.03517, 0.54986, 0, 0, 0.77778],
    "8848": [0.03517, 0.54986, 0, 0, 0.77778],
    "8858": [0.08198, 0.58198, 0, 0, 0.77778],
    "8859": [0.08198, 0.58198, 0, 0, 0.77778],
    "8861": [0.08198, 0.58198, 0, 0, 0.77778],
    "8862": [0, 0.675, 0, 0, 0.77778],
    "8863": [0, 0.675, 0, 0, 0.77778],
    "8864": [0, 0.675, 0, 0, 0.77778],
    "8865": [0, 0.675, 0, 0, 0.77778],
    "8872": [0, 0.69224, 0, 0, 0.61111],
    "8873": [0, 0.69224, 0, 0, 0.72222],
    "8874": [0, 0.69224, 0, 0, 0.88889],
    "8876": [0, 0.68889, 0, 0, 0.61111],
    "8877": [0, 0.68889, 0, 0, 0.61111],
    "8878": [0, 0.68889, 0, 0, 0.72222],
    "8879": [0, 0.68889, 0, 0, 0.72222],
    "8882": [0.03517, 0.54986, 0, 0, 0.77778],
    "8883": [0.03517, 0.54986, 0, 0, 0.77778],
    "8884": [0.13667, 0.63667, 0, 0, 0.77778],
    "8885": [0.13667, 0.63667, 0, 0, 0.77778],
    "8888": [0, 0.54986, 0, 0, 1.11111],
    "8890": [0.19444, 0.43056, 0, 0, 0.55556],
    "8891": [0.19444, 0.69224, 0, 0, 0.61111],
    "8892": [0.19444, 0.69224, 0, 0, 0.61111],
    "8901": [0, 0.54986, 0, 0, 0.27778],
    "8903": [0.08167, 0.58167, 0, 0, 0.77778],
    "8905": [0.08167, 0.58167, 0, 0, 0.77778],
    "8906": [0.08167, 0.58167, 0, 0, 0.77778],
    "8907": [0, 0.69224, 0, 0, 0.77778],
    "8908": [0, 0.69224, 0, 0, 0.77778],
    "8909": [-0.03598, 0.46402, 0, 0, 0.77778],
    "8910": [0, 0.54986, 0, 0, 0.76042],
    "8911": [0, 0.54986, 0, 0, 0.76042],
    "8912": [0.03517, 0.54986, 0, 0, 0.77778],
    "8913": [0.03517, 0.54986, 0, 0, 0.77778],
    "8914": [0, 0.54986, 0, 0, 0.66667],
    "8915": [0, 0.54986, 0, 0, 0.66667],
    "8916": [0, 0.69224, 0, 0, 0.66667],
    "8918": [0.0391, 0.5391, 0, 0, 0.77778],
    "8919": [0.0391, 0.5391, 0, 0, 0.77778],
    "8920": [0.03517, 0.54986, 0, 0, 1.33334],
    "8921": [0.03517, 0.54986, 0, 0, 1.33334],
    "8922": [0.38569, 0.88569, 0, 0, 0.77778],
    "8923": [0.38569, 0.88569, 0, 0, 0.77778],
    "8926": [0.13667, 0.63667, 0, 0, 0.77778],
    "8927": [0.13667, 0.63667, 0, 0, 0.77778],
    "8928": [0.30274, 0.79383, 0, 0, 0.77778],
    "8929": [0.30274, 0.79383, 0, 0, 0.77778],
    "8934": [0.23222, 0.74111, 0, 0, 0.77778],
    "8935": [0.23222, 0.74111, 0, 0, 0.77778],
    "8936": [0.23222, 0.74111, 0, 0, 0.77778],
    "8937": [0.23222, 0.74111, 0, 0, 0.77778],
    "8938": [0.20576, 0.70576, 0, 0, 0.77778],
    "8939": [0.20576, 0.70576, 0, 0, 0.77778],
    "8940": [0.30274, 0.79383, 0, 0, 0.77778],
    "8941": [0.30274, 0.79383, 0, 0, 0.77778],
    "8994": [0.19444, 0.69224, 0, 0, 0.77778],
    "8995": [0.19444, 0.69224, 0, 0, 0.77778],
    "9416": [0.15559, 0.69224, 0, 0, 0.90222],
    "9484": [0, 0.69224, 0, 0, 0.5],
    "9488": [0, 0.69224, 0, 0, 0.5],
    "9492": [0, 0.37788, 0, 0, 0.5],
    "9496": [0, 0.37788, 0, 0, 0.5],
    "9585": [0.19444, 0.68889, 0, 0, 0.88889],
    "9586": [0.19444, 0.74111, 0, 0, 0.88889],
    "9632": [0, 0.675, 0, 0, 0.77778],
    "9633": [0, 0.675, 0, 0, 0.77778],
    "9650": [0, 0.54986, 0, 0, 0.72222],
    "9651": [0, 0.54986, 0, 0, 0.72222],
    "9654": [0.03517, 0.54986, 0, 0, 0.77778],
    "9660": [0, 0.54986, 0, 0, 0.72222],
    "9661": [0, 0.54986, 0, 0, 0.72222],
    "9664": [0.03517, 0.54986, 0, 0, 0.77778],
    "9674": [0.11111, 0.69224, 0, 0, 0.66667],
    "9733": [0.19444, 0.69224, 0, 0, 0.94445],
    "10003": [0, 0.69224, 0, 0, 0.83334],
    "10016": [0, 0.69224, 0, 0, 0.83334],
    "10731": [0.11111, 0.69224, 0, 0, 0.66667],
    "10846": [0.19444, 0.75583, 0, 0, 0.61111],
    "10877": [0.13667, 0.63667, 0, 0, 0.77778],
    "10878": [0.13667, 0.63667, 0, 0, 0.77778],
    "10885": [0.25583, 0.75583, 0, 0, 0.77778],
    "10886": [0.25583, 0.75583, 0, 0, 0.77778],
    "10887": [0.13597, 0.63597, 0, 0, 0.77778],
    "10888": [0.13597, 0.63597, 0, 0, 0.77778],
    "10889": [0.26167, 0.75726, 0, 0, 0.77778],
    "10890": [0.26167, 0.75726, 0, 0, 0.77778],
    "10891": [0.48256, 0.98256, 0, 0, 0.77778],
    "10892": [0.48256, 0.98256, 0, 0, 0.77778],
    "10901": [0.13667, 0.63667, 0, 0, 0.77778],
    "10902": [0.13667, 0.63667, 0, 0, 0.77778],
    "10933": [0.25142, 0.75726, 0, 0, 0.77778],
    "10934": [0.25142, 0.75726, 0, 0, 0.77778],
    "10935": [0.26167, 0.75726, 0, 0, 0.77778],
    "10936": [0.26167, 0.75726, 0, 0, 0.77778],
    "10937": [0.26167, 0.75726, 0, 0, 0.77778],
    "10938": [0.26167, 0.75726, 0, 0, 0.77778],
    "10949": [0.25583, 0.75583, 0, 0, 0.77778],
    "10950": [0.25583, 0.75583, 0, 0, 0.77778],
    "10955": [0.28481, 0.79383, 0, 0, 0.77778],
    "10956": [0.28481, 0.79383, 0, 0, 0.77778],
    "57350": [0.08167, 0.58167, 0, 0, 0.22222],
    "57351": [0.08167, 0.58167, 0, 0, 0.38889],
    "57352": [0.08167, 0.58167, 0, 0, 0.77778],
    "57353": [0, 0.43056, 0.04028, 0, 0.66667],
    "57356": [0.25142, 0.75726, 0, 0, 0.77778],
    "57357": [0.25142, 0.75726, 0, 0, 0.77778],
    "57358": [0.41951, 0.91951, 0, 0, 0.77778],
    "57359": [0.30274, 0.79383, 0, 0, 0.77778],
    "57360": [0.30274, 0.79383, 0, 0, 0.77778],
    "57361": [0.41951, 0.91951, 0, 0, 0.77778],
    "57366": [0.25142, 0.75726, 0, 0, 0.77778],
    "57367": [0.25142, 0.75726, 0, 0, 0.77778],
    "57368": [0.25142, 0.75726, 0, 0, 0.77778],
    "57369": [0.25142, 0.75726, 0, 0, 0.77778],
    "57370": [0.13597, 0.63597, 0, 0, 0.77778],
    "57371": [0.13597, 0.63597, 0, 0, 0.77778]
  },
  "Caligraphic-Regular": {
    "48": [0, 0.43056, 0, 0, 0.5],
    "49": [0, 0.43056, 0, 0, 0.5],
    "50": [0, 0.43056, 0, 0, 0.5],
    "51": [0.19444, 0.43056, 0, 0, 0.5],
    "52": [0.19444, 0.43056, 0, 0, 0.5],
    "53": [0.19444, 0.43056, 0, 0, 0.5],
    "54": [0, 0.64444, 0, 0, 0.5],
    "55": [0.19444, 0.43056, 0, 0, 0.5],
    "56": [0, 0.64444, 0, 0, 0.5],
    "57": [0.19444, 0.43056, 0, 0, 0.5],
    "65": [0, 0.68333, 0, 0.19445, 0.79847],
    "66": [0, 0.68333, 0.03041, 0.13889, 0.65681],
    "67": [0, 0.68333, 0.05834, 0.13889, 0.52653],
    "68": [0, 0.68333, 0.02778, 0.08334, 0.77139],
    "69": [0, 0.68333, 0.08944, 0.11111, 0.52778],
    "70": [0, 0.68333, 0.09931, 0.11111, 0.71875],
    "71": [0.09722, 0.68333, 0.0593, 0.11111, 0.59487],
    "72": [0, 0.68333, 0.00965, 0.11111, 0.84452],
    "73": [0, 0.68333, 0.07382, 0, 0.54452],
    "74": [0.09722, 0.68333, 0.18472, 0.16667, 0.67778],
    "75": [0, 0.68333, 0.01445, 0.05556, 0.76195],
    "76": [0, 0.68333, 0, 0.13889, 0.68972],
    "77": [0, 0.68333, 0, 0.13889, 1.2009],
    "78": [0, 0.68333, 0.14736, 0.08334, 0.82049],
    "79": [0, 0.68333, 0.02778, 0.11111, 0.79611],
    "80": [0, 0.68333, 0.08222, 0.08334, 0.69556],
    "81": [0.09722, 0.68333, 0, 0.11111, 0.81667],
    "82": [0, 0.68333, 0, 0.08334, 0.8475],
    "83": [0, 0.68333, 0.075, 0.13889, 0.60556],
    "84": [0, 0.68333, 0.25417, 0, 0.54464],
    "85": [0, 0.68333, 0.09931, 0.08334, 0.62583],
    "86": [0, 0.68333, 0.08222, 0, 0.61278],
    "87": [0, 0.68333, 0.08222, 0.08334, 0.98778],
    "88": [0, 0.68333, 0.14643, 0.13889, 0.7133],
    "89": [0.09722, 0.68333, 0.08222, 0.08334, 0.66834],
    "90": [0, 0.68333, 0.07944, 0.13889, 0.72473]
  },
  "Fraktur-Regular": {
    "33": [0, 0.69141, 0, 0, 0.29574],
    "34": [0, 0.69141, 0, 0, 0.21471],
    "38": [0, 0.69141, 0, 0, 0.73786],
    "39": [0, 0.69141, 0, 0, 0.21201],
    "40": [0.24982, 0.74947, 0, 0, 0.38865],
    "41": [0.24982, 0.74947, 0, 0, 0.38865],
    "42": [0, 0.62119, 0, 0, 0.27764],
    "43": [0.08319, 0.58283, 0, 0, 0.75623],
    "44": [0, 0.10803, 0, 0, 0.27764],
    "45": [0.08319, 0.58283, 0, 0, 0.75623],
    "46": [0, 0.10803, 0, 0, 0.27764],
    "47": [0.24982, 0.74947, 0, 0, 0.50181],
    "48": [0, 0.47534, 0, 0, 0.50181],
    "49": [0, 0.47534, 0, 0, 0.50181],
    "50": [0, 0.47534, 0, 0, 0.50181],
    "51": [0.18906, 0.47534, 0, 0, 0.50181],
    "52": [0.18906, 0.47534, 0, 0, 0.50181],
    "53": [0.18906, 0.47534, 0, 0, 0.50181],
    "54": [0, 0.69141, 0, 0, 0.50181],
    "55": [0.18906, 0.47534, 0, 0, 0.50181],
    "56": [0, 0.69141, 0, 0, 0.50181],
    "57": [0.18906, 0.47534, 0, 0, 0.50181],
    "58": [0, 0.47534, 0, 0, 0.21606],
    "59": [0.12604, 0.47534, 0, 0, 0.21606],
    "61": [-0.13099, 0.36866, 0, 0, 0.75623],
    "63": [0, 0.69141, 0, 0, 0.36245],
    "65": [0, 0.69141, 0, 0, 0.7176],
    "66": [0, 0.69141, 0, 0, 0.88397],
    "67": [0, 0.69141, 0, 0, 0.61254],
    "68": [0, 0.69141, 0, 0, 0.83158],
    "69": [0, 0.69141, 0, 0, 0.66278],
    "70": [0.12604, 0.69141, 0, 0, 0.61119],
    "71": [0, 0.69141, 0, 0, 0.78539],
    "72": [0.06302, 0.69141, 0, 0, 0.7203],
    "73": [0, 0.69141, 0, 0, 0.55448],
    "74": [0.12604, 0.69141, 0, 0, 0.55231],
    "75": [0, 0.69141, 0, 0, 0.66845],
    "76": [0, 0.69141, 0, 0, 0.66602],
    "77": [0, 0.69141, 0, 0, 1.04953],
    "78": [0, 0.69141, 0, 0, 0.83212],
    "79": [0, 0.69141, 0, 0, 0.82699],
    "80": [0.18906, 0.69141, 0, 0, 0.82753],
    "81": [0.03781, 0.69141, 0, 0, 0.82699],
    "82": [0, 0.69141, 0, 0, 0.82807],
    "83": [0, 0.69141, 0, 0, 0.82861],
    "84": [0, 0.69141, 0, 0, 0.66899],
    "85": [0, 0.69141, 0, 0, 0.64576],
    "86": [0, 0.69141, 0, 0, 0.83131],
    "87": [0, 0.69141, 0, 0, 1.04602],
    "88": [0, 0.69141, 0, 0, 0.71922],
    "89": [0.18906, 0.69141, 0, 0, 0.83293],
    "90": [0.12604, 0.69141, 0, 0, 0.60201],
    "91": [0.24982, 0.74947, 0, 0, 0.27764],
    "93": [0.24982, 0.74947, 0, 0, 0.27764],
    "94": [0, 0.69141, 0, 0, 0.49965],
    "97": [0, 0.47534, 0, 0, 0.50046],
    "98": [0, 0.69141, 0, 0, 0.51315],
    "99": [0, 0.47534, 0, 0, 0.38946],
    "100": [0, 0.62119, 0, 0, 0.49857],
    "101": [0, 0.47534, 0, 0, 0.40053],
    "102": [0.18906, 0.69141, 0, 0, 0.32626],
    "103": [0.18906, 0.47534, 0, 0, 0.5037],
    "104": [0.18906, 0.69141, 0, 0, 0.52126],
    "105": [0, 0.69141, 0, 0, 0.27899],
    "106": [0, 0.69141, 0, 0, 0.28088],
    "107": [0, 0.69141, 0, 0, 0.38946],
    "108": [0, 0.69141, 0, 0, 0.27953],
    "109": [0, 0.47534, 0, 0, 0.76676],
    "110": [0, 0.47534, 0, 0, 0.52666],
    "111": [0, 0.47534, 0, 0, 0.48885],
    "112": [0.18906, 0.52396, 0, 0, 0.50046],
    "113": [0.18906, 0.47534, 0, 0, 0.48912],
    "114": [0, 0.47534, 0, 0, 0.38919],
    "115": [0, 0.47534, 0, 0, 0.44266],
    "116": [0, 0.62119, 0, 0, 0.33301],
    "117": [0, 0.47534, 0, 0, 0.5172],
    "118": [0, 0.52396, 0, 0, 0.5118],
    "119": [0, 0.52396, 0, 0, 0.77351],
    "120": [0.18906, 0.47534, 0, 0, 0.38865],
    "121": [0.18906, 0.47534, 0, 0, 0.49884],
    "122": [0.18906, 0.47534, 0, 0, 0.39054],
    "8216": [0, 0.69141, 0, 0, 0.21471],
    "8217": [0, 0.69141, 0, 0, 0.21471],
    "58112": [0, 0.62119, 0, 0, 0.49749],
    "58113": [0, 0.62119, 0, 0, 0.4983],
    "58114": [0.18906, 0.69141, 0, 0, 0.33328],
    "58115": [0.18906, 0.69141, 0, 0, 0.32923],
    "58116": [0.18906, 0.47534, 0, 0, 0.50343],
    "58117": [0, 0.69141, 0, 0, 0.33301],
    "58118": [0, 0.62119, 0, 0, 0.33409],
    "58119": [0, 0.47534, 0, 0, 0.50073]
  },
  "Main-Bold": {
    "33": [0, 0.69444, 0, 0, 0.35],
    "34": [0, 0.69444, 0, 0, 0.60278],
    "35": [0.19444, 0.69444, 0, 0, 0.95833],
    "36": [0.05556, 0.75, 0, 0, 0.575],
    "37": [0.05556, 0.75, 0, 0, 0.95833],
    "38": [0, 0.69444, 0, 0, 0.89444],
    "39": [0, 0.69444, 0, 0, 0.31944],
    "40": [0.25, 0.75, 0, 0, 0.44722],
    "41": [0.25, 0.75, 0, 0, 0.44722],
    "42": [0, 0.75, 0, 0, 0.575],
    "43": [0.13333, 0.63333, 0, 0, 0.89444],
    "44": [0.19444, 0.15556, 0, 0, 0.31944],
    "45": [0, 0.44444, 0, 0, 0.38333],
    "46": [0, 0.15556, 0, 0, 0.31944],
    "47": [0.25, 0.75, 0, 0, 0.575],
    "48": [0, 0.64444, 0, 0, 0.575],
    "49": [0, 0.64444, 0, 0, 0.575],
    "50": [0, 0.64444, 0, 0, 0.575],
    "51": [0, 0.64444, 0, 0, 0.575],
    "52": [0, 0.64444, 0, 0, 0.575],
    "53": [0, 0.64444, 0, 0, 0.575],
    "54": [0, 0.64444, 0, 0, 0.575],
    "55": [0, 0.64444, 0, 0, 0.575],
    "56": [0, 0.64444, 0, 0, 0.575],
    "57": [0, 0.64444, 0, 0, 0.575],
    "58": [0, 0.44444, 0, 0, 0.31944],
    "59": [0.19444, 0.44444, 0, 0, 0.31944],
    "60": [0.08556, 0.58556, 0, 0, 0.89444],
    "61": [-0.10889, 0.39111, 0, 0, 0.89444],
    "62": [0.08556, 0.58556, 0, 0, 0.89444],
    "63": [0, 0.69444, 0, 0, 0.54305],
    "64": [0, 0.69444, 0, 0, 0.89444],
    "65": [0, 0.68611, 0, 0, 0.86944],
    "66": [0, 0.68611, 0, 0, 0.81805],
    "67": [0, 0.68611, 0, 0, 0.83055],
    "68": [0, 0.68611, 0, 0, 0.88194],
    "69": [0, 0.68611, 0, 0, 0.75555],
    "70": [0, 0.68611, 0, 0, 0.72361],
    "71": [0, 0.68611, 0, 0, 0.90416],
    "72": [0, 0.68611, 0, 0, 0.9],
    "73": [0, 0.68611, 0, 0, 0.43611],
    "74": [0, 0.68611, 0, 0, 0.59444],
    "75": [0, 0.68611, 0, 0, 0.90138],
    "76": [0, 0.68611, 0, 0, 0.69166],
    "77": [0, 0.68611, 0, 0, 1.09166],
    "78": [0, 0.68611, 0, 0, 0.9],
    "79": [0, 0.68611, 0, 0, 0.86388],
    "80": [0, 0.68611, 0, 0, 0.78611],
    "81": [0.19444, 0.68611, 0, 0, 0.86388],
    "82": [0, 0.68611, 0, 0, 0.8625],
    "83": [0, 0.68611, 0, 0, 0.63889],
    "84": [0, 0.68611, 0, 0, 0.8],
    "85": [0, 0.68611, 0, 0, 0.88472],
    "86": [0, 0.68611, 0.01597, 0, 0.86944],
    "87": [0, 0.68611, 0.01597, 0, 1.18888],
    "88": [0, 0.68611, 0, 0, 0.86944],
    "89": [0, 0.68611, 0.02875, 0, 0.86944],
    "90": [0, 0.68611, 0, 0, 0.70277],
    "91": [0.25, 0.75, 0, 0, 0.31944],
    "92": [0.25, 0.75, 0, 0, 0.575],
    "93": [0.25, 0.75, 0, 0, 0.31944],
    "94": [0, 0.69444, 0, 0, 0.575],
    "95": [0.31, 0.13444, 0.03194, 0, 0.575],
    "97": [0, 0.44444, 0, 0, 0.55902],
    "98": [0, 0.69444, 0, 0, 0.63889],
    "99": [0, 0.44444, 0, 0, 0.51111],
    "100": [0, 0.69444, 0, 0, 0.63889],
    "101": [0, 0.44444, 0, 0, 0.52708],
    "102": [0, 0.69444, 0.10903, 0, 0.35139],
    "103": [0.19444, 0.44444, 0.01597, 0, 0.575],
    "104": [0, 0.69444, 0, 0, 0.63889],
    "105": [0, 0.69444, 0, 0, 0.31944],
    "106": [0.19444, 0.69444, 0, 0, 0.35139],
    "107": [0, 0.69444, 0, 0, 0.60694],
    "108": [0, 0.69444, 0, 0, 0.31944],
    "109": [0, 0.44444, 0, 0, 0.95833],
    "110": [0, 0.44444, 0, 0, 0.63889],
    "111": [0, 0.44444, 0, 0, 0.575],
    "112": [0.19444, 0.44444, 0, 0, 0.63889],
    "113": [0.19444, 0.44444, 0, 0, 0.60694],
    "114": [0, 0.44444, 0, 0, 0.47361],
    "115": [0, 0.44444, 0, 0, 0.45361],
    "116": [0, 0.63492, 0, 0, 0.44722],
    "117": [0, 0.44444, 0, 0, 0.63889],
    "118": [0, 0.44444, 0.01597, 0, 0.60694],
    "119": [0, 0.44444, 0.01597, 0, 0.83055],
    "120": [0, 0.44444, 0, 0, 0.60694],
    "121": [0.19444, 0.44444, 0.01597, 0, 0.60694],
    "122": [0, 0.44444, 0, 0, 0.51111],
    "123": [0.25, 0.75, 0, 0, 0.575],
    "124": [0.25, 0.75, 0, 0, 0.31944],
    "125": [0.25, 0.75, 0, 0, 0.575],
    "126": [0.35, 0.34444, 0, 0, 0.575],
    "168": [0, 0.69444, 0, 0, 0.575],
    "172": [0, 0.44444, 0, 0, 0.76666],
    "176": [0, 0.69444, 0, 0, 0.86944],
    "177": [0.13333, 0.63333, 0, 0, 0.89444],
    "184": [0.17014, 0, 0, 0, 0.51111],
    "198": [0, 0.68611, 0, 0, 1.04166],
    "215": [0.13333, 0.63333, 0, 0, 0.89444],
    "216": [0.04861, 0.73472, 0, 0, 0.89444],
    "223": [0, 0.69444, 0, 0, 0.59722],
    "230": [0, 0.44444, 0, 0, 0.83055],
    "247": [0.13333, 0.63333, 0, 0, 0.89444],
    "248": [0.09722, 0.54167, 0, 0, 0.575],
    "305": [0, 0.44444, 0, 0, 0.31944],
    "338": [0, 0.68611, 0, 0, 1.16944],
    "339": [0, 0.44444, 0, 0, 0.89444],
    "567": [0.19444, 0.44444, 0, 0, 0.35139],
    "710": [0, 0.69444, 0, 0, 0.575],
    "711": [0, 0.63194, 0, 0, 0.575],
    "713": [0, 0.59611, 0, 0, 0.575],
    "714": [0, 0.69444, 0, 0, 0.575],
    "715": [0, 0.69444, 0, 0, 0.575],
    "728": [0, 0.69444, 0, 0, 0.575],
    "729": [0, 0.69444, 0, 0, 0.31944],
    "730": [0, 0.69444, 0, 0, 0.86944],
    "732": [0, 0.69444, 0, 0, 0.575],
    "733": [0, 0.69444, 0, 0, 0.575],
    "915": [0, 0.68611, 0, 0, 0.69166],
    "916": [0, 0.68611, 0, 0, 0.95833],
    "920": [0, 0.68611, 0, 0, 0.89444],
    "923": [0, 0.68611, 0, 0, 0.80555],
    "926": [0, 0.68611, 0, 0, 0.76666],
    "928": [0, 0.68611, 0, 0, 0.9],
    "931": [0, 0.68611, 0, 0, 0.83055],
    "933": [0, 0.68611, 0, 0, 0.89444],
    "934": [0, 0.68611, 0, 0, 0.83055],
    "936": [0, 0.68611, 0, 0, 0.89444],
    "937": [0, 0.68611, 0, 0, 0.83055],
    "8211": [0, 0.44444, 0.03194, 0, 0.575],
    "8212": [0, 0.44444, 0.03194, 0, 1.14999],
    "8216": [0, 0.69444, 0, 0, 0.31944],
    "8217": [0, 0.69444, 0, 0, 0.31944],
    "8220": [0, 0.69444, 0, 0, 0.60278],
    "8221": [0, 0.69444, 0, 0, 0.60278],
    "8224": [0.19444, 0.69444, 0, 0, 0.51111],
    "8225": [0.19444, 0.69444, 0, 0, 0.51111],
    "8242": [0, 0.55556, 0, 0, 0.34444],
    "8407": [0, 0.72444, 0.15486, 0, 0.575],
    "8463": [0, 0.69444, 0, 0, 0.66759],
    "8465": [0, 0.69444, 0, 0, 0.83055],
    "8467": [0, 0.69444, 0, 0, 0.47361],
    "8472": [0.19444, 0.44444, 0, 0, 0.74027],
    "8476": [0, 0.69444, 0, 0, 0.83055],
    "8501": [0, 0.69444, 0, 0, 0.70277],
    "8592": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8593": [0.19444, 0.69444, 0, 0, 0.575],
    "8594": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8595": [0.19444, 0.69444, 0, 0, 0.575],
    "8596": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8597": [0.25, 0.75, 0, 0, 0.575],
    "8598": [0.19444, 0.69444, 0, 0, 1.14999],
    "8599": [0.19444, 0.69444, 0, 0, 1.14999],
    "8600": [0.19444, 0.69444, 0, 0, 1.14999],
    "8601": [0.19444, 0.69444, 0, 0, 1.14999],
    "8636": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8637": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8640": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8641": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8656": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8657": [0.19444, 0.69444, 0, 0, 0.70277],
    "8658": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8659": [0.19444, 0.69444, 0, 0, 0.70277],
    "8660": [-0.10889, 0.39111, 0, 0, 1.14999],
    "8661": [0.25, 0.75, 0, 0, 0.70277],
    "8704": [0, 0.69444, 0, 0, 0.63889],
    "8706": [0, 0.69444, 0.06389, 0, 0.62847],
    "8707": [0, 0.69444, 0, 0, 0.63889],
    "8709": [0.05556, 0.75, 0, 0, 0.575],
    "8711": [0, 0.68611, 0, 0, 0.95833],
    "8712": [0.08556, 0.58556, 0, 0, 0.76666],
    "8715": [0.08556, 0.58556, 0, 0, 0.76666],
    "8722": [0.13333, 0.63333, 0, 0, 0.89444],
    "8723": [0.13333, 0.63333, 0, 0, 0.89444],
    "8725": [0.25, 0.75, 0, 0, 0.575],
    "8726": [0.25, 0.75, 0, 0, 0.575],
    "8727": [-0.02778, 0.47222, 0, 0, 0.575],
    "8728": [-0.02639, 0.47361, 0, 0, 0.575],
    "8729": [-0.02639, 0.47361, 0, 0, 0.575],
    "8730": [0.18, 0.82, 0, 0, 0.95833],
    "8733": [0, 0.44444, 0, 0, 0.89444],
    "8734": [0, 0.44444, 0, 0, 1.14999],
    "8736": [0, 0.69224, 0, 0, 0.72222],
    "8739": [0.25, 0.75, 0, 0, 0.31944],
    "8741": [0.25, 0.75, 0, 0, 0.575],
    "8743": [0, 0.55556, 0, 0, 0.76666],
    "8744": [0, 0.55556, 0, 0, 0.76666],
    "8745": [0, 0.55556, 0, 0, 0.76666],
    "8746": [0, 0.55556, 0, 0, 0.76666],
    "8747": [0.19444, 0.69444, 0.12778, 0, 0.56875],
    "8764": [-0.10889, 0.39111, 0, 0, 0.89444],
    "8768": [0.19444, 0.69444, 0, 0, 0.31944],
    "8771": [0.00222, 0.50222, 0, 0, 0.89444],
    "8776": [0.02444, 0.52444, 0, 0, 0.89444],
    "8781": [0.00222, 0.50222, 0, 0, 0.89444],
    "8801": [0.00222, 0.50222, 0, 0, 0.89444],
    "8804": [0.19667, 0.69667, 0, 0, 0.89444],
    "8805": [0.19667, 0.69667, 0, 0, 0.89444],
    "8810": [0.08556, 0.58556, 0, 0, 1.14999],
    "8811": [0.08556, 0.58556, 0, 0, 1.14999],
    "8826": [0.08556, 0.58556, 0, 0, 0.89444],
    "8827": [0.08556, 0.58556, 0, 0, 0.89444],
    "8834": [0.08556, 0.58556, 0, 0, 0.89444],
    "8835": [0.08556, 0.58556, 0, 0, 0.89444],
    "8838": [0.19667, 0.69667, 0, 0, 0.89444],
    "8839": [0.19667, 0.69667, 0, 0, 0.89444],
    "8846": [0, 0.55556, 0, 0, 0.76666],
    "8849": [0.19667, 0.69667, 0, 0, 0.89444],
    "8850": [0.19667, 0.69667, 0, 0, 0.89444],
    "8851": [0, 0.55556, 0, 0, 0.76666],
    "8852": [0, 0.55556, 0, 0, 0.76666],
    "8853": [0.13333, 0.63333, 0, 0, 0.89444],
    "8854": [0.13333, 0.63333, 0, 0, 0.89444],
    "8855": [0.13333, 0.63333, 0, 0, 0.89444],
    "8856": [0.13333, 0.63333, 0, 0, 0.89444],
    "8857": [0.13333, 0.63333, 0, 0, 0.89444],
    "8866": [0, 0.69444, 0, 0, 0.70277],
    "8867": [0, 0.69444, 0, 0, 0.70277],
    "8868": [0, 0.69444, 0, 0, 0.89444],
    "8869": [0, 0.69444, 0, 0, 0.89444],
    "8900": [-0.02639, 0.47361, 0, 0, 0.575],
    "8901": [-0.02639, 0.47361, 0, 0, 0.31944],
    "8902": [-0.02778, 0.47222, 0, 0, 0.575],
    "8968": [0.25, 0.75, 0, 0, 0.51111],
    "8969": [0.25, 0.75, 0, 0, 0.51111],
    "8970": [0.25, 0.75, 0, 0, 0.51111],
    "8971": [0.25, 0.75, 0, 0, 0.51111],
    "8994": [-0.13889, 0.36111, 0, 0, 1.14999],
    "8995": [-0.13889, 0.36111, 0, 0, 1.14999],
    "9651": [0.19444, 0.69444, 0, 0, 1.02222],
    "9657": [-0.02778, 0.47222, 0, 0, 0.575],
    "9661": [0.19444, 0.69444, 0, 0, 1.02222],
    "9667": [-0.02778, 0.47222, 0, 0, 0.575],
    "9711": [0.19444, 0.69444, 0, 0, 1.14999],
    "9824": [0.12963, 0.69444, 0, 0, 0.89444],
    "9825": [0.12963, 0.69444, 0, 0, 0.89444],
    "9826": [0.12963, 0.69444, 0, 0, 0.89444],
    "9827": [0.12963, 0.69444, 0, 0, 0.89444],
    "9837": [0, 0.75, 0, 0, 0.44722],
    "9838": [0.19444, 0.69444, 0, 0, 0.44722],
    "9839": [0.19444, 0.69444, 0, 0, 0.44722],
    "10216": [0.25, 0.75, 0, 0, 0.44722],
    "10217": [0.25, 0.75, 0, 0, 0.44722],
    "10815": [0, 0.68611, 0, 0, 0.9],
    "10927": [0.19667, 0.69667, 0, 0, 0.89444],
    "10928": [0.19667, 0.69667, 0, 0, 0.89444],
    "57376": [0.19444, 0.69444, 0, 0, 0]
  },
  "Main-BoldItalic": {
    "33": [0, 0.69444, 0.11417, 0, 0.38611],
    "34": [0, 0.69444, 0.07939, 0, 0.62055],
    "35": [0.19444, 0.69444, 0.06833, 0, 0.94444],
    "37": [0.05556, 0.75, 0.12861, 0, 0.94444],
    "38": [0, 0.69444, 0.08528, 0, 0.88555],
    "39": [0, 0.69444, 0.12945, 0, 0.35555],
    "40": [0.25, 0.75, 0.15806, 0, 0.47333],
    "41": [0.25, 0.75, 0.03306, 0, 0.47333],
    "42": [0, 0.75, 0.14333, 0, 0.59111],
    "43": [0.10333, 0.60333, 0.03306, 0, 0.88555],
    "44": [0.19444, 0.14722, 0, 0, 0.35555],
    "45": [0, 0.44444, 0.02611, 0, 0.41444],
    "46": [0, 0.14722, 0, 0, 0.35555],
    "47": [0.25, 0.75, 0.15806, 0, 0.59111],
    "48": [0, 0.64444, 0.13167, 0, 0.59111],
    "49": [0, 0.64444, 0.13167, 0, 0.59111],
    "50": [0, 0.64444, 0.13167, 0, 0.59111],
    "51": [0, 0.64444, 0.13167, 0, 0.59111],
    "52": [0.19444, 0.64444, 0.13167, 0, 0.59111],
    "53": [0, 0.64444, 0.13167, 0, 0.59111],
    "54": [0, 0.64444, 0.13167, 0, 0.59111],
    "55": [0.19444, 0.64444, 0.13167, 0, 0.59111],
    "56": [0, 0.64444, 0.13167, 0, 0.59111],
    "57": [0, 0.64444, 0.13167, 0, 0.59111],
    "58": [0, 0.44444, 0.06695, 0, 0.35555],
    "59": [0.19444, 0.44444, 0.06695, 0, 0.35555],
    "61": [-0.10889, 0.39111, 0.06833, 0, 0.88555],
    "63": [0, 0.69444, 0.11472, 0, 0.59111],
    "64": [0, 0.69444, 0.09208, 0, 0.88555],
    "65": [0, 0.68611, 0, 0, 0.86555],
    "66": [0, 0.68611, 0.0992, 0, 0.81666],
    "67": [0, 0.68611, 0.14208, 0, 0.82666],
    "68": [0, 0.68611, 0.09062, 0, 0.87555],
    "69": [0, 0.68611, 0.11431, 0, 0.75666],
    "70": [0, 0.68611, 0.12903, 0, 0.72722],
    "71": [0, 0.68611, 0.07347, 0, 0.89527],
    "72": [0, 0.68611, 0.17208, 0, 0.8961],
    "73": [0, 0.68611, 0.15681, 0, 0.47166],
    "74": [0, 0.68611, 0.145, 0, 0.61055],
    "75": [0, 0.68611, 0.14208, 0, 0.89499],
    "76": [0, 0.68611, 0, 0, 0.69777],
    "77": [0, 0.68611, 0.17208, 0, 1.07277],
    "78": [0, 0.68611, 0.17208, 0, 0.8961],
    "79": [0, 0.68611, 0.09062, 0, 0.85499],
    "80": [0, 0.68611, 0.0992, 0, 0.78721],
    "81": [0.19444, 0.68611, 0.09062, 0, 0.85499],
    "82": [0, 0.68611, 0.02559, 0, 0.85944],
    "83": [0, 0.68611, 0.11264, 0, 0.64999],
    "84": [0, 0.68611, 0.12903, 0, 0.7961],
    "85": [0, 0.68611, 0.17208, 0, 0.88083],
    "86": [0, 0.68611, 0.18625, 0, 0.86555],
    "87": [0, 0.68611, 0.18625, 0, 1.15999],
    "88": [0, 0.68611, 0.15681, 0, 0.86555],
    "89": [0, 0.68611, 0.19803, 0, 0.86555],
    "90": [0, 0.68611, 0.14208, 0, 0.70888],
    "91": [0.25, 0.75, 0.1875, 0, 0.35611],
    "93": [0.25, 0.75, 0.09972, 0, 0.35611],
    "94": [0, 0.69444, 0.06709, 0, 0.59111],
    "95": [0.31, 0.13444, 0.09811, 0, 0.59111],
    "97": [0, 0.44444, 0.09426, 0, 0.59111],
    "98": [0, 0.69444, 0.07861, 0, 0.53222],
    "99": [0, 0.44444, 0.05222, 0, 0.53222],
    "100": [0, 0.69444, 0.10861, 0, 0.59111],
    "101": [0, 0.44444, 0.085, 0, 0.53222],
    "102": [0.19444, 0.69444, 0.21778, 0, 0.4],
    "103": [0.19444, 0.44444, 0.105, 0, 0.53222],
    "104": [0, 0.69444, 0.09426, 0, 0.59111],
    "105": [0, 0.69326, 0.11387, 0, 0.35555],
    "106": [0.19444, 0.69326, 0.1672, 0, 0.35555],
    "107": [0, 0.69444, 0.11111, 0, 0.53222],
    "108": [0, 0.69444, 0.10861, 0, 0.29666],
    "109": [0, 0.44444, 0.09426, 0, 0.94444],
    "110": [0, 0.44444, 0.09426, 0, 0.64999],
    "111": [0, 0.44444, 0.07861, 0, 0.59111],
    "112": [0.19444, 0.44444, 0.07861, 0, 0.59111],
    "113": [0.19444, 0.44444, 0.105, 0, 0.53222],
    "114": [0, 0.44444, 0.11111, 0, 0.50167],
    "115": [0, 0.44444, 0.08167, 0, 0.48694],
    "116": [0, 0.63492, 0.09639, 0, 0.385],
    "117": [0, 0.44444, 0.09426, 0, 0.62055],
    "118": [0, 0.44444, 0.11111, 0, 0.53222],
    "119": [0, 0.44444, 0.11111, 0, 0.76777],
    "120": [0, 0.44444, 0.12583, 0, 0.56055],
    "121": [0.19444, 0.44444, 0.105, 0, 0.56166],
    "122": [0, 0.44444, 0.13889, 0, 0.49055],
    "126": [0.35, 0.34444, 0.11472, 0, 0.59111],
    "163": [0, 0.69444, 0, 0, 0.86853],
    "168": [0, 0.69444, 0.11473, 0, 0.59111],
    "176": [0, 0.69444, 0, 0, 0.94888],
    "184": [0.17014, 0, 0, 0, 0.53222],
    "198": [0, 0.68611, 0.11431, 0, 1.02277],
    "216": [0.04861, 0.73472, 0.09062, 0, 0.88555],
    "223": [0.19444, 0.69444, 0.09736, 0, 0.665],
    "230": [0, 0.44444, 0.085, 0, 0.82666],
    "248": [0.09722, 0.54167, 0.09458, 0, 0.59111],
    "305": [0, 0.44444, 0.09426, 0, 0.35555],
    "338": [0, 0.68611, 0.11431, 0, 1.14054],
    "339": [0, 0.44444, 0.085, 0, 0.82666],
    "567": [0.19444, 0.44444, 0.04611, 0, 0.385],
    "710": [0, 0.69444, 0.06709, 0, 0.59111],
    "711": [0, 0.63194, 0.08271, 0, 0.59111],
    "713": [0, 0.59444, 0.10444, 0, 0.59111],
    "714": [0, 0.69444, 0.08528, 0, 0.59111],
    "715": [0, 0.69444, 0, 0, 0.59111],
    "728": [0, 0.69444, 0.10333, 0, 0.59111],
    "729": [0, 0.69444, 0.12945, 0, 0.35555],
    "730": [0, 0.69444, 0, 0, 0.94888],
    "732": [0, 0.69444, 0.11472, 0, 0.59111],
    "733": [0, 0.69444, 0.11472, 0, 0.59111],
    "915": [0, 0.68611, 0.12903, 0, 0.69777],
    "916": [0, 0.68611, 0, 0, 0.94444],
    "920": [0, 0.68611, 0.09062, 0, 0.88555],
    "923": [0, 0.68611, 0, 0, 0.80666],
    "926": [0, 0.68611, 0.15092, 0, 0.76777],
    "928": [0, 0.68611, 0.17208, 0, 0.8961],
    "931": [0, 0.68611, 0.11431, 0, 0.82666],
    "933": [0, 0.68611, 0.10778, 0, 0.88555],
    "934": [0, 0.68611, 0.05632, 0, 0.82666],
    "936": [0, 0.68611, 0.10778, 0, 0.88555],
    "937": [0, 0.68611, 0.0992, 0, 0.82666],
    "8211": [0, 0.44444, 0.09811, 0, 0.59111],
    "8212": [0, 0.44444, 0.09811, 0, 1.18221],
    "8216": [0, 0.69444, 0.12945, 0, 0.35555],
    "8217": [0, 0.69444, 0.12945, 0, 0.35555],
    "8220": [0, 0.69444, 0.16772, 0, 0.62055],
    "8221": [0, 0.69444, 0.07939, 0, 0.62055]
  },
  "Main-Italic": {
    "33": [0, 0.69444, 0.12417, 0, 0.30667],
    "34": [0, 0.69444, 0.06961, 0, 0.51444],
    "35": [0.19444, 0.69444, 0.06616, 0, 0.81777],
    "37": [0.05556, 0.75, 0.13639, 0, 0.81777],
    "38": [0, 0.69444, 0.09694, 0, 0.76666],
    "39": [0, 0.69444, 0.12417, 0, 0.30667],
    "40": [0.25, 0.75, 0.16194, 0, 0.40889],
    "41": [0.25, 0.75, 0.03694, 0, 0.40889],
    "42": [0, 0.75, 0.14917, 0, 0.51111],
    "43": [0.05667, 0.56167, 0.03694, 0, 0.76666],
    "44": [0.19444, 0.10556, 0, 0, 0.30667],
    "45": [0, 0.43056, 0.02826, 0, 0.35778],
    "46": [0, 0.10556, 0, 0, 0.30667],
    "47": [0.25, 0.75, 0.16194, 0, 0.51111],
    "48": [0, 0.64444, 0.13556, 0, 0.51111],
    "49": [0, 0.64444, 0.13556, 0, 0.51111],
    "50": [0, 0.64444, 0.13556, 0, 0.51111],
    "51": [0, 0.64444, 0.13556, 0, 0.51111],
    "52": [0.19444, 0.64444, 0.13556, 0, 0.51111],
    "53": [0, 0.64444, 0.13556, 0, 0.51111],
    "54": [0, 0.64444, 0.13556, 0, 0.51111],
    "55": [0.19444, 0.64444, 0.13556, 0, 0.51111],
    "56": [0, 0.64444, 0.13556, 0, 0.51111],
    "57": [0, 0.64444, 0.13556, 0, 0.51111],
    "58": [0, 0.43056, 0.0582, 0, 0.30667],
    "59": [0.19444, 0.43056, 0.0582, 0, 0.30667],
    "61": [-0.13313, 0.36687, 0.06616, 0, 0.76666],
    "63": [0, 0.69444, 0.1225, 0, 0.51111],
    "64": [0, 0.69444, 0.09597, 0, 0.76666],
    "65": [0, 0.68333, 0, 0, 0.74333],
    "66": [0, 0.68333, 0.10257, 0, 0.70389],
    "67": [0, 0.68333, 0.14528, 0, 0.71555],
    "68": [0, 0.68333, 0.09403, 0, 0.755],
    "69": [0, 0.68333, 0.12028, 0, 0.67833],
    "70": [0, 0.68333, 0.13305, 0, 0.65277],
    "71": [0, 0.68333, 0.08722, 0, 0.77361],
    "72": [0, 0.68333, 0.16389, 0, 0.74333],
    "73": [0, 0.68333, 0.15806, 0, 0.38555],
    "74": [0, 0.68333, 0.14028, 0, 0.525],
    "75": [0, 0.68333, 0.14528, 0, 0.76888],
    "76": [0, 0.68333, 0, 0, 0.62722],
    "77": [0, 0.68333, 0.16389, 0, 0.89666],
    "78": [0, 0.68333, 0.16389, 0, 0.74333],
    "79": [0, 0.68333, 0.09403, 0, 0.76666],
    "80": [0, 0.68333, 0.10257, 0, 0.67833],
    "81": [0.19444, 0.68333, 0.09403, 0, 0.76666],
    "82": [0, 0.68333, 0.03868, 0, 0.72944],
    "83": [0, 0.68333, 0.11972, 0, 0.56222],
    "84": [0, 0.68333, 0.13305, 0, 0.71555],
    "85": [0, 0.68333, 0.16389, 0, 0.74333],
    "86": [0, 0.68333, 0.18361, 0, 0.74333],
    "87": [0, 0.68333, 0.18361, 0, 0.99888],
    "88": [0, 0.68333, 0.15806, 0, 0.74333],
    "89": [0, 0.68333, 0.19383, 0, 0.74333],
    "90": [0, 0.68333, 0.14528, 0, 0.61333],
    "91": [0.25, 0.75, 0.1875, 0, 0.30667],
    "93": [0.25, 0.75, 0.10528, 0, 0.30667],
    "94": [0, 0.69444, 0.06646, 0, 0.51111],
    "95": [0.31, 0.12056, 0.09208, 0, 0.51111],
    "97": [0, 0.43056, 0.07671, 0, 0.51111],
    "98": [0, 0.69444, 0.06312, 0, 0.46],
    "99": [0, 0.43056, 0.05653, 0, 0.46],
    "100": [0, 0.69444, 0.10333, 0, 0.51111],
    "101": [0, 0.43056, 0.07514, 0, 0.46],
    "102": [0.19444, 0.69444, 0.21194, 0, 0.30667],
    "103": [0.19444, 0.43056, 0.08847, 0, 0.46],
    "104": [0, 0.69444, 0.07671, 0, 0.51111],
    "105": [0, 0.65536, 0.1019, 0, 0.30667],
    "106": [0.19444, 0.65536, 0.14467, 0, 0.30667],
    "107": [0, 0.69444, 0.10764, 0, 0.46],
    "108": [0, 0.69444, 0.10333, 0, 0.25555],
    "109": [0, 0.43056, 0.07671, 0, 0.81777],
    "110": [0, 0.43056, 0.07671, 0, 0.56222],
    "111": [0, 0.43056, 0.06312, 0, 0.51111],
    "112": [0.19444, 0.43056, 0.06312, 0, 0.51111],
    "113": [0.19444, 0.43056, 0.08847, 0, 0.46],
    "114": [0, 0.43056, 0.10764, 0, 0.42166],
    "115": [0, 0.43056, 0.08208, 0, 0.40889],
    "116": [0, 0.61508, 0.09486, 0, 0.33222],
    "117": [0, 0.43056, 0.07671, 0, 0.53666],
    "118": [0, 0.43056, 0.10764, 0, 0.46],
    "119": [0, 0.43056, 0.10764, 0, 0.66444],
    "120": [0, 0.43056, 0.12042, 0, 0.46389],
    "121": [0.19444, 0.43056, 0.08847, 0, 0.48555],
    "122": [0, 0.43056, 0.12292, 0, 0.40889],
    "126": [0.35, 0.31786, 0.11585, 0, 0.51111],
    "163": [0, 0.69444, 0, 0, 0.76909],
    "168": [0, 0.66786, 0.10474, 0, 0.51111],
    "176": [0, 0.69444, 0, 0, 0.83129],
    "184": [0.17014, 0, 0, 0, 0.46],
    "198": [0, 0.68333, 0.12028, 0, 0.88277],
    "216": [0.04861, 0.73194, 0.09403, 0, 0.76666],
    "223": [0.19444, 0.69444, 0.10514, 0, 0.53666],
    "230": [0, 0.43056, 0.07514, 0, 0.71555],
    "248": [0.09722, 0.52778, 0.09194, 0, 0.51111],
    "305": [0, 0.43056, 0, 0.02778, 0.32246],
    "338": [0, 0.68333, 0.12028, 0, 0.98499],
    "339": [0, 0.43056, 0.07514, 0, 0.71555],
    "567": [0.19444, 0.43056, 0, 0.08334, 0.38403],
    "710": [0, 0.69444, 0.06646, 0, 0.51111],
    "711": [0, 0.62847, 0.08295, 0, 0.51111],
    "713": [0, 0.56167, 0.10333, 0, 0.51111],
    "714": [0, 0.69444, 0.09694, 0, 0.51111],
    "715": [0, 0.69444, 0, 0, 0.51111],
    "728": [0, 0.69444, 0.10806, 0, 0.51111],
    "729": [0, 0.66786, 0.11752, 0, 0.30667],
    "730": [0, 0.69444, 0, 0, 0.83129],
    "732": [0, 0.66786, 0.11585, 0, 0.51111],
    "733": [0, 0.69444, 0.1225, 0, 0.51111],
    "915": [0, 0.68333, 0.13305, 0, 0.62722],
    "916": [0, 0.68333, 0, 0, 0.81777],
    "920": [0, 0.68333, 0.09403, 0, 0.76666],
    "923": [0, 0.68333, 0, 0, 0.69222],
    "926": [0, 0.68333, 0.15294, 0, 0.66444],
    "928": [0, 0.68333, 0.16389, 0, 0.74333],
    "931": [0, 0.68333, 0.12028, 0, 0.71555],
    "933": [0, 0.68333, 0.11111, 0, 0.76666],
    "934": [0, 0.68333, 0.05986, 0, 0.71555],
    "936": [0, 0.68333, 0.11111, 0, 0.76666],
    "937": [0, 0.68333, 0.10257, 0, 0.71555],
    "8211": [0, 0.43056, 0.09208, 0, 0.51111],
    "8212": [0, 0.43056, 0.09208, 0, 1.02222],
    "8216": [0, 0.69444, 0.12417, 0, 0.30667],
    "8217": [0, 0.69444, 0.12417, 0, 0.30667],
    "8220": [0, 0.69444, 0.1685, 0, 0.51444],
    "8221": [0, 0.69444, 0.06961, 0, 0.51444],
    "8463": [0, 0.68889, 0, 0, 0.54028]
  },
  "Main-Regular": {
    "32": [0, 0, 0, 0, 0.25],
    "33": [0, 0.69444, 0, 0, 0.27778],
    "34": [0, 0.69444, 0, 0, 0.5],
    "35": [0.19444, 0.69444, 0, 0, 0.83334],
    "36": [0.05556, 0.75, 0, 0, 0.5],
    "37": [0.05556, 0.75, 0, 0, 0.83334],
    "38": [0, 0.69444, 0, 0, 0.77778],
    "39": [0, 0.69444, 0, 0, 0.27778],
    "40": [0.25, 0.75, 0, 0, 0.38889],
    "41": [0.25, 0.75, 0, 0, 0.38889],
    "42": [0, 0.75, 0, 0, 0.5],
    "43": [0.08333, 0.58333, 0, 0, 0.77778],
    "44": [0.19444, 0.10556, 0, 0, 0.27778],
    "45": [0, 0.43056, 0, 0, 0.33333],
    "46": [0, 0.10556, 0, 0, 0.27778],
    "47": [0.25, 0.75, 0, 0, 0.5],
    "48": [0, 0.64444, 0, 0, 0.5],
    "49": [0, 0.64444, 0, 0, 0.5],
    "50": [0, 0.64444, 0, 0, 0.5],
    "51": [0, 0.64444, 0, 0, 0.5],
    "52": [0, 0.64444, 0, 0, 0.5],
    "53": [0, 0.64444, 0, 0, 0.5],
    "54": [0, 0.64444, 0, 0, 0.5],
    "55": [0, 0.64444, 0, 0, 0.5],
    "56": [0, 0.64444, 0, 0, 0.5],
    "57": [0, 0.64444, 0, 0, 0.5],
    "58": [0, 0.43056, 0, 0, 0.27778],
    "59": [0.19444, 0.43056, 0, 0, 0.27778],
    "60": [0.0391, 0.5391, 0, 0, 0.77778],
    "61": [-0.13313, 0.36687, 0, 0, 0.77778],
    "62": [0.0391, 0.5391, 0, 0, 0.77778],
    "63": [0, 0.69444, 0, 0, 0.47222],
    "64": [0, 0.69444, 0, 0, 0.77778],
    "65": [0, 0.68333, 0, 0, 0.75],
    "66": [0, 0.68333, 0, 0, 0.70834],
    "67": [0, 0.68333, 0, 0, 0.72222],
    "68": [0, 0.68333, 0, 0, 0.76389],
    "69": [0, 0.68333, 0, 0, 0.68056],
    "70": [0, 0.68333, 0, 0, 0.65278],
    "71": [0, 0.68333, 0, 0, 0.78472],
    "72": [0, 0.68333, 0, 0, 0.75],
    "73": [0, 0.68333, 0, 0, 0.36111],
    "74": [0, 0.68333, 0, 0, 0.51389],
    "75": [0, 0.68333, 0, 0, 0.77778],
    "76": [0, 0.68333, 0, 0, 0.625],
    "77": [0, 0.68333, 0, 0, 0.91667],
    "78": [0, 0.68333, 0, 0, 0.75],
    "79": [0, 0.68333, 0, 0, 0.77778],
    "80": [0, 0.68333, 0, 0, 0.68056],
    "81": [0.19444, 0.68333, 0, 0, 0.77778],
    "82": [0, 0.68333, 0, 0, 0.73611],
    "83": [0, 0.68333, 0, 0, 0.55556],
    "84": [0, 0.68333, 0, 0, 0.72222],
    "85": [0, 0.68333, 0, 0, 0.75],
    "86": [0, 0.68333, 0.01389, 0, 0.75],
    "87": [0, 0.68333, 0.01389, 0, 1.02778],
    "88": [0, 0.68333, 0, 0, 0.75],
    "89": [0, 0.68333, 0.025, 0, 0.75],
    "90": [0, 0.68333, 0, 0, 0.61111],
    "91": [0.25, 0.75, 0, 0, 0.27778],
    "92": [0.25, 0.75, 0, 0, 0.5],
    "93": [0.25, 0.75, 0, 0, 0.27778],
    "94": [0, 0.69444, 0, 0, 0.5],
    "95": [0.31, 0.12056, 0.02778, 0, 0.5],
    "97": [0, 0.43056, 0, 0, 0.5],
    "98": [0, 0.69444, 0, 0, 0.55556],
    "99": [0, 0.43056, 0, 0, 0.44445],
    "100": [0, 0.69444, 0, 0, 0.55556],
    "101": [0, 0.43056, 0, 0, 0.44445],
    "102": [0, 0.69444, 0.07778, 0, 0.30556],
    "103": [0.19444, 0.43056, 0.01389, 0, 0.5],
    "104": [0, 0.69444, 0, 0, 0.55556],
    "105": [0, 0.66786, 0, 0, 0.27778],
    "106": [0.19444, 0.66786, 0, 0, 0.30556],
    "107": [0, 0.69444, 0, 0, 0.52778],
    "108": [0, 0.69444, 0, 0, 0.27778],
    "109": [0, 0.43056, 0, 0, 0.83334],
    "110": [0, 0.43056, 0, 0, 0.55556],
    "111": [0, 0.43056, 0, 0, 0.5],
    "112": [0.19444, 0.43056, 0, 0, 0.55556],
    "113": [0.19444, 0.43056, 0, 0, 0.52778],
    "114": [0, 0.43056, 0, 0, 0.39167],
    "115": [0, 0.43056, 0, 0, 0.39445],
    "116": [0, 0.61508, 0, 0, 0.38889],
    "117": [0, 0.43056, 0, 0, 0.55556],
    "118": [0, 0.43056, 0.01389, 0, 0.52778],
    "119": [0, 0.43056, 0.01389, 0, 0.72222],
    "120": [0, 0.43056, 0, 0, 0.52778],
    "121": [0.19444, 0.43056, 0.01389, 0, 0.52778],
    "122": [0, 0.43056, 0, 0, 0.44445],
    "123": [0.25, 0.75, 0, 0, 0.5],
    "124": [0.25, 0.75, 0, 0, 0.27778],
    "125": [0.25, 0.75, 0, 0, 0.5],
    "126": [0.35, 0.31786, 0, 0, 0.5],
    "160": [0, 0, 0, 0, 0.25],
    "167": [0.19444, 0.69444, 0, 0, 0.44445],
    "168": [0, 0.66786, 0, 0, 0.5],
    "172": [0, 0.43056, 0, 0, 0.66667],
    "176": [0, 0.69444, 0, 0, 0.75],
    "177": [0.08333, 0.58333, 0, 0, 0.77778],
    "182": [0.19444, 0.69444, 0, 0, 0.61111],
    "184": [0.17014, 0, 0, 0, 0.44445],
    "198": [0, 0.68333, 0, 0, 0.90278],
    "215": [0.08333, 0.58333, 0, 0, 0.77778],
    "216": [0.04861, 0.73194, 0, 0, 0.77778],
    "223": [0, 0.69444, 0, 0, 0.5],
    "230": [0, 0.43056, 0, 0, 0.72222],
    "247": [0.08333, 0.58333, 0, 0, 0.77778],
    "248": [0.09722, 0.52778, 0, 0, 0.5],
    "305": [0, 0.43056, 0, 0, 0.27778],
    "338": [0, 0.68333, 0, 0, 1.01389],
    "339": [0, 0.43056, 0, 0, 0.77778],
    "567": [0.19444, 0.43056, 0, 0, 0.30556],
    "710": [0, 0.69444, 0, 0, 0.5],
    "711": [0, 0.62847, 0, 0, 0.5],
    "713": [0, 0.56778, 0, 0, 0.5],
    "714": [0, 0.69444, 0, 0, 0.5],
    "715": [0, 0.69444, 0, 0, 0.5],
    "728": [0, 0.69444, 0, 0, 0.5],
    "729": [0, 0.66786, 0, 0, 0.27778],
    "730": [0, 0.69444, 0, 0, 0.75],
    "732": [0, 0.66786, 0, 0, 0.5],
    "733": [0, 0.69444, 0, 0, 0.5],
    "915": [0, 0.68333, 0, 0, 0.625],
    "916": [0, 0.68333, 0, 0, 0.83334],
    "920": [0, 0.68333, 0, 0, 0.77778],
    "923": [0, 0.68333, 0, 0, 0.69445],
    "926": [0, 0.68333, 0, 0, 0.66667],
    "928": [0, 0.68333, 0, 0, 0.75],
    "931": [0, 0.68333, 0, 0, 0.72222],
    "933": [0, 0.68333, 0, 0, 0.77778],
    "934": [0, 0.68333, 0, 0, 0.72222],
    "936": [0, 0.68333, 0, 0, 0.77778],
    "937": [0, 0.68333, 0, 0, 0.72222],
    "8211": [0, 0.43056, 0.02778, 0, 0.5],
    "8212": [0, 0.43056, 0.02778, 0, 1.0],
    "8216": [0, 0.69444, 0, 0, 0.27778],
    "8217": [0, 0.69444, 0, 0, 0.27778],
    "8220": [0, 0.69444, 0, 0, 0.5],
    "8221": [0, 0.69444, 0, 0, 0.5],
    "8224": [0.19444, 0.69444, 0, 0, 0.44445],
    "8225": [0.19444, 0.69444, 0, 0, 0.44445],
    "8230": [0, 0.12, 0, 0, 1.172],
    "8242": [0, 0.55556, 0, 0, 0.275],
    "8407": [0, 0.71444, 0.15382, 0, 0.5],
    "8463": [0, 0.68889, 0, 0, 0.54028],
    "8465": [0, 0.69444, 0, 0, 0.72222],
    "8467": [0, 0.69444, 0, 0.11111, 0.41667],
    "8472": [0.19444, 0.43056, 0, 0.11111, 0.63646],
    "8476": [0, 0.69444, 0, 0, 0.72222],
    "8501": [0, 0.69444, 0, 0, 0.61111],
    "8592": [-0.13313, 0.36687, 0, 0, 1.0],
    "8593": [0.19444, 0.69444, 0, 0, 0.5],
    "8594": [-0.13313, 0.36687, 0, 0, 1.0],
    "8595": [0.19444, 0.69444, 0, 0, 0.5],
    "8596": [-0.13313, 0.36687, 0, 0, 1.0],
    "8597": [0.25, 0.75, 0, 0, 0.5],
    "8598": [0.19444, 0.69444, 0, 0, 1.0],
    "8599": [0.19444, 0.69444, 0, 0, 1.0],
    "8600": [0.19444, 0.69444, 0, 0, 1.0],
    "8601": [0.19444, 0.69444, 0, 0, 1.0],
    "8614": [0.011, 0.511, 0, 0, 1.0],
    "8617": [0.011, 0.511, 0, 0, 1.126],
    "8618": [0.011, 0.511, 0, 0, 1.126],
    "8636": [-0.13313, 0.36687, 0, 0, 1.0],
    "8637": [-0.13313, 0.36687, 0, 0, 1.0],
    "8640": [-0.13313, 0.36687, 0, 0, 1.0],
    "8641": [-0.13313, 0.36687, 0, 0, 1.0],
    "8652": [0.011, 0.671, 0, 0, 1.0],
    "8656": [-0.13313, 0.36687, 0, 0, 1.0],
    "8657": [0.19444, 0.69444, 0, 0, 0.61111],
    "8658": [-0.13313, 0.36687, 0, 0, 1.0],
    "8659": [0.19444, 0.69444, 0, 0, 0.61111],
    "8660": [-0.13313, 0.36687, 0, 0, 1.0],
    "8661": [0.25, 0.75, 0, 0, 0.61111],
    "8704": [0, 0.69444, 0, 0, 0.55556],
    "8706": [0, 0.69444, 0.05556, 0.08334, 0.5309],
    "8707": [0, 0.69444, 0, 0, 0.55556],
    "8709": [0.05556, 0.75, 0, 0, 0.5],
    "8711": [0, 0.68333, 0, 0, 0.83334],
    "8712": [0.0391, 0.5391, 0, 0, 0.66667],
    "8715": [0.0391, 0.5391, 0, 0, 0.66667],
    "8722": [0.08333, 0.58333, 0, 0, 0.77778],
    "8723": [0.08333, 0.58333, 0, 0, 0.77778],
    "8725": [0.25, 0.75, 0, 0, 0.5],
    "8726": [0.25, 0.75, 0, 0, 0.5],
    "8727": [-0.03472, 0.46528, 0, 0, 0.5],
    "8728": [-0.05555, 0.44445, 0, 0, 0.5],
    "8729": [-0.05555, 0.44445, 0, 0, 0.5],
    "8730": [0.2, 0.8, 0, 0, 0.83334],
    "8733": [0, 0.43056, 0, 0, 0.77778],
    "8734": [0, 0.43056, 0, 0, 1.0],
    "8736": [0, 0.69224, 0, 0, 0.72222],
    "8739": [0.25, 0.75, 0, 0, 0.27778],
    "8741": [0.25, 0.75, 0, 0, 0.5],
    "8743": [0, 0.55556, 0, 0, 0.66667],
    "8744": [0, 0.55556, 0, 0, 0.66667],
    "8745": [0, 0.55556, 0, 0, 0.66667],
    "8746": [0, 0.55556, 0, 0, 0.66667],
    "8747": [0.19444, 0.69444, 0.11111, 0, 0.41667],
    "8764": [-0.13313, 0.36687, 0, 0, 0.77778],
    "8768": [0.19444, 0.69444, 0, 0, 0.27778],
    "8771": [-0.03625, 0.46375, 0, 0, 0.77778],
    "8773": [-0.022, 0.589, 0, 0, 1.0],
    "8776": [-0.01688, 0.48312, 0, 0, 0.77778],
    "8781": [-0.03625, 0.46375, 0, 0, 0.77778],
    "8784": [-0.133, 0.67, 0, 0, 0.778],
    "8801": [-0.03625, 0.46375, 0, 0, 0.77778],
    "8804": [0.13597, 0.63597, 0, 0, 0.77778],
    "8805": [0.13597, 0.63597, 0, 0, 0.77778],
    "8810": [0.0391, 0.5391, 0, 0, 1.0],
    "8811": [0.0391, 0.5391, 0, 0, 1.0],
    "8826": [0.0391, 0.5391, 0, 0, 0.77778],
    "8827": [0.0391, 0.5391, 0, 0, 0.77778],
    "8834": [0.0391, 0.5391, 0, 0, 0.77778],
    "8835": [0.0391, 0.5391, 0, 0, 0.77778],
    "8838": [0.13597, 0.63597, 0, 0, 0.77778],
    "8839": [0.13597, 0.63597, 0, 0, 0.77778],
    "8846": [0, 0.55556, 0, 0, 0.66667],
    "8849": [0.13597, 0.63597, 0, 0, 0.77778],
    "8850": [0.13597, 0.63597, 0, 0, 0.77778],
    "8851": [0, 0.55556, 0, 0, 0.66667],
    "8852": [0, 0.55556, 0, 0, 0.66667],
    "8853": [0.08333, 0.58333, 0, 0, 0.77778],
    "8854": [0.08333, 0.58333, 0, 0, 0.77778],
    "8855": [0.08333, 0.58333, 0, 0, 0.77778],
    "8856": [0.08333, 0.58333, 0, 0, 0.77778],
    "8857": [0.08333, 0.58333, 0, 0, 0.77778],
    "8866": [0, 0.69444, 0, 0, 0.61111],
    "8867": [0, 0.69444, 0, 0, 0.61111],
    "8868": [0, 0.69444, 0, 0, 0.77778],
    "8869": [0, 0.69444, 0, 0, 0.77778],
    "8872": [0.249, 0.75, 0, 0, 0.867],
    "8900": [-0.05555, 0.44445, 0, 0, 0.5],
    "8901": [-0.05555, 0.44445, 0, 0, 0.27778],
    "8902": [-0.03472, 0.46528, 0, 0, 0.5],
    "8904": [0.005, 0.505, 0, 0, 0.9],
    "8942": [0.03, 0.9, 0, 0, 0.278],
    "8943": [-0.19, 0.31, 0, 0, 1.172],
    "8945": [-0.1, 0.82, 0, 0, 1.282],
    "8968": [0.25, 0.75, 0, 0, 0.44445],
    "8969": [0.25, 0.75, 0, 0, 0.44445],
    "8970": [0.25, 0.75, 0, 0, 0.44445],
    "8971": [0.25, 0.75, 0, 0, 0.44445],
    "8994": [-0.14236, 0.35764, 0, 0, 1.0],
    "8995": [-0.14236, 0.35764, 0, 0, 1.0],
    "9136": [0.244, 0.744, 0, 0, 0.412],
    "9137": [0.244, 0.744, 0, 0, 0.412],
    "9651": [0.19444, 0.69444, 0, 0, 0.88889],
    "9657": [-0.03472, 0.46528, 0, 0, 0.5],
    "9661": [0.19444, 0.69444, 0, 0, 0.88889],
    "9667": [-0.03472, 0.46528, 0, 0, 0.5],
    "9711": [0.19444, 0.69444, 0, 0, 1.0],
    "9824": [0.12963, 0.69444, 0, 0, 0.77778],
    "9825": [0.12963, 0.69444, 0, 0, 0.77778],
    "9826": [0.12963, 0.69444, 0, 0, 0.77778],
    "9827": [0.12963, 0.69444, 0, 0, 0.77778],
    "9837": [0, 0.75, 0, 0, 0.38889],
    "9838": [0.19444, 0.69444, 0, 0, 0.38889],
    "9839": [0.19444, 0.69444, 0, 0, 0.38889],
    "10216": [0.25, 0.75, 0, 0, 0.38889],
    "10217": [0.25, 0.75, 0, 0, 0.38889],
    "10222": [0.244, 0.744, 0, 0, 0.412],
    "10223": [0.244, 0.744, 0, 0, 0.412],
    "10229": [0.011, 0.511, 0, 0, 1.609],
    "10230": [0.011, 0.511, 0, 0, 1.638],
    "10231": [0.011, 0.511, 0, 0, 1.859],
    "10232": [0.024, 0.525, 0, 0, 1.609],
    "10233": [0.024, 0.525, 0, 0, 1.638],
    "10234": [0.024, 0.525, 0, 0, 1.858],
    "10236": [0.011, 0.511, 0, 0, 1.638],
    "10815": [0, 0.68333, 0, 0, 0.75],
    "10927": [0.13597, 0.63597, 0, 0, 0.77778],
    "10928": [0.13597, 0.63597, 0, 0, 0.77778],
    "57376": [0.19444, 0.69444, 0, 0, 0]
  },
  "Math-BoldItalic": {
    "65": [0, 0.68611, 0, 0, 0.86944],
    "66": [0, 0.68611, 0.04835, 0, 0.8664],
    "67": [0, 0.68611, 0.06979, 0, 0.81694],
    "68": [0, 0.68611, 0.03194, 0, 0.93812],
    "69": [0, 0.68611, 0.05451, 0, 0.81007],
    "70": [0, 0.68611, 0.15972, 0, 0.68889],
    "71": [0, 0.68611, 0, 0, 0.88673],
    "72": [0, 0.68611, 0.08229, 0, 0.98229],
    "73": [0, 0.68611, 0.07778, 0, 0.51111],
    "74": [0, 0.68611, 0.10069, 0, 0.63125],
    "75": [0, 0.68611, 0.06979, 0, 0.97118],
    "76": [0, 0.68611, 0, 0, 0.75555],
    "77": [0, 0.68611, 0.11424, 0, 1.14201],
    "78": [0, 0.68611, 0.11424, 0, 0.95034],
    "79": [0, 0.68611, 0.03194, 0, 0.83666],
    "80": [0, 0.68611, 0.15972, 0, 0.72309],
    "81": [0.19444, 0.68611, 0, 0, 0.86861],
    "82": [0, 0.68611, 0.00421, 0, 0.87235],
    "83": [0, 0.68611, 0.05382, 0, 0.69271],
    "84": [0, 0.68611, 0.15972, 0, 0.63663],
    "85": [0, 0.68611, 0.11424, 0, 0.80027],
    "86": [0, 0.68611, 0.25555, 0, 0.67778],
    "87": [0, 0.68611, 0.15972, 0, 1.09305],
    "88": [0, 0.68611, 0.07778, 0, 0.94722],
    "89": [0, 0.68611, 0.25555, 0, 0.67458],
    "90": [0, 0.68611, 0.06979, 0, 0.77257],
    "97": [0, 0.44444, 0, 0, 0.63287],
    "98": [0, 0.69444, 0, 0, 0.52083],
    "99": [0, 0.44444, 0, 0, 0.51342],
    "100": [0, 0.69444, 0, 0, 0.60972],
    "101": [0, 0.44444, 0, 0, 0.55361],
    "102": [0.19444, 0.69444, 0.11042, 0, 0.56806],
    "103": [0.19444, 0.44444, 0.03704, 0, 0.5449],
    "104": [0, 0.69444, 0, 0, 0.66759],
    "105": [0, 0.69326, 0, 0, 0.4048],
    "106": [0.19444, 0.69326, 0.0622, 0, 0.47083],
    "107": [0, 0.69444, 0.01852, 0, 0.6037],
    "108": [0, 0.69444, 0.0088, 0, 0.34815],
    "109": [0, 0.44444, 0, 0, 1.0324],
    "110": [0, 0.44444, 0, 0, 0.71296],
    "111": [0, 0.44444, 0, 0, 0.58472],
    "112": [0.19444, 0.44444, 0, 0, 0.60092],
    "113": [0.19444, 0.44444, 0.03704, 0, 0.54213],
    "114": [0, 0.44444, 0.03194, 0, 0.5287],
    "115": [0, 0.44444, 0, 0, 0.53125],
    "116": [0, 0.63492, 0, 0, 0.41528],
    "117": [0, 0.44444, 0, 0, 0.68102],
    "118": [0, 0.44444, 0.03704, 0, 0.56666],
    "119": [0, 0.44444, 0.02778, 0, 0.83148],
    "120": [0, 0.44444, 0, 0, 0.65903],
    "121": [0.19444, 0.44444, 0.03704, 0, 0.59028],
    "122": [0, 0.44444, 0.04213, 0, 0.55509],
    "915": [0, 0.68611, 0.15972, 0, 0.65694],
    "916": [0, 0.68611, 0, 0, 0.95833],
    "920": [0, 0.68611, 0.03194, 0, 0.86722],
    "923": [0, 0.68611, 0, 0, 0.80555],
    "926": [0, 0.68611, 0.07458, 0, 0.84125],
    "928": [0, 0.68611, 0.08229, 0, 0.98229],
    "931": [0, 0.68611, 0.05451, 0, 0.88507],
    "933": [0, 0.68611, 0.15972, 0, 0.67083],
    "934": [0, 0.68611, 0, 0, 0.76666],
    "936": [0, 0.68611, 0.11653, 0, 0.71402],
    "937": [0, 0.68611, 0.04835, 0, 0.8789],
    "945": [0, 0.44444, 0, 0, 0.76064],
    "946": [0.19444, 0.69444, 0.03403, 0, 0.65972],
    "947": [0.19444, 0.44444, 0.06389, 0, 0.59003],
    "948": [0, 0.69444, 0.03819, 0, 0.52222],
    "949": [0, 0.44444, 0, 0, 0.52882],
    "950": [0.19444, 0.69444, 0.06215, 0, 0.50833],
    "951": [0.19444, 0.44444, 0.03704, 0, 0.6],
    "952": [0, 0.69444, 0.03194, 0, 0.5618],
    "953": [0, 0.44444, 0, 0, 0.41204],
    "954": [0, 0.44444, 0, 0, 0.66759],
    "955": [0, 0.69444, 0, 0, 0.67083],
    "956": [0.19444, 0.44444, 0, 0, 0.70787],
    "957": [0, 0.44444, 0.06898, 0, 0.57685],
    "958": [0.19444, 0.69444, 0.03021, 0, 0.50833],
    "959": [0, 0.44444, 0, 0, 0.58472],
    "960": [0, 0.44444, 0.03704, 0, 0.68241],
    "961": [0.19444, 0.44444, 0, 0, 0.6118],
    "962": [0.09722, 0.44444, 0.07917, 0, 0.42361],
    "963": [0, 0.44444, 0.03704, 0, 0.68588],
    "964": [0, 0.44444, 0.13472, 0, 0.52083],
    "965": [0, 0.44444, 0.03704, 0, 0.63055],
    "966": [0.19444, 0.44444, 0, 0, 0.74722],
    "967": [0.19444, 0.44444, 0, 0, 0.71805],
    "968": [0.19444, 0.69444, 0.03704, 0, 0.75833],
    "969": [0, 0.44444, 0.03704, 0, 0.71782],
    "977": [0, 0.69444, 0, 0, 0.69155],
    "981": [0.19444, 0.69444, 0, 0, 0.7125],
    "982": [0, 0.44444, 0.03194, 0, 0.975],
    "1009": [0.19444, 0.44444, 0, 0, 0.6118],
    "1013": [0, 0.44444, 0, 0, 0.48333]
  },
  "Math-Italic": {
    "65": [0, 0.68333, 0, 0.13889, 0.75],
    "66": [0, 0.68333, 0.05017, 0.08334, 0.75851],
    "67": [0, 0.68333, 0.07153, 0.08334, 0.71472],
    "68": [0, 0.68333, 0.02778, 0.05556, 0.82792],
    "69": [0, 0.68333, 0.05764, 0.08334, 0.7382],
    "70": [0, 0.68333, 0.13889, 0.08334, 0.64306],
    "71": [0, 0.68333, 0, 0.08334, 0.78625],
    "72": [0, 0.68333, 0.08125, 0.05556, 0.83125],
    "73": [0, 0.68333, 0.07847, 0.11111, 0.43958],
    "74": [0, 0.68333, 0.09618, 0.16667, 0.55451],
    "75": [0, 0.68333, 0.07153, 0.05556, 0.84931],
    "76": [0, 0.68333, 0, 0.02778, 0.68056],
    "77": [0, 0.68333, 0.10903, 0.08334, 0.97014],
    "78": [0, 0.68333, 0.10903, 0.08334, 0.80347],
    "79": [0, 0.68333, 0.02778, 0.08334, 0.76278],
    "80": [0, 0.68333, 0.13889, 0.08334, 0.64201],
    "81": [0.19444, 0.68333, 0, 0.08334, 0.79056],
    "82": [0, 0.68333, 0.00773, 0.08334, 0.75929],
    "83": [0, 0.68333, 0.05764, 0.08334, 0.6132],
    "84": [0, 0.68333, 0.13889, 0.08334, 0.58438],
    "85": [0, 0.68333, 0.10903, 0.02778, 0.68278],
    "86": [0, 0.68333, 0.22222, 0, 0.58333],
    "87": [0, 0.68333, 0.13889, 0, 0.94445],
    "88": [0, 0.68333, 0.07847, 0.08334, 0.82847],
    "89": [0, 0.68333, 0.22222, 0, 0.58056],
    "90": [0, 0.68333, 0.07153, 0.08334, 0.68264],
    "97": [0, 0.43056, 0, 0, 0.52859],
    "98": [0, 0.69444, 0, 0, 0.42917],
    "99": [0, 0.43056, 0, 0.05556, 0.43276],
    "100": [0, 0.69444, 0, 0.16667, 0.52049],
    "101": [0, 0.43056, 0, 0.05556, 0.46563],
    "102": [0.19444, 0.69444, 0.10764, 0.16667, 0.48959],
    "103": [0.19444, 0.43056, 0.03588, 0.02778, 0.47697],
    "104": [0, 0.69444, 0, 0, 0.57616],
    "105": [0, 0.65952, 0, 0, 0.34451],
    "106": [0.19444, 0.65952, 0.05724, 0, 0.41181],
    "107": [0, 0.69444, 0.03148, 0, 0.5206],
    "108": [0, 0.69444, 0.01968, 0.08334, 0.29838],
    "109": [0, 0.43056, 0, 0, 0.87801],
    "110": [0, 0.43056, 0, 0, 0.60023],
    "111": [0, 0.43056, 0, 0.05556, 0.48472],
    "112": [0.19444, 0.43056, 0, 0.08334, 0.50313],
    "113": [0.19444, 0.43056, 0.03588, 0.08334, 0.44641],
    "114": [0, 0.43056, 0.02778, 0.05556, 0.45116],
    "115": [0, 0.43056, 0, 0.05556, 0.46875],
    "116": [0, 0.61508, 0, 0.08334, 0.36111],
    "117": [0, 0.43056, 0, 0.02778, 0.57246],
    "118": [0, 0.43056, 0.03588, 0.02778, 0.48472],
    "119": [0, 0.43056, 0.02691, 0.08334, 0.71592],
    "120": [0, 0.43056, 0, 0.02778, 0.57153],
    "121": [0.19444, 0.43056, 0.03588, 0.05556, 0.49028],
    "122": [0, 0.43056, 0.04398, 0.05556, 0.46505],
    "915": [0, 0.68333, 0.13889, 0.08334, 0.61528],
    "916": [0, 0.68333, 0, 0.16667, 0.83334],
    "920": [0, 0.68333, 0.02778, 0.08334, 0.76278],
    "923": [0, 0.68333, 0, 0.16667, 0.69445],
    "926": [0, 0.68333, 0.07569, 0.08334, 0.74236],
    "928": [0, 0.68333, 0.08125, 0.05556, 0.83125],
    "931": [0, 0.68333, 0.05764, 0.08334, 0.77986],
    "933": [0, 0.68333, 0.13889, 0.05556, 0.58333],
    "934": [0, 0.68333, 0, 0.08334, 0.66667],
    "936": [0, 0.68333, 0.11, 0.05556, 0.61222],
    "937": [0, 0.68333, 0.05017, 0.08334, 0.7724],
    "945": [0, 0.43056, 0.0037, 0.02778, 0.6397],
    "946": [0.19444, 0.69444, 0.05278, 0.08334, 0.56563],
    "947": [0.19444, 0.43056, 0.05556, 0, 0.51773],
    "948": [0, 0.69444, 0.03785, 0.05556, 0.44444],
    "949": [0, 0.43056, 0, 0.08334, 0.46632],
    "950": [0.19444, 0.69444, 0.07378, 0.08334, 0.4375],
    "951": [0.19444, 0.43056, 0.03588, 0.05556, 0.49653],
    "952": [0, 0.69444, 0.02778, 0.08334, 0.46944],
    "953": [0, 0.43056, 0, 0.05556, 0.35394],
    "954": [0, 0.43056, 0, 0, 0.57616],
    "955": [0, 0.69444, 0, 0, 0.58334],
    "956": [0.19444, 0.43056, 0, 0.02778, 0.60255],
    "957": [0, 0.43056, 0.06366, 0.02778, 0.49398],
    "958": [0.19444, 0.69444, 0.04601, 0.11111, 0.4375],
    "959": [0, 0.43056, 0, 0.05556, 0.48472],
    "960": [0, 0.43056, 0.03588, 0, 0.57003],
    "961": [0.19444, 0.43056, 0, 0.08334, 0.51702],
    "962": [0.09722, 0.43056, 0.07986, 0.08334, 0.36285],
    "963": [0, 0.43056, 0.03588, 0, 0.57141],
    "964": [0, 0.43056, 0.1132, 0.02778, 0.43715],
    "965": [0, 0.43056, 0.03588, 0.02778, 0.54028],
    "966": [0.19444, 0.43056, 0, 0.08334, 0.65417],
    "967": [0.19444, 0.43056, 0, 0.05556, 0.62569],
    "968": [0.19444, 0.69444, 0.03588, 0.11111, 0.65139],
    "969": [0, 0.43056, 0.03588, 0, 0.62245],
    "977": [0, 0.69444, 0, 0.08334, 0.59144],
    "981": [0.19444, 0.69444, 0, 0.08334, 0.59583],
    "982": [0, 0.43056, 0.02778, 0, 0.82813],
    "1009": [0.19444, 0.43056, 0, 0.08334, 0.51702],
    "1013": [0, 0.43056, 0, 0.05556, 0.4059]
  },
  "Math-Regular": {
    "65": [0, 0.68333, 0, 0.13889, 0.75],
    "66": [0, 0.68333, 0.05017, 0.08334, 0.75851],
    "67": [0, 0.68333, 0.07153, 0.08334, 0.71472],
    "68": [0, 0.68333, 0.02778, 0.05556, 0.82792],
    "69": [0, 0.68333, 0.05764, 0.08334, 0.7382],
    "70": [0, 0.68333, 0.13889, 0.08334, 0.64306],
    "71": [0, 0.68333, 0, 0.08334, 0.78625],
    "72": [0, 0.68333, 0.08125, 0.05556, 0.83125],
    "73": [0, 0.68333, 0.07847, 0.11111, 0.43958],
    "74": [0, 0.68333, 0.09618, 0.16667, 0.55451],
    "75": [0, 0.68333, 0.07153, 0.05556, 0.84931],
    "76": [0, 0.68333, 0, 0.02778, 0.68056],
    "77": [0, 0.68333, 0.10903, 0.08334, 0.97014],
    "78": [0, 0.68333, 0.10903, 0.08334, 0.80347],
    "79": [0, 0.68333, 0.02778, 0.08334, 0.76278],
    "80": [0, 0.68333, 0.13889, 0.08334, 0.64201],
    "81": [0.19444, 0.68333, 0, 0.08334, 0.79056],
    "82": [0, 0.68333, 0.00773, 0.08334, 0.75929],
    "83": [0, 0.68333, 0.05764, 0.08334, 0.6132],
    "84": [0, 0.68333, 0.13889, 0.08334, 0.58438],
    "85": [0, 0.68333, 0.10903, 0.02778, 0.68278],
    "86": [0, 0.68333, 0.22222, 0, 0.58333],
    "87": [0, 0.68333, 0.13889, 0, 0.94445],
    "88": [0, 0.68333, 0.07847, 0.08334, 0.82847],
    "89": [0, 0.68333, 0.22222, 0, 0.58056],
    "90": [0, 0.68333, 0.07153, 0.08334, 0.68264],
    "97": [0, 0.43056, 0, 0, 0.52859],
    "98": [0, 0.69444, 0, 0, 0.42917],
    "99": [0, 0.43056, 0, 0.05556, 0.43276],
    "100": [0, 0.69444, 0, 0.16667, 0.52049],
    "101": [0, 0.43056, 0, 0.05556, 0.46563],
    "102": [0.19444, 0.69444, 0.10764, 0.16667, 0.48959],
    "103": [0.19444, 0.43056, 0.03588, 0.02778, 0.47697],
    "104": [0, 0.69444, 0, 0, 0.57616],
    "105": [0, 0.65952, 0, 0, 0.34451],
    "106": [0.19444, 0.65952, 0.05724, 0, 0.41181],
    "107": [0, 0.69444, 0.03148, 0, 0.5206],
    "108": [0, 0.69444, 0.01968, 0.08334, 0.29838],
    "109": [0, 0.43056, 0, 0, 0.87801],
    "110": [0, 0.43056, 0, 0, 0.60023],
    "111": [0, 0.43056, 0, 0.05556, 0.48472],
    "112": [0.19444, 0.43056, 0, 0.08334, 0.50313],
    "113": [0.19444, 0.43056, 0.03588, 0.08334, 0.44641],
    "114": [0, 0.43056, 0.02778, 0.05556, 0.45116],
    "115": [0, 0.43056, 0, 0.05556, 0.46875],
    "116": [0, 0.61508, 0, 0.08334, 0.36111],
    "117": [0, 0.43056, 0, 0.02778, 0.57246],
    "118": [0, 0.43056, 0.03588, 0.02778, 0.48472],
    "119": [0, 0.43056, 0.02691, 0.08334, 0.71592],
    "120": [0, 0.43056, 0, 0.02778, 0.57153],
    "121": [0.19444, 0.43056, 0.03588, 0.05556, 0.49028],
    "122": [0, 0.43056, 0.04398, 0.05556, 0.46505],
    "915": [0, 0.68333, 0.13889, 0.08334, 0.61528],
    "916": [0, 0.68333, 0, 0.16667, 0.83334],
    "920": [0, 0.68333, 0.02778, 0.08334, 0.76278],
    "923": [0, 0.68333, 0, 0.16667, 0.69445],
    "926": [0, 0.68333, 0.07569, 0.08334, 0.74236],
    "928": [0, 0.68333, 0.08125, 0.05556, 0.83125],
    "931": [0, 0.68333, 0.05764, 0.08334, 0.77986],
    "933": [0, 0.68333, 0.13889, 0.05556, 0.58333],
    "934": [0, 0.68333, 0, 0.08334, 0.66667],
    "936": [0, 0.68333, 0.11, 0.05556, 0.61222],
    "937": [0, 0.68333, 0.05017, 0.08334, 0.7724],
    "945": [0, 0.43056, 0.0037, 0.02778, 0.6397],
    "946": [0.19444, 0.69444, 0.05278, 0.08334, 0.56563],
    "947": [0.19444, 0.43056, 0.05556, 0, 0.51773],
    "948": [0, 0.69444, 0.03785, 0.05556, 0.44444],
    "949": [0, 0.43056, 0, 0.08334, 0.46632],
    "950": [0.19444, 0.69444, 0.07378, 0.08334, 0.4375],
    "951": [0.19444, 0.43056, 0.03588, 0.05556, 0.49653],
    "952": [0, 0.69444, 0.02778, 0.08334, 0.46944],
    "953": [0, 0.43056, 0, 0.05556, 0.35394],
    "954": [0, 0.43056, 0, 0, 0.57616],
    "955": [0, 0.69444, 0, 0, 0.58334],
    "956": [0.19444, 0.43056, 0, 0.02778, 0.60255],
    "957": [0, 0.43056, 0.06366, 0.02778, 0.49398],
    "958": [0.19444, 0.69444, 0.04601, 0.11111, 0.4375],
    "959": [0, 0.43056, 0, 0.05556, 0.48472],
    "960": [0, 0.43056, 0.03588, 0, 0.57003],
    "961": [0.19444, 0.43056, 0, 0.08334, 0.51702],
    "962": [0.09722, 0.43056, 0.07986, 0.08334, 0.36285],
    "963": [0, 0.43056, 0.03588, 0, 0.57141],
    "964": [0, 0.43056, 0.1132, 0.02778, 0.43715],
    "965": [0, 0.43056, 0.03588, 0.02778, 0.54028],
    "966": [0.19444, 0.43056, 0, 0.08334, 0.65417],
    "967": [0.19444, 0.43056, 0, 0.05556, 0.62569],
    "968": [0.19444, 0.69444, 0.03588, 0.11111, 0.65139],
    "969": [0, 0.43056, 0.03588, 0, 0.62245],
    "977": [0, 0.69444, 0, 0.08334, 0.59144],
    "981": [0.19444, 0.69444, 0, 0.08334, 0.59583],
    "982": [0, 0.43056, 0.02778, 0, 0.82813],
    "1009": [0.19444, 0.43056, 0, 0.08334, 0.51702],
    "1013": [0, 0.43056, 0, 0.05556, 0.4059]
  },
  "SansSerif-Bold": {
    "33": [0, 0.69444, 0, 0, 0.36667],
    "34": [0, 0.69444, 0, 0, 0.55834],
    "35": [0.19444, 0.69444, 0, 0, 0.91667],
    "36": [0.05556, 0.75, 0, 0, 0.55],
    "37": [0.05556, 0.75, 0, 0, 1.02912],
    "38": [0, 0.69444, 0, 0, 0.83056],
    "39": [0, 0.69444, 0, 0, 0.30556],
    "40": [0.25, 0.75, 0, 0, 0.42778],
    "41": [0.25, 0.75, 0, 0, 0.42778],
    "42": [0, 0.75, 0, 0, 0.55],
    "43": [0.11667, 0.61667, 0, 0, 0.85556],
    "44": [0.10556, 0.13056, 0, 0, 0.30556],
    "45": [0, 0.45833, 0, 0, 0.36667],
    "46": [0, 0.13056, 0, 0, 0.30556],
    "47": [0.25, 0.75, 0, 0, 0.55],
    "48": [0, 0.69444, 0, 0, 0.55],
    "49": [0, 0.69444, 0, 0, 0.55],
    "50": [0, 0.69444, 0, 0, 0.55],
    "51": [0, 0.69444, 0, 0, 0.55],
    "52": [0, 0.69444, 0, 0, 0.55],
    "53": [0, 0.69444, 0, 0, 0.55],
    "54": [0, 0.69444, 0, 0, 0.55],
    "55": [0, 0.69444, 0, 0, 0.55],
    "56": [0, 0.69444, 0, 0, 0.55],
    "57": [0, 0.69444, 0, 0, 0.55],
    "58": [0, 0.45833, 0, 0, 0.30556],
    "59": [0.10556, 0.45833, 0, 0, 0.30556],
    "61": [-0.09375, 0.40625, 0, 0, 0.85556],
    "63": [0, 0.69444, 0, 0, 0.51945],
    "64": [0, 0.69444, 0, 0, 0.73334],
    "65": [0, 0.69444, 0, 0, 0.73334],
    "66": [0, 0.69444, 0, 0, 0.73334],
    "67": [0, 0.69444, 0, 0, 0.70278],
    "68": [0, 0.69444, 0, 0, 0.79445],
    "69": [0, 0.69444, 0, 0, 0.64167],
    "70": [0, 0.69444, 0, 0, 0.61111],
    "71": [0, 0.69444, 0, 0, 0.73334],
    "72": [0, 0.69444, 0, 0, 0.79445],
    "73": [0, 0.69444, 0, 0, 0.33056],
    "74": [0, 0.69444, 0, 0, 0.51945],
    "75": [0, 0.69444, 0, 0, 0.76389],
    "76": [0, 0.69444, 0, 0, 0.58056],
    "77": [0, 0.69444, 0, 0, 0.97778],
    "78": [0, 0.69444, 0, 0, 0.79445],
    "79": [0, 0.69444, 0, 0, 0.79445],
    "80": [0, 0.69444, 0, 0, 0.70278],
    "81": [0.10556, 0.69444, 0, 0, 0.79445],
    "82": [0, 0.69444, 0, 0, 0.70278],
    "83": [0, 0.69444, 0, 0, 0.61111],
    "84": [0, 0.69444, 0, 0, 0.73334],
    "85": [0, 0.69444, 0, 0, 0.76389],
    "86": [0, 0.69444, 0.01528, 0, 0.73334],
    "87": [0, 0.69444, 0.01528, 0, 1.03889],
    "88": [0, 0.69444, 0, 0, 0.73334],
    "89": [0, 0.69444, 0.0275, 0, 0.73334],
    "90": [0, 0.69444, 0, 0, 0.67223],
    "91": [0.25, 0.75, 0, 0, 0.34306],
    "93": [0.25, 0.75, 0, 0, 0.34306],
    "94": [0, 0.69444, 0, 0, 0.55],
    "95": [0.35, 0.10833, 0.03056, 0, 0.55],
    "97": [0, 0.45833, 0, 0, 0.525],
    "98": [0, 0.69444, 0, 0, 0.56111],
    "99": [0, 0.45833, 0, 0, 0.48889],
    "100": [0, 0.69444, 0, 0, 0.56111],
    "101": [0, 0.45833, 0, 0, 0.51111],
    "102": [0, 0.69444, 0.07639, 0, 0.33611],
    "103": [0.19444, 0.45833, 0.01528, 0, 0.55],
    "104": [0, 0.69444, 0, 0, 0.56111],
    "105": [0, 0.69444, 0, 0, 0.25556],
    "106": [0.19444, 0.69444, 0, 0, 0.28611],
    "107": [0, 0.69444, 0, 0, 0.53056],
    "108": [0, 0.69444, 0, 0, 0.25556],
    "109": [0, 0.45833, 0, 0, 0.86667],
    "110": [0, 0.45833, 0, 0, 0.56111],
    "111": [0, 0.45833, 0, 0, 0.55],
    "112": [0.19444, 0.45833, 0, 0, 0.56111],
    "113": [0.19444, 0.45833, 0, 0, 0.56111],
    "114": [0, 0.45833, 0.01528, 0, 0.37222],
    "115": [0, 0.45833, 0, 0, 0.42167],
    "116": [0, 0.58929, 0, 0, 0.40417],
    "117": [0, 0.45833, 0, 0, 0.56111],
    "118": [0, 0.45833, 0.01528, 0, 0.5],
    "119": [0, 0.45833, 0.01528, 0, 0.74445],
    "120": [0, 0.45833, 0, 0, 0.5],
    "121": [0.19444, 0.45833, 0.01528, 0, 0.5],
    "122": [0, 0.45833, 0, 0, 0.47639],
    "126": [0.35, 0.34444, 0, 0, 0.55],
    "168": [0, 0.69444, 0, 0, 0.55],
    "176": [0, 0.69444, 0, 0, 0.73334],
    "180": [0, 0.69444, 0, 0, 0.55],
    "184": [0.17014, 0, 0, 0, 0.48889],
    "305": [0, 0.45833, 0, 0, 0.25556],
    "567": [0.19444, 0.45833, 0, 0, 0.28611],
    "710": [0, 0.69444, 0, 0, 0.55],
    "711": [0, 0.63542, 0, 0, 0.55],
    "713": [0, 0.63778, 0, 0, 0.55],
    "728": [0, 0.69444, 0, 0, 0.55],
    "729": [0, 0.69444, 0, 0, 0.30556],
    "730": [0, 0.69444, 0, 0, 0.73334],
    "732": [0, 0.69444, 0, 0, 0.55],
    "733": [0, 0.69444, 0, 0, 0.55],
    "915": [0, 0.69444, 0, 0, 0.58056],
    "916": [0, 0.69444, 0, 0, 0.91667],
    "920": [0, 0.69444, 0, 0, 0.85556],
    "923": [0, 0.69444, 0, 0, 0.67223],
    "926": [0, 0.69444, 0, 0, 0.73334],
    "928": [0, 0.69444, 0, 0, 0.79445],
    "931": [0, 0.69444, 0, 0, 0.79445],
    "933": [0, 0.69444, 0, 0, 0.85556],
    "934": [0, 0.69444, 0, 0, 0.79445],
    "936": [0, 0.69444, 0, 0, 0.85556],
    "937": [0, 0.69444, 0, 0, 0.79445],
    "8211": [0, 0.45833, 0.03056, 0, 0.55],
    "8212": [0, 0.45833, 0.03056, 0, 1.10001],
    "8216": [0, 0.69444, 0, 0, 0.30556],
    "8217": [0, 0.69444, 0, 0, 0.30556],
    "8220": [0, 0.69444, 0, 0, 0.55834],
    "8221": [0, 0.69444, 0, 0, 0.55834]
  },
  "SansSerif-Italic": {
    "33": [0, 0.69444, 0.05733, 0, 0.31945],
    "34": [0, 0.69444, 0.00316, 0, 0.5],
    "35": [0.19444, 0.69444, 0.05087, 0, 0.83334],
    "36": [0.05556, 0.75, 0.11156, 0, 0.5],
    "37": [0.05556, 0.75, 0.03126, 0, 0.83334],
    "38": [0, 0.69444, 0.03058, 0, 0.75834],
    "39": [0, 0.69444, 0.07816, 0, 0.27778],
    "40": [0.25, 0.75, 0.13164, 0, 0.38889],
    "41": [0.25, 0.75, 0.02536, 0, 0.38889],
    "42": [0, 0.75, 0.11775, 0, 0.5],
    "43": [0.08333, 0.58333, 0.02536, 0, 0.77778],
    "44": [0.125, 0.08333, 0, 0, 0.27778],
    "45": [0, 0.44444, 0.01946, 0, 0.33333],
    "46": [0, 0.08333, 0, 0, 0.27778],
    "47": [0.25, 0.75, 0.13164, 0, 0.5],
    "48": [0, 0.65556, 0.11156, 0, 0.5],
    "49": [0, 0.65556, 0.11156, 0, 0.5],
    "50": [0, 0.65556, 0.11156, 0, 0.5],
    "51": [0, 0.65556, 0.11156, 0, 0.5],
    "52": [0, 0.65556, 0.11156, 0, 0.5],
    "53": [0, 0.65556, 0.11156, 0, 0.5],
    "54": [0, 0.65556, 0.11156, 0, 0.5],
    "55": [0, 0.65556, 0.11156, 0, 0.5],
    "56": [0, 0.65556, 0.11156, 0, 0.5],
    "57": [0, 0.65556, 0.11156, 0, 0.5],
    "58": [0, 0.44444, 0.02502, 0, 0.27778],
    "59": [0.125, 0.44444, 0.02502, 0, 0.27778],
    "61": [-0.13, 0.37, 0.05087, 0, 0.77778],
    "63": [0, 0.69444, 0.11809, 0, 0.47222],
    "64": [0, 0.69444, 0.07555, 0, 0.66667],
    "65": [0, 0.69444, 0, 0, 0.66667],
    "66": [0, 0.69444, 0.08293, 0, 0.66667],
    "67": [0, 0.69444, 0.11983, 0, 0.63889],
    "68": [0, 0.69444, 0.07555, 0, 0.72223],
    "69": [0, 0.69444, 0.11983, 0, 0.59722],
    "70": [0, 0.69444, 0.13372, 0, 0.56945],
    "71": [0, 0.69444, 0.11983, 0, 0.66667],
    "72": [0, 0.69444, 0.08094, 0, 0.70834],
    "73": [0, 0.69444, 0.13372, 0, 0.27778],
    "74": [0, 0.69444, 0.08094, 0, 0.47222],
    "75": [0, 0.69444, 0.11983, 0, 0.69445],
    "76": [0, 0.69444, 0, 0, 0.54167],
    "77": [0, 0.69444, 0.08094, 0, 0.875],
    "78": [0, 0.69444, 0.08094, 0, 0.70834],
    "79": [0, 0.69444, 0.07555, 0, 0.73611],
    "80": [0, 0.69444, 0.08293, 0, 0.63889],
    "81": [0.125, 0.69444, 0.07555, 0, 0.73611],
    "82": [0, 0.69444, 0.08293, 0, 0.64584],
    "83": [0, 0.69444, 0.09205, 0, 0.55556],
    "84": [0, 0.69444, 0.13372, 0, 0.68056],
    "85": [0, 0.69444, 0.08094, 0, 0.6875],
    "86": [0, 0.69444, 0.1615, 0, 0.66667],
    "87": [0, 0.69444, 0.1615, 0, 0.94445],
    "88": [0, 0.69444, 0.13372, 0, 0.66667],
    "89": [0, 0.69444, 0.17261, 0, 0.66667],
    "90": [0, 0.69444, 0.11983, 0, 0.61111],
    "91": [0.25, 0.75, 0.15942, 0, 0.28889],
    "93": [0.25, 0.75, 0.08719, 0, 0.28889],
    "94": [0, 0.69444, 0.0799, 0, 0.5],
    "95": [0.35, 0.09444, 0.08616, 0, 0.5],
    "97": [0, 0.44444, 0.00981, 0, 0.48056],
    "98": [0, 0.69444, 0.03057, 0, 0.51667],
    "99": [0, 0.44444, 0.08336, 0, 0.44445],
    "100": [0, 0.69444, 0.09483, 0, 0.51667],
    "101": [0, 0.44444, 0.06778, 0, 0.44445],
    "102": [0, 0.69444, 0.21705, 0, 0.30556],
    "103": [0.19444, 0.44444, 0.10836, 0, 0.5],
    "104": [0, 0.69444, 0.01778, 0, 0.51667],
    "105": [0, 0.67937, 0.09718, 0, 0.23889],
    "106": [0.19444, 0.67937, 0.09162, 0, 0.26667],
    "107": [0, 0.69444, 0.08336, 0, 0.48889],
    "108": [0, 0.69444, 0.09483, 0, 0.23889],
    "109": [0, 0.44444, 0.01778, 0, 0.79445],
    "110": [0, 0.44444, 0.01778, 0, 0.51667],
    "111": [0, 0.44444, 0.06613, 0, 0.5],
    "112": [0.19444, 0.44444, 0.0389, 0, 0.51667],
    "113": [0.19444, 0.44444, 0.04169, 0, 0.51667],
    "114": [0, 0.44444, 0.10836, 0, 0.34167],
    "115": [0, 0.44444, 0.0778, 0, 0.38333],
    "116": [0, 0.57143, 0.07225, 0, 0.36111],
    "117": [0, 0.44444, 0.04169, 0, 0.51667],
    "118": [0, 0.44444, 0.10836, 0, 0.46111],
    "119": [0, 0.44444, 0.10836, 0, 0.68334],
    "120": [0, 0.44444, 0.09169, 0, 0.46111],
    "121": [0.19444, 0.44444, 0.10836, 0, 0.46111],
    "122": [0, 0.44444, 0.08752, 0, 0.43472],
    "126": [0.35, 0.32659, 0.08826, 0, 0.5],
    "168": [0, 0.67937, 0.06385, 0, 0.5],
    "176": [0, 0.69444, 0, 0, 0.73752],
    "184": [0.17014, 0, 0, 0, 0.44445],
    "305": [0, 0.44444, 0.04169, 0, 0.23889],
    "567": [0.19444, 0.44444, 0.04169, 0, 0.26667],
    "710": [0, 0.69444, 0.0799, 0, 0.5],
    "711": [0, 0.63194, 0.08432, 0, 0.5],
    "713": [0, 0.60889, 0.08776, 0, 0.5],
    "714": [0, 0.69444, 0.09205, 0, 0.5],
    "715": [0, 0.69444, 0, 0, 0.5],
    "728": [0, 0.69444, 0.09483, 0, 0.5],
    "729": [0, 0.67937, 0.07774, 0, 0.27778],
    "730": [0, 0.69444, 0, 0, 0.73752],
    "732": [0, 0.67659, 0.08826, 0, 0.5],
    "733": [0, 0.69444, 0.09205, 0, 0.5],
    "915": [0, 0.69444, 0.13372, 0, 0.54167],
    "916": [0, 0.69444, 0, 0, 0.83334],
    "920": [0, 0.69444, 0.07555, 0, 0.77778],
    "923": [0, 0.69444, 0, 0, 0.61111],
    "926": [0, 0.69444, 0.12816, 0, 0.66667],
    "928": [0, 0.69444, 0.08094, 0, 0.70834],
    "931": [0, 0.69444, 0.11983, 0, 0.72222],
    "933": [0, 0.69444, 0.09031, 0, 0.77778],
    "934": [0, 0.69444, 0.04603, 0, 0.72222],
    "936": [0, 0.69444, 0.09031, 0, 0.77778],
    "937": [0, 0.69444, 0.08293, 0, 0.72222],
    "8211": [0, 0.44444, 0.08616, 0, 0.5],
    "8212": [0, 0.44444, 0.08616, 0, 1.0],
    "8216": [0, 0.69444, 0.07816, 0, 0.27778],
    "8217": [0, 0.69444, 0.07816, 0, 0.27778],
    "8220": [0, 0.69444, 0.14205, 0, 0.5],
    "8221": [0, 0.69444, 0.00316, 0, 0.5]
  },
  "SansSerif-Regular": {
    "33": [0, 0.69444, 0, 0, 0.31945],
    "34": [0, 0.69444, 0, 0, 0.5],
    "35": [0.19444, 0.69444, 0, 0, 0.83334],
    "36": [0.05556, 0.75, 0, 0, 0.5],
    "37": [0.05556, 0.75, 0, 0, 0.83334],
    "38": [0, 0.69444, 0, 0, 0.75834],
    "39": [0, 0.69444, 0, 0, 0.27778],
    "40": [0.25, 0.75, 0, 0, 0.38889],
    "41": [0.25, 0.75, 0, 0, 0.38889],
    "42": [0, 0.75, 0, 0, 0.5],
    "43": [0.08333, 0.58333, 0, 0, 0.77778],
    "44": [0.125, 0.08333, 0, 0, 0.27778],
    "45": [0, 0.44444, 0, 0, 0.33333],
    "46": [0, 0.08333, 0, 0, 0.27778],
    "47": [0.25, 0.75, 0, 0, 0.5],
    "48": [0, 0.65556, 0, 0, 0.5],
    "49": [0, 0.65556, 0, 0, 0.5],
    "50": [0, 0.65556, 0, 0, 0.5],
    "51": [0, 0.65556, 0, 0, 0.5],
    "52": [0, 0.65556, 0, 0, 0.5],
    "53": [0, 0.65556, 0, 0, 0.5],
    "54": [0, 0.65556, 0, 0, 0.5],
    "55": [0, 0.65556, 0, 0, 0.5],
    "56": [0, 0.65556, 0, 0, 0.5],
    "57": [0, 0.65556, 0, 0, 0.5],
    "58": [0, 0.44444, 0, 0, 0.27778],
    "59": [0.125, 0.44444, 0, 0, 0.27778],
    "61": [-0.13, 0.37, 0, 0, 0.77778],
    "63": [0, 0.69444, 0, 0, 0.47222],
    "64": [0, 0.69444, 0, 0, 0.66667],
    "65": [0, 0.69444, 0, 0, 0.66667],
    "66": [0, 0.69444, 0, 0, 0.66667],
    "67": [0, 0.69444, 0, 0, 0.63889],
    "68": [0, 0.69444, 0, 0, 0.72223],
    "69": [0, 0.69444, 0, 0, 0.59722],
    "70": [0, 0.69444, 0, 0, 0.56945],
    "71": [0, 0.69444, 0, 0, 0.66667],
    "72": [0, 0.69444, 0, 0, 0.70834],
    "73": [0, 0.69444, 0, 0, 0.27778],
    "74": [0, 0.69444, 0, 0, 0.47222],
    "75": [0, 0.69444, 0, 0, 0.69445],
    "76": [0, 0.69444, 0, 0, 0.54167],
    "77": [0, 0.69444, 0, 0, 0.875],
    "78": [0, 0.69444, 0, 0, 0.70834],
    "79": [0, 0.69444, 0, 0, 0.73611],
    "80": [0, 0.69444, 0, 0, 0.63889],
    "81": [0.125, 0.69444, 0, 0, 0.73611],
    "82": [0, 0.69444, 0, 0, 0.64584],
    "83": [0, 0.69444, 0, 0, 0.55556],
    "84": [0, 0.69444, 0, 0, 0.68056],
    "85": [0, 0.69444, 0, 0, 0.6875],
    "86": [0, 0.69444, 0.01389, 0, 0.66667],
    "87": [0, 0.69444, 0.01389, 0, 0.94445],
    "88": [0, 0.69444, 0, 0, 0.66667],
    "89": [0, 0.69444, 0.025, 0, 0.66667],
    "90": [0, 0.69444, 0, 0, 0.61111],
    "91": [0.25, 0.75, 0, 0, 0.28889],
    "93": [0.25, 0.75, 0, 0, 0.28889],
    "94": [0, 0.69444, 0, 0, 0.5],
    "95": [0.35, 0.09444, 0.02778, 0, 0.5],
    "97": [0, 0.44444, 0, 0, 0.48056],
    "98": [0, 0.69444, 0, 0, 0.51667],
    "99": [0, 0.44444, 0, 0, 0.44445],
    "100": [0, 0.69444, 0, 0, 0.51667],
    "101": [0, 0.44444, 0, 0, 0.44445],
    "102": [0, 0.69444, 0.06944, 0, 0.30556],
    "103": [0.19444, 0.44444, 0.01389, 0, 0.5],
    "104": [0, 0.69444, 0, 0, 0.51667],
    "105": [0, 0.67937, 0, 0, 0.23889],
    "106": [0.19444, 0.67937, 0, 0, 0.26667],
    "107": [0, 0.69444, 0, 0, 0.48889],
    "108": [0, 0.69444, 0, 0, 0.23889],
    "109": [0, 0.44444, 0, 0, 0.79445],
    "110": [0, 0.44444, 0, 0, 0.51667],
    "111": [0, 0.44444, 0, 0, 0.5],
    "112": [0.19444, 0.44444, 0, 0, 0.51667],
    "113": [0.19444, 0.44444, 0, 0, 0.51667],
    "114": [0, 0.44444, 0.01389, 0, 0.34167],
    "115": [0, 0.44444, 0, 0, 0.38333],
    "116": [0, 0.57143, 0, 0, 0.36111],
    "117": [0, 0.44444, 0, 0, 0.51667],
    "118": [0, 0.44444, 0.01389, 0, 0.46111],
    "119": [0, 0.44444, 0.01389, 0, 0.68334],
    "120": [0, 0.44444, 0, 0, 0.46111],
    "121": [0.19444, 0.44444, 0.01389, 0, 0.46111],
    "122": [0, 0.44444, 0, 0, 0.43472],
    "126": [0.35, 0.32659, 0, 0, 0.5],
    "168": [0, 0.67937, 0, 0, 0.5],
    "176": [0, 0.69444, 0, 0, 0.66667],
    "184": [0.17014, 0, 0, 0, 0.44445],
    "305": [0, 0.44444, 0, 0, 0.23889],
    "567": [0.19444, 0.44444, 0, 0, 0.26667],
    "710": [0, 0.69444, 0, 0, 0.5],
    "711": [0, 0.63194, 0, 0, 0.5],
    "713": [0, 0.60889, 0, 0, 0.5],
    "714": [0, 0.69444, 0, 0, 0.5],
    "715": [0, 0.69444, 0, 0, 0.5],
    "728": [0, 0.69444, 0, 0, 0.5],
    "729": [0, 0.67937, 0, 0, 0.27778],
    "730": [0, 0.69444, 0, 0, 0.66667],
    "732": [0, 0.67659, 0, 0, 0.5],
    "733": [0, 0.69444, 0, 0, 0.5],
    "915": [0, 0.69444, 0, 0, 0.54167],
    "916": [0, 0.69444, 0, 0, 0.83334],
    "920": [0, 0.69444, 0, 0, 0.77778],
    "923": [0, 0.69444, 0, 0, 0.61111],
    "926": [0, 0.69444, 0, 0, 0.66667],
    "928": [0, 0.69444, 0, 0, 0.70834],
    "931": [0, 0.69444, 0, 0, 0.72222],
    "933": [0, 0.69444, 0, 0, 0.77778],
    "934": [0, 0.69444, 0, 0, 0.72222],
    "936": [0, 0.69444, 0, 0, 0.77778],
    "937": [0, 0.69444, 0, 0, 0.72222],
    "8211": [0, 0.44444, 0.02778, 0, 0.5],
    "8212": [0, 0.44444, 0.02778, 0, 1.0],
    "8216": [0, 0.69444, 0, 0, 0.27778],
    "8217": [0, 0.69444, 0, 0, 0.27778],
    "8220": [0, 0.69444, 0, 0, 0.5],
    "8221": [0, 0.69444, 0, 0, 0.5]
  },
  "Script-Regular": {
    "65": [0, 0.7, 0.22925, 0, 0.80253],
    "66": [0, 0.7, 0.04087, 0, 0.90757],
    "67": [0, 0.7, 0.1689, 0, 0.66619],
    "68": [0, 0.7, 0.09371, 0, 0.77443],
    "69": [0, 0.7, 0.18583, 0, 0.56162],
    "70": [0, 0.7, 0.13634, 0, 0.89544],
    "71": [0, 0.7, 0.17322, 0, 0.60961],
    "72": [0, 0.7, 0.29694, 0, 0.96919],
    "73": [0, 0.7, 0.19189, 0, 0.80907],
    "74": [0.27778, 0.7, 0.19189, 0, 1.05159],
    "75": [0, 0.7, 0.31259, 0, 0.91364],
    "76": [0, 0.7, 0.19189, 0, 0.87373],
    "77": [0, 0.7, 0.15981, 0, 1.08031],
    "78": [0, 0.7, 0.3525, 0, 0.9015],
    "79": [0, 0.7, 0.08078, 0, 0.73787],
    "80": [0, 0.7, 0.08078, 0, 1.01262],
    "81": [0, 0.7, 0.03305, 0, 0.88282],
    "82": [0, 0.7, 0.06259, 0, 0.85],
    "83": [0, 0.7, 0.19189, 0, 0.86767],
    "84": [0, 0.7, 0.29087, 0, 0.74697],
    "85": [0, 0.7, 0.25815, 0, 0.79996],
    "86": [0, 0.7, 0.27523, 0, 0.62204],
    "87": [0, 0.7, 0.27523, 0, 0.80532],
    "88": [0, 0.7, 0.26006, 0, 0.94445],
    "89": [0, 0.7, 0.2939, 0, 0.70961],
    "90": [0, 0.7, 0.24037, 0, 0.8212]
  },
  "Size1-Regular": {
    "40": [0.35001, 0.85, 0, 0, 0.45834],
    "41": [0.35001, 0.85, 0, 0, 0.45834],
    "47": [0.35001, 0.85, 0, 0, 0.57778],
    "91": [0.35001, 0.85, 0, 0, 0.41667],
    "92": [0.35001, 0.85, 0, 0, 0.57778],
    "93": [0.35001, 0.85, 0, 0, 0.41667],
    "123": [0.35001, 0.85, 0, 0, 0.58334],
    "125": [0.35001, 0.85, 0, 0, 0.58334],
    "710": [0, 0.72222, 0, 0, 0.55556],
    "732": [0, 0.72222, 0, 0, 0.55556],
    "770": [0, 0.72222, 0, 0, 0.55556],
    "771": [0, 0.72222, 0, 0, 0.55556],
    "8214": [-0.00099, 0.601, 0, 0, 0.77778],
    "8593": [1e-05, 0.6, 0, 0, 0.66667],
    "8595": [1e-05, 0.6, 0, 0, 0.66667],
    "8657": [1e-05, 0.6, 0, 0, 0.77778],
    "8659": [1e-05, 0.6, 0, 0, 0.77778],
    "8719": [0.25001, 0.75, 0, 0, 0.94445],
    "8720": [0.25001, 0.75, 0, 0, 0.94445],
    "8721": [0.25001, 0.75, 0, 0, 1.05556],
    "8730": [0.35001, 0.85, 0, 0, 1.0],
    "8739": [-0.00599, 0.606, 0, 0, 0.33333],
    "8741": [-0.00599, 0.606, 0, 0, 0.55556],
    "8747": [0.30612, 0.805, 0.19445, 0, 0.47222],
    "8748": [0.306, 0.805, 0.19445, 0, 0.47222],
    "8749": [0.306, 0.805, 0.19445, 0, 0.47222],
    "8750": [0.30612, 0.805, 0.19445, 0, 0.47222],
    "8896": [0.25001, 0.75, 0, 0, 0.83334],
    "8897": [0.25001, 0.75, 0, 0, 0.83334],
    "8898": [0.25001, 0.75, 0, 0, 0.83334],
    "8899": [0.25001, 0.75, 0, 0, 0.83334],
    "8968": [0.35001, 0.85, 0, 0, 0.47222],
    "8969": [0.35001, 0.85, 0, 0, 0.47222],
    "8970": [0.35001, 0.85, 0, 0, 0.47222],
    "8971": [0.35001, 0.85, 0, 0, 0.47222],
    "9168": [-0.00099, 0.601, 0, 0, 0.66667],
    "10216": [0.35001, 0.85, 0, 0, 0.47222],
    "10217": [0.35001, 0.85, 0, 0, 0.47222],
    "10752": [0.25001, 0.75, 0, 0, 1.11111],
    "10753": [0.25001, 0.75, 0, 0, 1.11111],
    "10754": [0.25001, 0.75, 0, 0, 1.11111],
    "10756": [0.25001, 0.75, 0, 0, 0.83334],
    "10758": [0.25001, 0.75, 0, 0, 0.83334]
  },
  "Size2-Regular": {
    "40": [0.65002, 1.15, 0, 0, 0.59722],
    "41": [0.65002, 1.15, 0, 0, 0.59722],
    "47": [0.65002, 1.15, 0, 0, 0.81111],
    "91": [0.65002, 1.15, 0, 0, 0.47222],
    "92": [0.65002, 1.15, 0, 0, 0.81111],
    "93": [0.65002, 1.15, 0, 0, 0.47222],
    "123": [0.65002, 1.15, 0, 0, 0.66667],
    "125": [0.65002, 1.15, 0, 0, 0.66667],
    "710": [0, 0.75, 0, 0, 1.0],
    "732": [0, 0.75, 0, 0, 1.0],
    "770": [0, 0.75, 0, 0, 1.0],
    "771": [0, 0.75, 0, 0, 1.0],
    "8719": [0.55001, 1.05, 0, 0, 1.27778],
    "8720": [0.55001, 1.05, 0, 0, 1.27778],
    "8721": [0.55001, 1.05, 0, 0, 1.44445],
    "8730": [0.65002, 1.15, 0, 0, 1.0],
    "8747": [0.86225, 1.36, 0.44445, 0, 0.55556],
    "8748": [0.862, 1.36, 0.44445, 0, 0.55556],
    "8749": [0.862, 1.36, 0.44445, 0, 0.55556],
    "8750": [0.86225, 1.36, 0.44445, 0, 0.55556],
    "8896": [0.55001, 1.05, 0, 0, 1.11111],
    "8897": [0.55001, 1.05, 0, 0, 1.11111],
    "8898": [0.55001, 1.05, 0, 0, 1.11111],
    "8899": [0.55001, 1.05, 0, 0, 1.11111],
    "8968": [0.65002, 1.15, 0, 0, 0.52778],
    "8969": [0.65002, 1.15, 0, 0, 0.52778],
    "8970": [0.65002, 1.15, 0, 0, 0.52778],
    "8971": [0.65002, 1.15, 0, 0, 0.52778],
    "10216": [0.65002, 1.15, 0, 0, 0.61111],
    "10217": [0.65002, 1.15, 0, 0, 0.61111],
    "10752": [0.55001, 1.05, 0, 0, 1.51112],
    "10753": [0.55001, 1.05, 0, 0, 1.51112],
    "10754": [0.55001, 1.05, 0, 0, 1.51112],
    "10756": [0.55001, 1.05, 0, 0, 1.11111],
    "10758": [0.55001, 1.05, 0, 0, 1.11111]
  },
  "Size3-Regular": {
    "40": [0.95003, 1.45, 0, 0, 0.73611],
    "41": [0.95003, 1.45, 0, 0, 0.73611],
    "47": [0.95003, 1.45, 0, 0, 1.04445],
    "91": [0.95003, 1.45, 0, 0, 0.52778],
    "92": [0.95003, 1.45, 0, 0, 1.04445],
    "93": [0.95003, 1.45, 0, 0, 0.52778],
    "123": [0.95003, 1.45, 0, 0, 0.75],
    "125": [0.95003, 1.45, 0, 0, 0.75],
    "710": [0, 0.75, 0, 0, 1.44445],
    "732": [0, 0.75, 0, 0, 1.44445],
    "770": [0, 0.75, 0, 0, 1.44445],
    "771": [0, 0.75, 0, 0, 1.44445],
    "8730": [0.95003, 1.45, 0, 0, 1.0],
    "8968": [0.95003, 1.45, 0, 0, 0.58334],
    "8969": [0.95003, 1.45, 0, 0, 0.58334],
    "8970": [0.95003, 1.45, 0, 0, 0.58334],
    "8971": [0.95003, 1.45, 0, 0, 0.58334],
    "10216": [0.95003, 1.45, 0, 0, 0.75],
    "10217": [0.95003, 1.45, 0, 0, 0.75]
  },
  "Size4-Regular": {
    "40": [1.25003, 1.75, 0, 0, 0.79167],
    "41": [1.25003, 1.75, 0, 0, 0.79167],
    "47": [1.25003, 1.75, 0, 0, 1.27778],
    "91": [1.25003, 1.75, 0, 0, 0.58334],
    "92": [1.25003, 1.75, 0, 0, 1.27778],
    "93": [1.25003, 1.75, 0, 0, 0.58334],
    "123": [1.25003, 1.75, 0, 0, 0.80556],
    "125": [1.25003, 1.75, 0, 0, 0.80556],
    "710": [0, 0.825, 0, 0, 1.8889],
    "732": [0, 0.825, 0, 0, 1.8889],
    "770": [0, 0.825, 0, 0, 1.8889],
    "771": [0, 0.825, 0, 0, 1.8889],
    "8730": [1.25003, 1.75, 0, 0, 1.0],
    "8968": [1.25003, 1.75, 0, 0, 0.63889],
    "8969": [1.25003, 1.75, 0, 0, 0.63889],
    "8970": [1.25003, 1.75, 0, 0, 0.63889],
    "8971": [1.25003, 1.75, 0, 0, 0.63889],
    "9115": [0.64502, 1.155, 0, 0, 0.875],
    "9116": [1e-05, 0.6, 0, 0, 0.875],
    "9117": [0.64502, 1.155, 0, 0, 0.875],
    "9118": [0.64502, 1.155, 0, 0, 0.875],
    "9119": [1e-05, 0.6, 0, 0, 0.875],
    "9120": [0.64502, 1.155, 0, 0, 0.875],
    "9121": [0.64502, 1.155, 0, 0, 0.66667],
    "9122": [-0.00099, 0.601, 0, 0, 0.66667],
    "9123": [0.64502, 1.155, 0, 0, 0.66667],
    "9124": [0.64502, 1.155, 0, 0, 0.66667],
    "9125": [-0.00099, 0.601, 0, 0, 0.66667],
    "9126": [0.64502, 1.155, 0, 0, 0.66667],
    "9127": [1e-05, 0.9, 0, 0, 0.88889],
    "9128": [0.65002, 1.15, 0, 0, 0.88889],
    "9129": [0.90001, 0, 0, 0, 0.88889],
    "9130": [0, 0.3, 0, 0, 0.88889],
    "9131": [1e-05, 0.9, 0, 0, 0.88889],
    "9132": [0.65002, 1.15, 0, 0, 0.88889],
    "9133": [0.90001, 0, 0, 0, 0.88889],
    "9143": [0.88502, 0.915, 0, 0, 1.05556],
    "10216": [1.25003, 1.75, 0, 0, 0.80556],
    "10217": [1.25003, 1.75, 0, 0, 0.80556],
    "57344": [-0.00499, 0.605, 0, 0, 1.05556],
    "57345": [-0.00499, 0.605, 0, 0, 1.05556],
    "57680": [0, 0.12, 0, 0, 0.45],
    "57681": [0, 0.12, 0, 0, 0.45],
    "57682": [0, 0.12, 0, 0, 0.45],
    "57683": [0, 0.12, 0, 0, 0.45]
  },
  "Typewriter-Regular": {
    "32": [0, 0, 0, 0, 0.525],
    "33": [0, 0.61111, 0, 0, 0.525],
    "34": [0, 0.61111, 0, 0, 0.525],
    "35": [0, 0.61111, 0, 0, 0.525],
    "36": [0.08333, 0.69444, 0, 0, 0.525],
    "37": [0.08333, 0.69444, 0, 0, 0.525],
    "38": [0, 0.61111, 0, 0, 0.525],
    "39": [0, 0.61111, 0, 0, 0.525],
    "40": [0.08333, 0.69444, 0, 0, 0.525],
    "41": [0.08333, 0.69444, 0, 0, 0.525],
    "42": [0, 0.52083, 0, 0, 0.525],
    "43": [-0.08056, 0.53055, 0, 0, 0.525],
    "44": [0.13889, 0.125, 0, 0, 0.525],
    "45": [-0.08056, 0.53055, 0, 0, 0.525],
    "46": [0, 0.125, 0, 0, 0.525],
    "47": [0.08333, 0.69444, 0, 0, 0.525],
    "48": [0, 0.61111, 0, 0, 0.525],
    "49": [0, 0.61111, 0, 0, 0.525],
    "50": [0, 0.61111, 0, 0, 0.525],
    "51": [0, 0.61111, 0, 0, 0.525],
    "52": [0, 0.61111, 0, 0, 0.525],
    "53": [0, 0.61111, 0, 0, 0.525],
    "54": [0, 0.61111, 0, 0, 0.525],
    "55": [0, 0.61111, 0, 0, 0.525],
    "56": [0, 0.61111, 0, 0, 0.525],
    "57": [0, 0.61111, 0, 0, 0.525],
    "58": [0, 0.43056, 0, 0, 0.525],
    "59": [0.13889, 0.43056, 0, 0, 0.525],
    "60": [-0.05556, 0.55556, 0, 0, 0.525],
    "61": [-0.19549, 0.41562, 0, 0, 0.525],
    "62": [-0.05556, 0.55556, 0, 0, 0.525],
    "63": [0, 0.61111, 0, 0, 0.525],
    "64": [0, 0.61111, 0, 0, 0.525],
    "65": [0, 0.61111, 0, 0, 0.525],
    "66": [0, 0.61111, 0, 0, 0.525],
    "67": [0, 0.61111, 0, 0, 0.525],
    "68": [0, 0.61111, 0, 0, 0.525],
    "69": [0, 0.61111, 0, 0, 0.525],
    "70": [0, 0.61111, 0, 0, 0.525],
    "71": [0, 0.61111, 0, 0, 0.525],
    "72": [0, 0.61111, 0, 0, 0.525],
    "73": [0, 0.61111, 0, 0, 0.525],
    "74": [0, 0.61111, 0, 0, 0.525],
    "75": [0, 0.61111, 0, 0, 0.525],
    "76": [0, 0.61111, 0, 0, 0.525],
    "77": [0, 0.61111, 0, 0, 0.525],
    "78": [0, 0.61111, 0, 0, 0.525],
    "79": [0, 0.61111, 0, 0, 0.525],
    "80": [0, 0.61111, 0, 0, 0.525],
    "81": [0.13889, 0.61111, 0, 0, 0.525],
    "82": [0, 0.61111, 0, 0, 0.525],
    "83": [0, 0.61111, 0, 0, 0.525],
    "84": [0, 0.61111, 0, 0, 0.525],
    "85": [0, 0.61111, 0, 0, 0.525],
    "86": [0, 0.61111, 0, 0, 0.525],
    "87": [0, 0.61111, 0, 0, 0.525],
    "88": [0, 0.61111, 0, 0, 0.525],
    "89": [0, 0.61111, 0, 0, 0.525],
    "90": [0, 0.61111, 0, 0, 0.525],
    "91": [0.08333, 0.69444, 0, 0, 0.525],
    "92": [0.08333, 0.69444, 0, 0, 0.525],
    "93": [0.08333, 0.69444, 0, 0, 0.525],
    "94": [0, 0.61111, 0, 0, 0.525],
    "95": [0.09514, 0, 0, 0, 0.525],
    "96": [0, 0.61111, 0, 0, 0.525],
    "97": [0, 0.43056, 0, 0, 0.525],
    "98": [0, 0.61111, 0, 0, 0.525],
    "99": [0, 0.43056, 0, 0, 0.525],
    "100": [0, 0.61111, 0, 0, 0.525],
    "101": [0, 0.43056, 0, 0, 0.525],
    "102": [0, 0.61111, 0, 0, 0.525],
    "103": [0.22222, 0.43056, 0, 0, 0.525],
    "104": [0, 0.61111, 0, 0, 0.525],
    "105": [0, 0.61111, 0, 0, 0.525],
    "106": [0.22222, 0.61111, 0, 0, 0.525],
    "107": [0, 0.61111, 0, 0, 0.525],
    "108": [0, 0.61111, 0, 0, 0.525],
    "109": [0, 0.43056, 0, 0, 0.525],
    "110": [0, 0.43056, 0, 0, 0.525],
    "111": [0, 0.43056, 0, 0, 0.525],
    "112": [0.22222, 0.43056, 0, 0, 0.525],
    "113": [0.22222, 0.43056, 0, 0, 0.525],
    "114": [0, 0.43056, 0, 0, 0.525],
    "115": [0, 0.43056, 0, 0, 0.525],
    "116": [0, 0.55358, 0, 0, 0.525],
    "117": [0, 0.43056, 0, 0, 0.525],
    "118": [0, 0.43056, 0, 0, 0.525],
    "119": [0, 0.43056, 0, 0, 0.525],
    "120": [0, 0.43056, 0, 0, 0.525],
    "121": [0.22222, 0.43056, 0, 0, 0.525],
    "122": [0, 0.43056, 0, 0, 0.525],
    "123": [0.08333, 0.69444, 0, 0, 0.525],
    "124": [0.08333, 0.69444, 0, 0, 0.525],
    "125": [0.08333, 0.69444, 0, 0, 0.525],
    "126": [0, 0.61111, 0, 0, 0.525],
    "127": [0, 0.61111, 0, 0, 0.525],
    "160": [0, 0, 0, 0, 0.525],
    "176": [0, 0.61111, 0, 0, 0.525],
    "184": [0.19445, 0, 0, 0, 0.525],
    "305": [0, 0.43056, 0, 0, 0.525],
    "567": [0.22222, 0.43056, 0, 0, 0.525],
    "711": [0, 0.56597, 0, 0, 0.525],
    "713": [0, 0.56555, 0, 0, 0.525],
    "714": [0, 0.61111, 0, 0, 0.525],
    "715": [0, 0.61111, 0, 0, 0.525],
    "728": [0, 0.61111, 0, 0, 0.525],
    "730": [0, 0.61111, 0, 0, 0.525],
    "770": [0, 0.61111, 0, 0, 0.525],
    "771": [0, 0.61111, 0, 0, 0.525],
    "776": [0, 0.61111, 0, 0, 0.525],
    "915": [0, 0.61111, 0, 0, 0.525],
    "916": [0, 0.61111, 0, 0, 0.525],
    "920": [0, 0.61111, 0, 0, 0.525],
    "923": [0, 0.61111, 0, 0, 0.525],
    "926": [0, 0.61111, 0, 0, 0.525],
    "928": [0, 0.61111, 0, 0, 0.525],
    "931": [0, 0.61111, 0, 0, 0.525],
    "933": [0, 0.61111, 0, 0, 0.525],
    "934": [0, 0.61111, 0, 0, 0.525],
    "936": [0, 0.61111, 0, 0, 0.525],
    "937": [0, 0.61111, 0, 0, 0.525],
    "8216": [0, 0.61111, 0, 0, 0.525],
    "8217": [0, 0.61111, 0, 0, 0.525],
    "8242": [0, 0.61111, 0, 0, 0.525],
    "9251": [0.11111, 0.21944, 0, 0, 0.525]
  }
});
// CONCATENATED MODULE: ./src/fontMetrics.js


/**
 * This file contains metrics regarding fonts and individual symbols. The sigma
 * and xi variables, as well as the metricMap map contain data extracted from
 * TeX, TeX font metrics, and the TTF files. These data are then exposed via the
 * `metrics` variable and the getCharacterMetrics function.
 */
// In TeX, there are actually three sets of dimensions, one for each of
// textstyle (size index 5 and higher: >=9pt), scriptstyle (size index 3 and 4:
// 7-8pt), and scriptscriptstyle (size index 1 and 2: 5-6pt).  These are
// provided in the the arrays below, in that order.
//
// The font metrics are stored in fonts cmsy10, cmsy7, and cmsy5 respsectively.
// This was determined by running the following script:
//
//     latex -interaction=nonstopmode \
//     '\documentclass{article}\usepackage{amsmath}\begin{document}' \
//     '$a$ \expandafter\show\the\textfont2' \
//     '\expandafter\show\the\scriptfont2' \
//     '\expandafter\show\the\scriptscriptfont2' \
//     '\stop'
//
// The metrics themselves were retreived using the following commands:
//
//     tftopl cmsy10
//     tftopl cmsy7
//     tftopl cmsy5
//
// The output of each of these commands is quite lengthy.  The only part we
// care about is the FONTDIMEN section. Each value is measured in EMs.
var sigmasAndXis = {
  slant: [0.250, 0.250, 0.250],
  // sigma1
  space: [0.000, 0.000, 0.000],
  // sigma2
  stretch: [0.000, 0.000, 0.000],
  // sigma3
  shrink: [0.000, 0.000, 0.000],
  // sigma4
  xHeight: [0.431, 0.431, 0.431],
  // sigma5
  quad: [1.000, 1.171, 1.472],
  // sigma6
  extraSpace: [0.000, 0.000, 0.000],
  // sigma7
  num1: [0.677, 0.732, 0.925],
  // sigma8
  num2: [0.394, 0.384, 0.387],
  // sigma9
  num3: [0.444, 0.471, 0.504],
  // sigma10
  denom1: [0.686, 0.752, 1.025],
  // sigma11
  denom2: [0.345, 0.344, 0.532],
  // sigma12
  sup1: [0.413, 0.503, 0.504],
  // sigma13
  sup2: [0.363, 0.431, 0.404],
  // sigma14
  sup3: [0.289, 0.286, 0.294],
  // sigma15
  sub1: [0.150, 0.143, 0.200],
  // sigma16
  sub2: [0.247, 0.286, 0.400],
  // sigma17
  supDrop: [0.386, 0.353, 0.494],
  // sigma18
  subDrop: [0.050, 0.071, 0.100],
  // sigma19
  delim1: [2.390, 1.700, 1.980],
  // sigma20
  delim2: [1.010, 1.157, 1.420],
  // sigma21
  axisHeight: [0.250, 0.250, 0.250],
  // sigma22
  // These font metrics are extracted from TeX by using tftopl on cmex10.tfm;
  // they correspond to the font parameters of the extension fonts (family 3).
  // See the TeXbook, page 441. In AMSTeX, the extension fonts scale; to
  // match cmex7, we'd use cmex7.tfm values for script and scriptscript
  // values.
  defaultRuleThickness: [0.04, 0.049, 0.049],
  // xi8; cmex7: 0.049
  bigOpSpacing1: [0.111, 0.111, 0.111],
  // xi9
  bigOpSpacing2: [0.166, 0.166, 0.166],
  // xi10
  bigOpSpacing3: [0.2, 0.2, 0.2],
  // xi11
  bigOpSpacing4: [0.6, 0.611, 0.611],
  // xi12; cmex7: 0.611
  bigOpSpacing5: [0.1, 0.143, 0.143],
  // xi13; cmex7: 0.143
  // The \sqrt rule width is taken from the height of the surd character.
  // Since we use the same font at all sizes, this thickness doesn't scale.
  sqrtRuleThickness: [0.04, 0.04, 0.04],
  // This value determines how large a pt is, for metrics which are defined
  // in terms of pts.
  // This value is also used in katex.less; if you change it make sure the
  // values match.
  ptPerEm: [10.0, 10.0, 10.0],
  // The space between adjacent `|` columns in an array definition. From
  // `\showthe\doublerulesep` in LaTeX. Equals 2.0 / ptPerEm.
  doubleRuleSep: [0.2, 0.2, 0.2]
}; // This map contains a mapping from font name and character code to character
// metrics, including height, depth, italic correction, and skew (kern from the
// character to the corresponding \skewchar)
// This map is generated via `make metrics`. It should not be changed manually.

 // These are very rough approximations.  We default to Times New Roman which
// should have Latin-1 and Cyrillic characters, but may not depending on the
// operating system.  The metrics do not account for extra height from the
// accents.  In the case of Cyrillic characters which have both ascenders and
// descenders we prefer approximations with ascenders, primarily to prevent
// the fraction bar or root line from intersecting the glyph.
// TODO(kevinb) allow union of multiple glyph metrics for better accuracy.

var extraCharacterMap = {
  // Latin-1
  'Å': 'A',
  'Ç': 'C',
  'Ð': 'D',
  'Þ': 'o',
  'å': 'a',
  'ç': 'c',
  'ð': 'd',
  'þ': 'o',
  // Cyrillic
  'А': 'A',
  'Б': 'B',
  'В': 'B',
  'Г': 'F',
  'Д': 'A',
  'Е': 'E',
  'Ж': 'K',
  'З': '3',
  'И': 'N',
  'Й': 'N',
  'К': 'K',
  'Л': 'N',
  'М': 'M',
  'Н': 'H',
  'О': 'O',
  'П': 'N',
  'Р': 'P',
  'С': 'C',
  'Т': 'T',
  'У': 'y',
  'Ф': 'O',
  'Х': 'X',
  'Ц': 'U',
  'Ч': 'h',
  'Ш': 'W',
  'Щ': 'W',
  'Ъ': 'B',
  'Ы': 'X',
  'Ь': 'B',
  'Э': '3',
  'Ю': 'X',
  'Я': 'R',
  'а': 'a',
  'б': 'b',
  'в': 'a',
  'г': 'r',
  'д': 'y',
  'е': 'e',
  'ж': 'm',
  'з': 'e',
  'и': 'n',
  'й': 'n',
  'к': 'n',
  'л': 'n',
  'м': 'm',
  'н': 'n',
  'о': 'o',
  'п': 'n',
  'р': 'p',
  'с': 'c',
  'т': 'o',
  'у': 'y',
  'ф': 'b',
  'х': 'x',
  'ц': 'n',
  'ч': 'n',
  'ш': 'w',
  'щ': 'w',
  'ъ': 'a',
  'ы': 'm',
  'ь': 'a',
  'э': 'e',
  'ю': 'm',
  'я': 'r'
};

/**
 * This function adds new font metrics to default metricMap
 * It can also override existing metrics
 */
function setFontMetrics(fontName, metrics) {
  fontMetricsData[fontName] = metrics;
}
/**
 * This function is a convenience function for looking up information in the
 * metricMap table. It takes a character as a string, and a font.
 *
 * Note: the `width` property may be undefined if fontMetricsData.js wasn't
 * built using `Make extended_metrics`.
 */

function getCharacterMetrics(character, font, mode) {
  if (!fontMetricsData[font]) {
    throw new Error("Font metrics not found for font: " + font + ".");
  }

  var ch = character.charCodeAt(0);
  var metrics = fontMetricsData[font][ch];

  if (!metrics && character[0] in extraCharacterMap) {
    ch = extraCharacterMap[character[0]].charCodeAt(0);
    metrics = fontMetricsData[font][ch];
  }

  if (!metrics && mode === 'text') {
    // We don't typically have font metrics for Asian scripts.
    // But since we support them in text mode, we need to return
    // some sort of metrics.
    // So if the character is in a script we support but we
    // don't have metrics for it, just use the metrics for
    // the Latin capital letter M. This is close enough because
    // we (currently) only care about the height of the glpyh
    // not its width.
    if (supportedCodepoint(ch)) {
      metrics = fontMetricsData[font][77]; // 77 is the charcode for 'M'
    }
  }

  if (metrics) {
    return {
      depth: metrics[0],
      height: metrics[1],
      italic: metrics[2],
      skew: metrics[3],
      width: metrics[4]
    };
  }
}
var fontMetricsBySizeIndex = {};
/**
 * Get the font metrics for a given size.
 */

function getGlobalMetrics(size) {
  var sizeIndex;

  if (size >= 5) {
    sizeIndex = 0;
  } else if (size >= 3) {
    sizeIndex = 1;
  } else {
    sizeIndex = 2;
  }

  if (!fontMetricsBySizeIndex[sizeIndex]) {
    var metrics = fontMetricsBySizeIndex[sizeIndex] = {
      cssEmPerMu: sigmasAndXis.quad[sizeIndex] / 18
    };

    for (var key in sigmasAndXis) {
      if (sigmasAndXis.hasOwnProperty(key)) {
        metrics[key] = sigmasAndXis[key][sizeIndex];
      }
    }
  }

  return fontMetricsBySizeIndex[sizeIndex];
}
// CONCATENATED MODULE: ./src/symbols.js
/**
 * This file holds a list of all no-argument functions and single-character
 * symbols (like 'a' or ';').
 *
 * For each of the symbols, there are three properties they can have:
 * - font (required): the font to be used for this symbol. Either "main" (the
     normal font), or "ams" (the ams fonts).
 * - group (required): the ParseNode group type the symbol should have (i.e.
     "textord", "mathord", etc).
     See https://github.com/KaTeX/KaTeX/wiki/Examining-TeX#group-types
 * - replace: the character that this symbol or function should be
 *   replaced with (i.e. "\phi" has a replace value of "\u03d5", the phi
 *   character in the main font).
 *
 * The outermost map in the table indicates what mode the symbols should be
 * accepted in (e.g. "math" or "text").
 */
// Some of these have a "-token" suffix since these are also used as `ParseNode`
// types for raw text tokens, and we want to avoid conflicts with higher-level
// `ParseNode` types. These `ParseNode`s are constructed within `Parser` by
// looking up the `symbols` map.
var ATOMS = {
  "bin": 1,
  "close": 1,
  "inner": 1,
  "open": 1,
  "punct": 1,
  "rel": 1
};
var NON_ATOMS = {
  "accent-token": 1,
  "mathord": 1,
  "op-token": 1,
  "spacing": 1,
  "textord": 1
};
var symbols = {
  "math": {},
  "text": {}
};
/* harmony default export */ var src_symbols = (symbols);
/** `acceptUnicodeChar = true` is only applicable if `replace` is set. */

function defineSymbol(mode, font, group, replace, name, acceptUnicodeChar) {
  symbols[mode][name] = {
    font: font,
    group: group,
    replace: replace
  };

  if (acceptUnicodeChar && replace) {
    symbols[mode][replace] = symbols[mode][name];
  }
} // Some abbreviations for commonly used strings.
// This helps minify the code, and also spotting typos using jshint.
// modes:

var symbols_math = "math";
var symbols_text = "text"; // fonts:

var main = "main";
var ams = "ams"; // groups:

var symbols_accent = "accent-token";
var bin = "bin";
var symbols_close = "close";
var symbols_inner = "inner";
var mathord = "mathord";
var op = "op-token";
var symbols_open = "open";
var punct = "punct";
var rel = "rel";
var symbols_spacing = "spacing";
var symbols_textord = "textord"; // Now comes the symbol table
// Relation Symbols

defineSymbol(symbols_math, main, rel, "\u2261", "\\equiv", true);
defineSymbol(symbols_math, main, rel, "\u227A", "\\prec", true);
defineSymbol(symbols_math, main, rel, "\u227B", "\\succ", true);
defineSymbol(symbols_math, main, rel, "\u223C", "\\sim", true);
defineSymbol(symbols_math, main, rel, "\u22A5", "\\perp");
defineSymbol(symbols_math, main, rel, "\u2AAF", "\\preceq", true);
defineSymbol(symbols_math, main, rel, "\u2AB0", "\\succeq", true);
defineSymbol(symbols_math, main, rel, "\u2243", "\\simeq", true);
defineSymbol(symbols_math, main, rel, "\u2223", "\\mid", true);
defineSymbol(symbols_math, main, rel, "\u226A", "\\ll", true);
defineSymbol(symbols_math, main, rel, "\u226B", "\\gg", true);
defineSymbol(symbols_math, main, rel, "\u224D", "\\asymp", true);
defineSymbol(symbols_math, main, rel, "\u2225", "\\parallel");
defineSymbol(symbols_math, main, rel, "\u22C8", "\\bowtie", true);
defineSymbol(symbols_math, main, rel, "\u2323", "\\smile", true);
defineSymbol(symbols_math, main, rel, "\u2291", "\\sqsubseteq", true);
defineSymbol(symbols_math, main, rel, "\u2292", "\\sqsupseteq", true);
defineSymbol(symbols_math, main, rel, "\u2250", "\\doteq", true);
defineSymbol(symbols_math, main, rel, "\u2322", "\\frown", true);
defineSymbol(symbols_math, main, rel, "\u220B", "\\ni", true);
defineSymbol(symbols_math, main, rel, "\u221D", "\\propto", true);
defineSymbol(symbols_math, main, rel, "\u22A2", "\\vdash", true);
defineSymbol(symbols_math, main, rel, "\u22A3", "\\dashv", true);
defineSymbol(symbols_math, main, rel, "\u220B", "\\owns"); // Punctuation

defineSymbol(symbols_math, main, punct, ".", "\\ldotp");
defineSymbol(symbols_math, main, punct, "\u22C5", "\\cdotp"); // Misc Symbols

defineSymbol(symbols_math, main, symbols_textord, "#", "\\#");
defineSymbol(symbols_text, main, symbols_textord, "#", "\\#");
defineSymbol(symbols_math, main, symbols_textord, "&", "\\&");
defineSymbol(symbols_text, main, symbols_textord, "&", "\\&");
defineSymbol(symbols_math, main, symbols_textord, "\u2135", "\\aleph", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2200", "\\forall", true);
defineSymbol(symbols_math, main, symbols_textord, "\u210F", "\\hbar", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2203", "\\exists", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2207", "\\nabla", true);
defineSymbol(symbols_math, main, symbols_textord, "\u266D", "\\flat", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2113", "\\ell", true);
defineSymbol(symbols_math, main, symbols_textord, "\u266E", "\\natural", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2663", "\\clubsuit", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2118", "\\wp", true);
defineSymbol(symbols_math, main, symbols_textord, "\u266F", "\\sharp", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2662", "\\diamondsuit", true);
defineSymbol(symbols_math, main, symbols_textord, "\u211C", "\\Re", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2661", "\\heartsuit", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2111", "\\Im", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2660", "\\spadesuit", true);
defineSymbol(symbols_text, main, symbols_textord, "\xA7", "\\S", true);
defineSymbol(symbols_text, main, symbols_textord, "\xB6", "\\P", true); // Math and Text

defineSymbol(symbols_math, main, symbols_textord, "\u2020", "\\dag");
defineSymbol(symbols_text, main, symbols_textord, "\u2020", "\\dag");
defineSymbol(symbols_text, main, symbols_textord, "\u2020", "\\textdagger");
defineSymbol(symbols_math, main, symbols_textord, "\u2021", "\\ddag");
defineSymbol(symbols_text, main, symbols_textord, "\u2021", "\\ddag");
defineSymbol(symbols_text, main, symbols_textord, "\u2021", "\\textdaggerdbl"); // Large Delimiters

defineSymbol(symbols_math, main, symbols_close, "\u23B1", "\\rmoustache", true);
defineSymbol(symbols_math, main, symbols_open, "\u23B0", "\\lmoustache", true);
defineSymbol(symbols_math, main, symbols_close, "\u27EF", "\\rgroup", true);
defineSymbol(symbols_math, main, symbols_open, "\u27EE", "\\lgroup", true); // Binary Operators

defineSymbol(symbols_math, main, bin, "\u2213", "\\mp", true);
defineSymbol(symbols_math, main, bin, "\u2296", "\\ominus", true);
defineSymbol(symbols_math, main, bin, "\u228E", "\\uplus", true);
defineSymbol(symbols_math, main, bin, "\u2293", "\\sqcap", true);
defineSymbol(symbols_math, main, bin, "\u2217", "\\ast");
defineSymbol(symbols_math, main, bin, "\u2294", "\\sqcup", true);
defineSymbol(symbols_math, main, bin, "\u25EF", "\\bigcirc");
defineSymbol(symbols_math, main, bin, "\u2219", "\\bullet");
defineSymbol(symbols_math, main, bin, "\u2021", "\\ddagger");
defineSymbol(symbols_math, main, bin, "\u2240", "\\wr", true);
defineSymbol(symbols_math, main, bin, "\u2A3F", "\\amalg");
defineSymbol(symbols_math, main, bin, "&", "\\And"); // from amsmath
// Arrow Symbols

defineSymbol(symbols_math, main, rel, "\u27F5", "\\longleftarrow", true);
defineSymbol(symbols_math, main, rel, "\u21D0", "\\Leftarrow", true);
defineSymbol(symbols_math, main, rel, "\u27F8", "\\Longleftarrow", true);
defineSymbol(symbols_math, main, rel, "\u27F6", "\\longrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u21D2", "\\Rightarrow", true);
defineSymbol(symbols_math, main, rel, "\u27F9", "\\Longrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u2194", "\\leftrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u27F7", "\\longleftrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u21D4", "\\Leftrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u27FA", "\\Longleftrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u21A6", "\\mapsto", true);
defineSymbol(symbols_math, main, rel, "\u27FC", "\\longmapsto", true);
defineSymbol(symbols_math, main, rel, "\u2197", "\\nearrow", true);
defineSymbol(symbols_math, main, rel, "\u21A9", "\\hookleftarrow", true);
defineSymbol(symbols_math, main, rel, "\u21AA", "\\hookrightarrow", true);
defineSymbol(symbols_math, main, rel, "\u2198", "\\searrow", true);
defineSymbol(symbols_math, main, rel, "\u21BC", "\\leftharpoonup", true);
defineSymbol(symbols_math, main, rel, "\u21C0", "\\rightharpoonup", true);
defineSymbol(symbols_math, main, rel, "\u2199", "\\swarrow", true);
defineSymbol(symbols_math, main, rel, "\u21BD", "\\leftharpoondown", true);
defineSymbol(symbols_math, main, rel, "\u21C1", "\\rightharpoondown", true);
defineSymbol(symbols_math, main, rel, "\u2196", "\\nwarrow", true);
defineSymbol(symbols_math, main, rel, "\u21CC", "\\rightleftharpoons", true); // AMS Negated Binary Relations

defineSymbol(symbols_math, ams, rel, "\u226E", "\\nless", true); // Symbol names preceeded by "@" each have a corresponding macro.

defineSymbol(symbols_math, ams, rel, "\uE010", "\\@nleqslant");
defineSymbol(symbols_math, ams, rel, "\uE011", "\\@nleqq");
defineSymbol(symbols_math, ams, rel, "\u2A87", "\\lneq", true);
defineSymbol(symbols_math, ams, rel, "\u2268", "\\lneqq", true);
defineSymbol(symbols_math, ams, rel, "\uE00C", "\\@lvertneqq");
defineSymbol(symbols_math, ams, rel, "\u22E6", "\\lnsim", true);
defineSymbol(symbols_math, ams, rel, "\u2A89", "\\lnapprox", true);
defineSymbol(symbols_math, ams, rel, "\u2280", "\\nprec", true); // unicode-math maps \u22e0 to \npreccurlyeq. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u22E0", "\\npreceq", true);
defineSymbol(symbols_math, ams, rel, "\u22E8", "\\precnsim", true);
defineSymbol(symbols_math, ams, rel, "\u2AB9", "\\precnapprox", true);
defineSymbol(symbols_math, ams, rel, "\u2241", "\\nsim", true);
defineSymbol(symbols_math, ams, rel, "\uE006", "\\@nshortmid");
defineSymbol(symbols_math, ams, rel, "\u2224", "\\nmid", true);
defineSymbol(symbols_math, ams, rel, "\u22AC", "\\nvdash", true);
defineSymbol(symbols_math, ams, rel, "\u22AD", "\\nvDash", true);
defineSymbol(symbols_math, ams, rel, "\u22EA", "\\ntriangleleft");
defineSymbol(symbols_math, ams, rel, "\u22EC", "\\ntrianglelefteq", true);
defineSymbol(symbols_math, ams, rel, "\u228A", "\\subsetneq", true);
defineSymbol(symbols_math, ams, rel, "\uE01A", "\\@varsubsetneq");
defineSymbol(symbols_math, ams, rel, "\u2ACB", "\\subsetneqq", true);
defineSymbol(symbols_math, ams, rel, "\uE017", "\\@varsubsetneqq");
defineSymbol(symbols_math, ams, rel, "\u226F", "\\ngtr", true);
defineSymbol(symbols_math, ams, rel, "\uE00F", "\\@ngeqslant");
defineSymbol(symbols_math, ams, rel, "\uE00E", "\\@ngeqq");
defineSymbol(symbols_math, ams, rel, "\u2A88", "\\gneq", true);
defineSymbol(symbols_math, ams, rel, "\u2269", "\\gneqq", true);
defineSymbol(symbols_math, ams, rel, "\uE00D", "\\@gvertneqq");
defineSymbol(symbols_math, ams, rel, "\u22E7", "\\gnsim", true);
defineSymbol(symbols_math, ams, rel, "\u2A8A", "\\gnapprox", true);
defineSymbol(symbols_math, ams, rel, "\u2281", "\\nsucc", true); // unicode-math maps \u22e1 to \nsucccurlyeq. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u22E1", "\\nsucceq", true);
defineSymbol(symbols_math, ams, rel, "\u22E9", "\\succnsim", true);
defineSymbol(symbols_math, ams, rel, "\u2ABA", "\\succnapprox", true); // unicode-math maps \u2246 to \simneqq. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u2246", "\\ncong", true);
defineSymbol(symbols_math, ams, rel, "\uE007", "\\@nshortparallel");
defineSymbol(symbols_math, ams, rel, "\u2226", "\\nparallel", true);
defineSymbol(symbols_math, ams, rel, "\u22AF", "\\nVDash", true);
defineSymbol(symbols_math, ams, rel, "\u22EB", "\\ntriangleright");
defineSymbol(symbols_math, ams, rel, "\u22ED", "\\ntrianglerighteq", true);
defineSymbol(symbols_math, ams, rel, "\uE018", "\\@nsupseteqq");
defineSymbol(symbols_math, ams, rel, "\u228B", "\\supsetneq", true);
defineSymbol(symbols_math, ams, rel, "\uE01B", "\\@varsupsetneq");
defineSymbol(symbols_math, ams, rel, "\u2ACC", "\\supsetneqq", true);
defineSymbol(symbols_math, ams, rel, "\uE019", "\\@varsupsetneqq");
defineSymbol(symbols_math, ams, rel, "\u22AE", "\\nVdash", true);
defineSymbol(symbols_math, ams, rel, "\u2AB5", "\\precneqq", true);
defineSymbol(symbols_math, ams, rel, "\u2AB6", "\\succneqq", true);
defineSymbol(symbols_math, ams, rel, "\uE016", "\\@nsubseteqq");
defineSymbol(symbols_math, ams, bin, "\u22B4", "\\unlhd");
defineSymbol(symbols_math, ams, bin, "\u22B5", "\\unrhd"); // AMS Negated Arrows

defineSymbol(symbols_math, ams, rel, "\u219A", "\\nleftarrow", true);
defineSymbol(symbols_math, ams, rel, "\u219B", "\\nrightarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21CD", "\\nLeftarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21CF", "\\nRightarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21AE", "\\nleftrightarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21CE", "\\nLeftrightarrow", true); // AMS Misc

defineSymbol(symbols_math, ams, rel, "\u25B3", "\\vartriangle");
defineSymbol(symbols_math, ams, symbols_textord, "\u210F", "\\hslash");
defineSymbol(symbols_math, ams, symbols_textord, "\u25BD", "\\triangledown");
defineSymbol(symbols_math, ams, symbols_textord, "\u25CA", "\\lozenge");
defineSymbol(symbols_math, ams, symbols_textord, "\u24C8", "\\circledS");
defineSymbol(symbols_math, ams, symbols_textord, "\xAE", "\\circledR");
defineSymbol(symbols_text, ams, symbols_textord, "\xAE", "\\circledR");
defineSymbol(symbols_math, ams, symbols_textord, "\u2221", "\\measuredangle", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2204", "\\nexists");
defineSymbol(symbols_math, ams, symbols_textord, "\u2127", "\\mho");
defineSymbol(symbols_math, ams, symbols_textord, "\u2132", "\\Finv", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2141", "\\Game", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2035", "\\backprime");
defineSymbol(symbols_math, ams, symbols_textord, "\u25B2", "\\blacktriangle");
defineSymbol(symbols_math, ams, symbols_textord, "\u25BC", "\\blacktriangledown");
defineSymbol(symbols_math, ams, symbols_textord, "\u25A0", "\\blacksquare");
defineSymbol(symbols_math, ams, symbols_textord, "\u29EB", "\\blacklozenge");
defineSymbol(symbols_math, ams, symbols_textord, "\u2605", "\\bigstar");
defineSymbol(symbols_math, ams, symbols_textord, "\u2222", "\\sphericalangle", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2201", "\\complement", true); // unicode-math maps U+F0 (ð) to \matheth. We map to AMS function \eth

defineSymbol(symbols_math, ams, symbols_textord, "\xF0", "\\eth", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2571", "\\diagup");
defineSymbol(symbols_math, ams, symbols_textord, "\u2572", "\\diagdown");
defineSymbol(symbols_math, ams, symbols_textord, "\u25A1", "\\square");
defineSymbol(symbols_math, ams, symbols_textord, "\u25A1", "\\Box");
defineSymbol(symbols_math, ams, symbols_textord, "\u25CA", "\\Diamond"); // unicode-math maps U+A5 to \mathyen. We map to AMS function \yen

defineSymbol(symbols_math, ams, symbols_textord, "\xA5", "\\yen", true);
defineSymbol(symbols_text, ams, symbols_textord, "\xA5", "\\yen", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2713", "\\checkmark", true);
defineSymbol(symbols_text, ams, symbols_textord, "\u2713", "\\checkmark"); // AMS Hebrew

defineSymbol(symbols_math, ams, symbols_textord, "\u2136", "\\beth", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2138", "\\daleth", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2137", "\\gimel", true); // AMS Greek

defineSymbol(symbols_math, ams, symbols_textord, "\u03DD", "\\digamma");
defineSymbol(symbols_math, ams, symbols_textord, "\u03F0", "\\varkappa"); // AMS Delimiters

defineSymbol(symbols_math, ams, symbols_open, "\u250C", "\\ulcorner", true);
defineSymbol(symbols_math, ams, symbols_close, "\u2510", "\\urcorner", true);
defineSymbol(symbols_math, ams, symbols_open, "\u2514", "\\llcorner", true);
defineSymbol(symbols_math, ams, symbols_close, "\u2518", "\\lrcorner", true); // AMS Binary Relations

defineSymbol(symbols_math, ams, rel, "\u2266", "\\leqq", true);
defineSymbol(symbols_math, ams, rel, "\u2A7D", "\\leqslant", true);
defineSymbol(symbols_math, ams, rel, "\u2A95", "\\eqslantless", true);
defineSymbol(symbols_math, ams, rel, "\u2272", "\\lesssim", true);
defineSymbol(symbols_math, ams, rel, "\u2A85", "\\lessapprox", true);
defineSymbol(symbols_math, ams, rel, "\u224A", "\\approxeq", true);
defineSymbol(symbols_math, ams, bin, "\u22D6", "\\lessdot");
defineSymbol(symbols_math, ams, rel, "\u22D8", "\\lll", true);
defineSymbol(symbols_math, ams, rel, "\u2276", "\\lessgtr", true);
defineSymbol(symbols_math, ams, rel, "\u22DA", "\\lesseqgtr", true);
defineSymbol(symbols_math, ams, rel, "\u2A8B", "\\lesseqqgtr", true);
defineSymbol(symbols_math, ams, rel, "\u2251", "\\doteqdot");
defineSymbol(symbols_math, ams, rel, "\u2253", "\\risingdotseq", true);
defineSymbol(symbols_math, ams, rel, "\u2252", "\\fallingdotseq", true);
defineSymbol(symbols_math, ams, rel, "\u223D", "\\backsim", true);
defineSymbol(symbols_math, ams, rel, "\u22CD", "\\backsimeq", true);
defineSymbol(symbols_math, ams, rel, "\u2AC5", "\\subseteqq", true);
defineSymbol(symbols_math, ams, rel, "\u22D0", "\\Subset", true);
defineSymbol(symbols_math, ams, rel, "\u228F", "\\sqsubset", true);
defineSymbol(symbols_math, ams, rel, "\u227C", "\\preccurlyeq", true);
defineSymbol(symbols_math, ams, rel, "\u22DE", "\\curlyeqprec", true);
defineSymbol(symbols_math, ams, rel, "\u227E", "\\precsim", true);
defineSymbol(symbols_math, ams, rel, "\u2AB7", "\\precapprox", true);
defineSymbol(symbols_math, ams, rel, "\u22B2", "\\vartriangleleft");
defineSymbol(symbols_math, ams, rel, "\u22B4", "\\trianglelefteq");
defineSymbol(symbols_math, ams, rel, "\u22A8", "\\vDash", true);
defineSymbol(symbols_math, ams, rel, "\u22AA", "\\Vvdash", true);
defineSymbol(symbols_math, ams, rel, "\u2323", "\\smallsmile");
defineSymbol(symbols_math, ams, rel, "\u2322", "\\smallfrown");
defineSymbol(symbols_math, ams, rel, "\u224F", "\\bumpeq", true);
defineSymbol(symbols_math, ams, rel, "\u224E", "\\Bumpeq", true);
defineSymbol(symbols_math, ams, rel, "\u2267", "\\geqq", true);
defineSymbol(symbols_math, ams, rel, "\u2A7E", "\\geqslant", true);
defineSymbol(symbols_math, ams, rel, "\u2A96", "\\eqslantgtr", true);
defineSymbol(symbols_math, ams, rel, "\u2273", "\\gtrsim", true);
defineSymbol(symbols_math, ams, rel, "\u2A86", "\\gtrapprox", true);
defineSymbol(symbols_math, ams, bin, "\u22D7", "\\gtrdot");
defineSymbol(symbols_math, ams, rel, "\u22D9", "\\ggg", true);
defineSymbol(symbols_math, ams, rel, "\u2277", "\\gtrless", true);
defineSymbol(symbols_math, ams, rel, "\u22DB", "\\gtreqless", true);
defineSymbol(symbols_math, ams, rel, "\u2A8C", "\\gtreqqless", true);
defineSymbol(symbols_math, ams, rel, "\u2256", "\\eqcirc", true);
defineSymbol(symbols_math, ams, rel, "\u2257", "\\circeq", true);
defineSymbol(symbols_math, ams, rel, "\u225C", "\\triangleq", true);
defineSymbol(symbols_math, ams, rel, "\u223C", "\\thicksim");
defineSymbol(symbols_math, ams, rel, "\u2248", "\\thickapprox");
defineSymbol(symbols_math, ams, rel, "\u2AC6", "\\supseteqq", true);
defineSymbol(symbols_math, ams, rel, "\u22D1", "\\Supset", true);
defineSymbol(symbols_math, ams, rel, "\u2290", "\\sqsupset", true);
defineSymbol(symbols_math, ams, rel, "\u227D", "\\succcurlyeq", true);
defineSymbol(symbols_math, ams, rel, "\u22DF", "\\curlyeqsucc", true);
defineSymbol(symbols_math, ams, rel, "\u227F", "\\succsim", true);
defineSymbol(symbols_math, ams, rel, "\u2AB8", "\\succapprox", true);
defineSymbol(symbols_math, ams, rel, "\u22B3", "\\vartriangleright");
defineSymbol(symbols_math, ams, rel, "\u22B5", "\\trianglerighteq");
defineSymbol(symbols_math, ams, rel, "\u22A9", "\\Vdash", true);
defineSymbol(symbols_math, ams, rel, "\u2223", "\\shortmid");
defineSymbol(symbols_math, ams, rel, "\u2225", "\\shortparallel");
defineSymbol(symbols_math, ams, rel, "\u226C", "\\between", true);
defineSymbol(symbols_math, ams, rel, "\u22D4", "\\pitchfork", true);
defineSymbol(symbols_math, ams, rel, "\u221D", "\\varpropto");
defineSymbol(symbols_math, ams, rel, "\u25C0", "\\blacktriangleleft"); // unicode-math says that \therefore is a mathord atom.
// We kept the amssymb atom type, which is rel.

defineSymbol(symbols_math, ams, rel, "\u2234", "\\therefore", true);
defineSymbol(symbols_math, ams, rel, "\u220D", "\\backepsilon");
defineSymbol(symbols_math, ams, rel, "\u25B6", "\\blacktriangleright"); // unicode-math says that \because is a mathord atom.
// We kept the amssymb atom type, which is rel.

defineSymbol(symbols_math, ams, rel, "\u2235", "\\because", true);
defineSymbol(symbols_math, ams, rel, "\u22D8", "\\llless");
defineSymbol(symbols_math, ams, rel, "\u22D9", "\\gggtr");
defineSymbol(symbols_math, ams, bin, "\u22B2", "\\lhd");
defineSymbol(symbols_math, ams, bin, "\u22B3", "\\rhd");
defineSymbol(symbols_math, ams, rel, "\u2242", "\\eqsim", true);
defineSymbol(symbols_math, main, rel, "\u22C8", "\\Join");
defineSymbol(symbols_math, ams, rel, "\u2251", "\\Doteq", true); // AMS Binary Operators

defineSymbol(symbols_math, ams, bin, "\u2214", "\\dotplus", true);
defineSymbol(symbols_math, ams, bin, "\u2216", "\\smallsetminus");
defineSymbol(symbols_math, ams, bin, "\u22D2", "\\Cap", true);
defineSymbol(symbols_math, ams, bin, "\u22D3", "\\Cup", true);
defineSymbol(symbols_math, ams, bin, "\u2A5E", "\\doublebarwedge", true);
defineSymbol(symbols_math, ams, bin, "\u229F", "\\boxminus", true);
defineSymbol(symbols_math, ams, bin, "\u229E", "\\boxplus", true);
defineSymbol(symbols_math, ams, bin, "\u22C7", "\\divideontimes", true);
defineSymbol(symbols_math, ams, bin, "\u22C9", "\\ltimes", true);
defineSymbol(symbols_math, ams, bin, "\u22CA", "\\rtimes", true);
defineSymbol(symbols_math, ams, bin, "\u22CB", "\\leftthreetimes", true);
defineSymbol(symbols_math, ams, bin, "\u22CC", "\\rightthreetimes", true);
defineSymbol(symbols_math, ams, bin, "\u22CF", "\\curlywedge", true);
defineSymbol(symbols_math, ams, bin, "\u22CE", "\\curlyvee", true);
defineSymbol(symbols_math, ams, bin, "\u229D", "\\circleddash", true);
defineSymbol(symbols_math, ams, bin, "\u229B", "\\circledast", true);
defineSymbol(symbols_math, ams, bin, "\u22C5", "\\centerdot");
defineSymbol(symbols_math, ams, bin, "\u22BA", "\\intercal", true);
defineSymbol(symbols_math, ams, bin, "\u22D2", "\\doublecap");
defineSymbol(symbols_math, ams, bin, "\u22D3", "\\doublecup");
defineSymbol(symbols_math, ams, bin, "\u22A0", "\\boxtimes", true); // AMS Arrows
// Note: unicode-math maps \u21e2 to their own function \rightdasharrow.
// We'll map it to AMS function \dashrightarrow. It produces the same atom.

defineSymbol(symbols_math, ams, rel, "\u21E2", "\\dashrightarrow", true); // unicode-math maps \u21e0 to \leftdasharrow. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u21E0", "\\dashleftarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21C7", "\\leftleftarrows", true);
defineSymbol(symbols_math, ams, rel, "\u21C6", "\\leftrightarrows", true);
defineSymbol(symbols_math, ams, rel, "\u21DA", "\\Lleftarrow", true);
defineSymbol(symbols_math, ams, rel, "\u219E", "\\twoheadleftarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21A2", "\\leftarrowtail", true);
defineSymbol(symbols_math, ams, rel, "\u21AB", "\\looparrowleft", true);
defineSymbol(symbols_math, ams, rel, "\u21CB", "\\leftrightharpoons", true);
defineSymbol(symbols_math, ams, rel, "\u21B6", "\\curvearrowleft", true); // unicode-math maps \u21ba to \acwopencirclearrow. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u21BA", "\\circlearrowleft", true);
defineSymbol(symbols_math, ams, rel, "\u21B0", "\\Lsh", true);
defineSymbol(symbols_math, ams, rel, "\u21C8", "\\upuparrows", true);
defineSymbol(symbols_math, ams, rel, "\u21BF", "\\upharpoonleft", true);
defineSymbol(symbols_math, ams, rel, "\u21C3", "\\downharpoonleft", true);
defineSymbol(symbols_math, ams, rel, "\u22B8", "\\multimap", true);
defineSymbol(symbols_math, ams, rel, "\u21AD", "\\leftrightsquigarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21C9", "\\rightrightarrows", true);
defineSymbol(symbols_math, ams, rel, "\u21C4", "\\rightleftarrows", true);
defineSymbol(symbols_math, ams, rel, "\u21A0", "\\twoheadrightarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21A3", "\\rightarrowtail", true);
defineSymbol(symbols_math, ams, rel, "\u21AC", "\\looparrowright", true);
defineSymbol(symbols_math, ams, rel, "\u21B7", "\\curvearrowright", true); // unicode-math maps \u21bb to \cwopencirclearrow. We'll use the AMS synonym.

defineSymbol(symbols_math, ams, rel, "\u21BB", "\\circlearrowright", true);
defineSymbol(symbols_math, ams, rel, "\u21B1", "\\Rsh", true);
defineSymbol(symbols_math, ams, rel, "\u21CA", "\\downdownarrows", true);
defineSymbol(symbols_math, ams, rel, "\u21BE", "\\upharpoonright", true);
defineSymbol(symbols_math, ams, rel, "\u21C2", "\\downharpoonright", true);
defineSymbol(symbols_math, ams, rel, "\u21DD", "\\rightsquigarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21DD", "\\leadsto");
defineSymbol(symbols_math, ams, rel, "\u21DB", "\\Rrightarrow", true);
defineSymbol(symbols_math, ams, rel, "\u21BE", "\\restriction");
defineSymbol(symbols_math, main, symbols_textord, "\u2018", "`");
defineSymbol(symbols_math, main, symbols_textord, "$", "\\$");
defineSymbol(symbols_text, main, symbols_textord, "$", "\\$");
defineSymbol(symbols_text, main, symbols_textord, "$", "\\textdollar");
defineSymbol(symbols_math, main, symbols_textord, "%", "\\%");
defineSymbol(symbols_text, main, symbols_textord, "%", "\\%");
defineSymbol(symbols_math, main, symbols_textord, "_", "\\_");
defineSymbol(symbols_text, main, symbols_textord, "_", "\\_");
defineSymbol(symbols_text, main, symbols_textord, "_", "\\textunderscore");
defineSymbol(symbols_math, main, symbols_textord, "\u2220", "\\angle", true);
defineSymbol(symbols_math, main, symbols_textord, "\u221E", "\\infty", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2032", "\\prime");
defineSymbol(symbols_math, main, symbols_textord, "\u25B3", "\\triangle");
defineSymbol(symbols_math, main, symbols_textord, "\u0393", "\\Gamma", true);
defineSymbol(symbols_math, main, symbols_textord, "\u0394", "\\Delta", true);
defineSymbol(symbols_math, main, symbols_textord, "\u0398", "\\Theta", true);
defineSymbol(symbols_math, main, symbols_textord, "\u039B", "\\Lambda", true);
defineSymbol(symbols_math, main, symbols_textord, "\u039E", "\\Xi", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A0", "\\Pi", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A3", "\\Sigma", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A5", "\\Upsilon", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A6", "\\Phi", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A8", "\\Psi", true);
defineSymbol(symbols_math, main, symbols_textord, "\u03A9", "\\Omega", true);
defineSymbol(symbols_math, main, symbols_textord, "A", "\u0391");
defineSymbol(symbols_math, main, symbols_textord, "B", "\u0392");
defineSymbol(symbols_math, main, symbols_textord, "E", "\u0395");
defineSymbol(symbols_math, main, symbols_textord, "Z", "\u0396");
defineSymbol(symbols_math, main, symbols_textord, "H", "\u0397");
defineSymbol(symbols_math, main, symbols_textord, "I", "\u0399");
defineSymbol(symbols_math, main, symbols_textord, "K", "\u039A");
defineSymbol(symbols_math, main, symbols_textord, "M", "\u039C");
defineSymbol(symbols_math, main, symbols_textord, "N", "\u039D");
defineSymbol(symbols_math, main, symbols_textord, "O", "\u039F");
defineSymbol(symbols_math, main, symbols_textord, "P", "\u03A1");
defineSymbol(symbols_math, main, symbols_textord, "T", "\u03A4");
defineSymbol(symbols_math, main, symbols_textord, "X", "\u03A7");
defineSymbol(symbols_math, main, symbols_textord, "\xAC", "\\neg", true);
defineSymbol(symbols_math, main, symbols_textord, "\xAC", "\\lnot");
defineSymbol(symbols_math, main, symbols_textord, "\u22A4", "\\top");
defineSymbol(symbols_math, main, symbols_textord, "\u22A5", "\\bot");
defineSymbol(symbols_math, main, symbols_textord, "\u2205", "\\emptyset");
defineSymbol(symbols_math, ams, symbols_textord, "\u2205", "\\varnothing");
defineSymbol(symbols_math, main, mathord, "\u03B1", "\\alpha", true);
defineSymbol(symbols_math, main, mathord, "\u03B2", "\\beta", true);
defineSymbol(symbols_math, main, mathord, "\u03B3", "\\gamma", true);
defineSymbol(symbols_math, main, mathord, "\u03B4", "\\delta", true);
defineSymbol(symbols_math, main, mathord, "\u03F5", "\\epsilon", true);
defineSymbol(symbols_math, main, mathord, "\u03B6", "\\zeta", true);
defineSymbol(symbols_math, main, mathord, "\u03B7", "\\eta", true);
defineSymbol(symbols_math, main, mathord, "\u03B8", "\\theta", true);
defineSymbol(symbols_math, main, mathord, "\u03B9", "\\iota", true);
defineSymbol(symbols_math, main, mathord, "\u03BA", "\\kappa", true);
defineSymbol(symbols_math, main, mathord, "\u03BB", "\\lambda", true);
defineSymbol(symbols_math, main, mathord, "\u03BC", "\\mu", true);
defineSymbol(symbols_math, main, mathord, "\u03BD", "\\nu", true);
defineSymbol(symbols_math, main, mathord, "\u03BE", "\\xi", true);
defineSymbol(symbols_math, main, mathord, "\u03BF", "\\omicron", true);
defineSymbol(symbols_math, main, mathord, "\u03C0", "\\pi", true);
defineSymbol(symbols_math, main, mathord, "\u03C1", "\\rho", true);
defineSymbol(symbols_math, main, mathord, "\u03C3", "\\sigma", true);
defineSymbol(symbols_math, main, mathord, "\u03C4", "\\tau", true);
defineSymbol(symbols_math, main, mathord, "\u03C5", "\\upsilon", true);
defineSymbol(symbols_math, main, mathord, "\u03D5", "\\phi", true);
defineSymbol(symbols_math, main, mathord, "\u03C7", "\\chi", true);
defineSymbol(symbols_math, main, mathord, "\u03C8", "\\psi", true);
defineSymbol(symbols_math, main, mathord, "\u03C9", "\\omega", true);
defineSymbol(symbols_math, main, mathord, "\u03B5", "\\varepsilon", true);
defineSymbol(symbols_math, main, mathord, "\u03D1", "\\vartheta", true);
defineSymbol(symbols_math, main, mathord, "\u03D6", "\\varpi", true);
defineSymbol(symbols_math, main, mathord, "\u03F1", "\\varrho", true);
defineSymbol(symbols_math, main, mathord, "\u03C2", "\\varsigma", true);
defineSymbol(symbols_math, main, mathord, "\u03C6", "\\varphi", true);
defineSymbol(symbols_math, main, bin, "\u2217", "*");
defineSymbol(symbols_math, main, bin, "+", "+");
defineSymbol(symbols_math, main, bin, "\u2212", "-");
defineSymbol(symbols_math, main, bin, "\u22C5", "\\cdot", true);
defineSymbol(symbols_math, main, bin, "\u2218", "\\circ");
defineSymbol(symbols_math, main, bin, "\xF7", "\\div", true);
defineSymbol(symbols_math, main, bin, "\xB1", "\\pm", true);
defineSymbol(symbols_math, main, bin, "\xD7", "\\times", true);
defineSymbol(symbols_math, main, bin, "\u2229", "\\cap", true);
defineSymbol(symbols_math, main, bin, "\u222A", "\\cup", true);
defineSymbol(symbols_math, main, bin, "\u2216", "\\setminus");
defineSymbol(symbols_math, main, bin, "\u2227", "\\land");
defineSymbol(symbols_math, main, bin, "\u2228", "\\lor");
defineSymbol(symbols_math, main, bin, "\u2227", "\\wedge", true);
defineSymbol(symbols_math, main, bin, "\u2228", "\\vee", true);
defineSymbol(symbols_math, main, symbols_textord, "\u221A", "\\surd");
defineSymbol(symbols_math, main, symbols_open, "(", "(");
defineSymbol(symbols_math, main, symbols_open, "[", "[");
defineSymbol(symbols_math, main, symbols_open, "\u27E8", "\\langle", true);
defineSymbol(symbols_math, main, symbols_open, "\u2223", "\\lvert");
defineSymbol(symbols_math, main, symbols_open, "\u2225", "\\lVert");
defineSymbol(symbols_math, main, symbols_close, ")", ")");
defineSymbol(symbols_math, main, symbols_close, "]", "]");
defineSymbol(symbols_math, main, symbols_close, "?", "?");
defineSymbol(symbols_math, main, symbols_close, "!", "!");
defineSymbol(symbols_math, main, symbols_close, "\u27E9", "\\rangle", true);
defineSymbol(symbols_math, main, symbols_close, "\u2223", "\\rvert");
defineSymbol(symbols_math, main, symbols_close, "\u2225", "\\rVert");
defineSymbol(symbols_math, main, rel, "=", "=");
defineSymbol(symbols_math, main, rel, "<", "<");
defineSymbol(symbols_math, main, rel, ">", ">");
defineSymbol(symbols_math, main, rel, ":", ":");
defineSymbol(symbols_math, main, rel, "\u2248", "\\approx", true);
defineSymbol(symbols_math, main, rel, "\u2245", "\\cong", true);
defineSymbol(symbols_math, main, rel, "\u2265", "\\ge");
defineSymbol(symbols_math, main, rel, "\u2265", "\\geq", true);
defineSymbol(symbols_math, main, rel, "\u2190", "\\gets");
defineSymbol(symbols_math, main, rel, ">", "\\gt");
defineSymbol(symbols_math, main, rel, "\u2208", "\\in", true);
defineSymbol(symbols_math, main, rel, "\uE020", "\\@not");
defineSymbol(symbols_math, main, rel, "\u2282", "\\subset", true);
defineSymbol(symbols_math, main, rel, "\u2283", "\\supset", true);
defineSymbol(symbols_math, main, rel, "\u2286", "\\subseteq", true);
defineSymbol(symbols_math, main, rel, "\u2287", "\\supseteq", true);
defineSymbol(symbols_math, ams, rel, "\u2288", "\\nsubseteq", true);
defineSymbol(symbols_math, ams, rel, "\u2289", "\\nsupseteq", true);
defineSymbol(symbols_math, main, rel, "\u22A8", "\\models");
defineSymbol(symbols_math, main, rel, "\u2190", "\\leftarrow", true);
defineSymbol(symbols_math, main, rel, "\u2264", "\\le");
defineSymbol(symbols_math, main, rel, "\u2264", "\\leq", true);
defineSymbol(symbols_math, main, rel, "<", "\\lt");
defineSymbol(symbols_math, main, rel, "\u2192", "\\rightarrow", true);
defineSymbol(symbols_math, main, rel, "\u2192", "\\to");
defineSymbol(symbols_math, ams, rel, "\u2271", "\\ngeq", true);
defineSymbol(symbols_math, ams, rel, "\u2270", "\\nleq", true);
defineSymbol(symbols_math, main, symbols_spacing, "\xA0", "\\ ");
defineSymbol(symbols_math, main, symbols_spacing, "\xA0", "~");
defineSymbol(symbols_math, main, symbols_spacing, "\xA0", "\\space"); // Ref: LaTeX Source 2e: \DeclareRobustCommand{\nobreakspace}{%

defineSymbol(symbols_math, main, symbols_spacing, "\xA0", "\\nobreakspace");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "\\ ");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "~");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "\\space");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "\\nobreakspace");
defineSymbol(symbols_math, main, symbols_spacing, null, "\\nobreak");
defineSymbol(symbols_math, main, symbols_spacing, null, "\\allowbreak");
defineSymbol(symbols_math, main, punct, ",", ",");
defineSymbol(symbols_math, main, punct, ";", ";");
defineSymbol(symbols_math, ams, bin, "\u22BC", "\\barwedge", true);
defineSymbol(symbols_math, ams, bin, "\u22BB", "\\veebar", true);
defineSymbol(symbols_math, main, bin, "\u2299", "\\odot", true);
defineSymbol(symbols_math, main, bin, "\u2295", "\\oplus", true);
defineSymbol(symbols_math, main, bin, "\u2297", "\\otimes", true);
defineSymbol(symbols_math, main, symbols_textord, "\u2202", "\\partial", true);
defineSymbol(symbols_math, main, bin, "\u2298", "\\oslash", true);
defineSymbol(symbols_math, ams, bin, "\u229A", "\\circledcirc", true);
defineSymbol(symbols_math, ams, bin, "\u22A1", "\\boxdot", true);
defineSymbol(symbols_math, main, bin, "\u25B3", "\\bigtriangleup");
defineSymbol(symbols_math, main, bin, "\u25BD", "\\bigtriangledown");
defineSymbol(symbols_math, main, bin, "\u2020", "\\dagger");
defineSymbol(symbols_math, main, bin, "\u22C4", "\\diamond");
defineSymbol(symbols_math, main, bin, "\u22C6", "\\star");
defineSymbol(symbols_math, main, bin, "\u25C3", "\\triangleleft");
defineSymbol(symbols_math, main, bin, "\u25B9", "\\triangleright");
defineSymbol(symbols_math, main, symbols_open, "{", "\\{");
defineSymbol(symbols_text, main, symbols_textord, "{", "\\{");
defineSymbol(symbols_text, main, symbols_textord, "{", "\\textbraceleft");
defineSymbol(symbols_math, main, symbols_close, "}", "\\}");
defineSymbol(symbols_text, main, symbols_textord, "}", "\\}");
defineSymbol(symbols_text, main, symbols_textord, "}", "\\textbraceright");
defineSymbol(symbols_math, main, symbols_open, "{", "\\lbrace");
defineSymbol(symbols_math, main, symbols_close, "}", "\\rbrace");
defineSymbol(symbols_math, main, symbols_open, "[", "\\lbrack");
defineSymbol(symbols_text, main, symbols_textord, "[", "\\lbrack");
defineSymbol(symbols_math, main, symbols_close, "]", "\\rbrack");
defineSymbol(symbols_text, main, symbols_textord, "]", "\\rbrack");
defineSymbol(symbols_math, main, symbols_open, "(", "\\lparen");
defineSymbol(symbols_math, main, symbols_close, ")", "\\rparen");
defineSymbol(symbols_text, main, symbols_textord, "<", "\\textless"); // in T1 fontenc

defineSymbol(symbols_text, main, symbols_textord, ">", "\\textgreater"); // in T1 fontenc

defineSymbol(symbols_math, main, symbols_open, "\u230A", "\\lfloor", true);
defineSymbol(symbols_math, main, symbols_close, "\u230B", "\\rfloor", true);
defineSymbol(symbols_math, main, symbols_open, "\u2308", "\\lceil", true);
defineSymbol(symbols_math, main, symbols_close, "\u2309", "\\rceil", true);
defineSymbol(symbols_math, main, symbols_textord, "\\", "\\backslash");
defineSymbol(symbols_math, main, symbols_textord, "\u2223", "|");
defineSymbol(symbols_math, main, symbols_textord, "\u2223", "\\vert");
defineSymbol(symbols_text, main, symbols_textord, "|", "\\textbar"); // in T1 fontenc

defineSymbol(symbols_math, main, symbols_textord, "\u2225", "\\|");
defineSymbol(symbols_math, main, symbols_textord, "\u2225", "\\Vert");
defineSymbol(symbols_text, main, symbols_textord, "\u2225", "\\textbardbl");
defineSymbol(symbols_text, main, symbols_textord, "~", "\\textasciitilde");
defineSymbol(symbols_text, main, symbols_textord, "\\", "\\textbackslash");
defineSymbol(symbols_text, main, symbols_textord, "^", "\\textasciicircum");
defineSymbol(symbols_math, main, rel, "\u2191", "\\uparrow", true);
defineSymbol(symbols_math, main, rel, "\u21D1", "\\Uparrow", true);
defineSymbol(symbols_math, main, rel, "\u2193", "\\downarrow", true);
defineSymbol(symbols_math, main, rel, "\u21D3", "\\Downarrow", true);
defineSymbol(symbols_math, main, rel, "\u2195", "\\updownarrow", true);
defineSymbol(symbols_math, main, rel, "\u21D5", "\\Updownarrow", true);
defineSymbol(symbols_math, main, op, "\u2210", "\\coprod");
defineSymbol(symbols_math, main, op, "\u22C1", "\\bigvee");
defineSymbol(symbols_math, main, op, "\u22C0", "\\bigwedge");
defineSymbol(symbols_math, main, op, "\u2A04", "\\biguplus");
defineSymbol(symbols_math, main, op, "\u22C2", "\\bigcap");
defineSymbol(symbols_math, main, op, "\u22C3", "\\bigcup");
defineSymbol(symbols_math, main, op, "\u222B", "\\int");
defineSymbol(symbols_math, main, op, "\u222B", "\\intop");
defineSymbol(symbols_math, main, op, "\u222C", "\\iint");
defineSymbol(symbols_math, main, op, "\u222D", "\\iiint");
defineSymbol(symbols_math, main, op, "\u220F", "\\prod");
defineSymbol(symbols_math, main, op, "\u2211", "\\sum");
defineSymbol(symbols_math, main, op, "\u2A02", "\\bigotimes");
defineSymbol(symbols_math, main, op, "\u2A01", "\\bigoplus");
defineSymbol(symbols_math, main, op, "\u2A00", "\\bigodot");
defineSymbol(symbols_math, main, op, "\u222E", "\\oint");
defineSymbol(symbols_math, main, op, "\u222F", "\\oiint");
defineSymbol(symbols_math, main, op, "\u2230", "\\oiiint");
defineSymbol(symbols_math, main, op, "\u2A06", "\\bigsqcup");
defineSymbol(symbols_math, main, op, "\u222B", "\\smallint");
defineSymbol(symbols_text, main, symbols_inner, "\u2026", "\\textellipsis");
defineSymbol(symbols_math, main, symbols_inner, "\u2026", "\\mathellipsis");
defineSymbol(symbols_text, main, symbols_inner, "\u2026", "\\ldots", true);
defineSymbol(symbols_math, main, symbols_inner, "\u2026", "\\ldots", true);
defineSymbol(symbols_math, main, symbols_inner, "\u22EF", "\\@cdots", true);
defineSymbol(symbols_math, main, symbols_inner, "\u22F1", "\\ddots", true);
defineSymbol(symbols_math, main, symbols_textord, "\u22EE", "\\varvdots"); // \vdots is a macro

defineSymbol(symbols_math, main, symbols_accent, "\u02CA", "\\acute");
defineSymbol(symbols_math, main, symbols_accent, "\u02CB", "\\grave");
defineSymbol(symbols_math, main, symbols_accent, "\xA8", "\\ddot");
defineSymbol(symbols_math, main, symbols_accent, "~", "\\tilde");
defineSymbol(symbols_math, main, symbols_accent, "\u02C9", "\\bar");
defineSymbol(symbols_math, main, symbols_accent, "\u02D8", "\\breve");
defineSymbol(symbols_math, main, symbols_accent, "\u02C7", "\\check");
defineSymbol(symbols_math, main, symbols_accent, "^", "\\hat");
defineSymbol(symbols_math, main, symbols_accent, "\u20D7", "\\vec");
defineSymbol(symbols_math, main, symbols_accent, "\u02D9", "\\dot");
defineSymbol(symbols_math, main, symbols_accent, "\u02DA", "\\mathring");
defineSymbol(symbols_math, main, mathord, "\u0131", "\\imath", true);
defineSymbol(symbols_math, main, mathord, "\u0237", "\\jmath", true);
defineSymbol(symbols_text, main, symbols_textord, "\u0131", "\\i", true);
defineSymbol(symbols_text, main, symbols_textord, "\u0237", "\\j", true);
defineSymbol(symbols_text, main, symbols_textord, "\xDF", "\\ss", true);
defineSymbol(symbols_text, main, symbols_textord, "\xE6", "\\ae", true);
defineSymbol(symbols_text, main, symbols_textord, "\xE6", "\\ae", true);
defineSymbol(symbols_text, main, symbols_textord, "\u0153", "\\oe", true);
defineSymbol(symbols_text, main, symbols_textord, "\xF8", "\\o", true);
defineSymbol(symbols_text, main, symbols_textord, "\xC6", "\\AE", true);
defineSymbol(symbols_text, main, symbols_textord, "\u0152", "\\OE", true);
defineSymbol(symbols_text, main, symbols_textord, "\xD8", "\\O", true);
defineSymbol(symbols_text, main, symbols_accent, "\u02CA", "\\'"); // acute

defineSymbol(symbols_text, main, symbols_accent, "\u02CB", "\\`"); // grave

defineSymbol(symbols_text, main, symbols_accent, "\u02C6", "\\^"); // circumflex

defineSymbol(symbols_text, main, symbols_accent, "\u02DC", "\\~"); // tilde

defineSymbol(symbols_text, main, symbols_accent, "\u02C9", "\\="); // macron

defineSymbol(symbols_text, main, symbols_accent, "\u02D8", "\\u"); // breve

defineSymbol(symbols_text, main, symbols_accent, "\u02D9", "\\."); // dot above

defineSymbol(symbols_text, main, symbols_accent, "\u02DA", "\\r"); // ring above

defineSymbol(symbols_text, main, symbols_accent, "\u02C7", "\\v"); // caron

defineSymbol(symbols_text, main, symbols_accent, "\xA8", '\\"'); // diaresis

defineSymbol(symbols_text, main, symbols_accent, "\u02DD", "\\H"); // double acute

defineSymbol(symbols_text, main, symbols_accent, "\u25EF", "\\textcircled"); // \bigcirc glyph
// These ligatures are detected and created in Parser.js's `formLigatures`.

var ligatures = {
  "--": true,
  "---": true,
  "``": true,
  "''": true
};
defineSymbol(symbols_text, main, symbols_textord, "\u2013", "--");
defineSymbol(symbols_text, main, symbols_textord, "\u2013", "\\textendash");
defineSymbol(symbols_text, main, symbols_textord, "\u2014", "---");
defineSymbol(symbols_text, main, symbols_textord, "\u2014", "\\textemdash");
defineSymbol(symbols_text, main, symbols_textord, "\u2018", "`");
defineSymbol(symbols_text, main, symbols_textord, "\u2018", "\\textquoteleft");
defineSymbol(symbols_text, main, symbols_textord, "\u2019", "'");
defineSymbol(symbols_text, main, symbols_textord, "\u2019", "\\textquoteright");
defineSymbol(symbols_text, main, symbols_textord, "\u201C", "``");
defineSymbol(symbols_text, main, symbols_textord, "\u201C", "\\textquotedblleft");
defineSymbol(symbols_text, main, symbols_textord, "\u201D", "''");
defineSymbol(symbols_text, main, symbols_textord, "\u201D", "\\textquotedblright"); //  \degree from gensymb package

defineSymbol(symbols_math, main, symbols_textord, "\xB0", "\\degree", true);
defineSymbol(symbols_text, main, symbols_textord, "\xB0", "\\degree"); // \textdegree from inputenc package

defineSymbol(symbols_text, main, symbols_textord, "\xB0", "\\textdegree", true); // TODO: In LaTeX, \pounds can generate a different character in text and math
// mode, but among our fonts, only Main-Italic defines this character "163".

defineSymbol(symbols_math, main, mathord, "\xA3", "\\pounds");
defineSymbol(symbols_math, main, mathord, "\xA3", "\\mathsterling", true);
defineSymbol(symbols_text, main, mathord, "\xA3", "\\pounds");
defineSymbol(symbols_text, main, mathord, "\xA3", "\\textsterling", true);
defineSymbol(symbols_math, ams, symbols_textord, "\u2720", "\\maltese");
defineSymbol(symbols_text, ams, symbols_textord, "\u2720", "\\maltese");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "\\ ");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", " ");
defineSymbol(symbols_text, main, symbols_spacing, "\xA0", "~"); // There are lots of symbols which are the same, so we add them in afterwards.
// All of these are textords in math mode

var mathTextSymbols = "0123456789/@.\"";

for (var symbols_i = 0; symbols_i < mathTextSymbols.length; symbols_i++) {
  var symbols_ch = mathTextSymbols.charAt(symbols_i);
  defineSymbol(symbols_math, main, symbols_textord, symbols_ch, symbols_ch);
} // All of these are textords in text mode


var textSymbols = "0123456789!@*()-=+[]<>|\";:?/.,";

for (var src_symbols_i = 0; src_symbols_i < textSymbols.length; src_symbols_i++) {
  var _ch = textSymbols.charAt(src_symbols_i);

  defineSymbol(symbols_text, main, symbols_textord, _ch, _ch);
} // All of these are textords in text mode, and mathords in math mode


var letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

for (var symbols_i2 = 0; symbols_i2 < letters.length; symbols_i2++) {
  var _ch2 = letters.charAt(symbols_i2);

  defineSymbol(symbols_math, main, mathord, _ch2, _ch2);
  defineSymbol(symbols_text, main, symbols_textord, _ch2, _ch2);
} // Blackboard bold and script letters in Unicode range


defineSymbol(symbols_math, ams, symbols_textord, "C", "\u2102"); // blackboard bold

defineSymbol(symbols_text, ams, symbols_textord, "C", "\u2102");
defineSymbol(symbols_math, ams, symbols_textord, "H", "\u210D");
defineSymbol(symbols_text, ams, symbols_textord, "H", "\u210D");
defineSymbol(symbols_math, ams, symbols_textord, "N", "\u2115");
defineSymbol(symbols_text, ams, symbols_textord, "N", "\u2115");
defineSymbol(symbols_math, ams, symbols_textord, "P", "\u2119");
defineSymbol(symbols_text, ams, symbols_textord, "P", "\u2119");
defineSymbol(symbols_math, ams, symbols_textord, "Q", "\u211A");
defineSymbol(symbols_text, ams, symbols_textord, "Q", "\u211A");
defineSymbol(symbols_math, ams, symbols_textord, "R", "\u211D");
defineSymbol(symbols_text, ams, symbols_textord, "R", "\u211D");
defineSymbol(symbols_math, ams, symbols_textord, "Z", "\u2124");
defineSymbol(symbols_text, ams, symbols_textord, "Z", "\u2124");
defineSymbol(symbols_math, main, mathord, "h", "\u210E"); // italic h, Planck constant

defineSymbol(symbols_text, main, mathord, "h", "\u210E"); // The next loop loads wide (surrogate pair) characters.
// We support some letters in the Unicode range U+1D400 to U+1D7FF,
// Mathematical Alphanumeric Symbols.
// Some editors do not deal well with wide characters. So don't write the
// string into this file. Instead, create the string from the surrogate pair.

var symbols_wideChar = "";

for (var symbols_i3 = 0; symbols_i3 < letters.length; symbols_i3++) {
  var _ch3 = letters.charAt(symbols_i3); // The hex numbers in the next line are a surrogate pair.
  // 0xD835 is the high surrogate for all letters in the range we support.
  // 0xDC00 is the low surrogate for bold A.


  symbols_wideChar = String.fromCharCode(0xD835, 0xDC00 + symbols_i3); // A-Z a-z bold

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDC34 + symbols_i3); // A-Z a-z italic

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDC68 + symbols_i3); // A-Z a-z bold italic

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDD04 + symbols_i3); // A-Z a-z Fractur

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDDA0 + symbols_i3); // A-Z a-z sans-serif

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDDD4 + symbols_i3); // A-Z a-z sans bold

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDE08 + symbols_i3); // A-Z a-z sans italic

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDE70 + symbols_i3); // A-Z a-z monospace

  defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);

  if (symbols_i3 < 26) {
    // KaTeX fonts have only capital letters for blackboard bold and script.
    // See exception for k below.
    symbols_wideChar = String.fromCharCode(0xD835, 0xDD38 + symbols_i3); // A-Z double struck

    defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
    defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
    symbols_wideChar = String.fromCharCode(0xD835, 0xDC9C + symbols_i3); // A-Z script

    defineSymbol(symbols_math, main, mathord, _ch3, symbols_wideChar);
    defineSymbol(symbols_text, main, symbols_textord, _ch3, symbols_wideChar);
  } // TODO: Add bold script when it is supported by a KaTeX font.

} // "k" is the only double struck lower case letter in the KaTeX fonts.


symbols_wideChar = String.fromCharCode(0xD835, 0xDD5C); // k double struck

defineSymbol(symbols_math, main, mathord, "k", symbols_wideChar);
defineSymbol(symbols_text, main, symbols_textord, "k", symbols_wideChar); // Next, some wide character numerals

for (var symbols_i4 = 0; symbols_i4 < 10; symbols_i4++) {
  var _ch4 = symbols_i4.toString();

  symbols_wideChar = String.fromCharCode(0xD835, 0xDFCE + symbols_i4); // 0-9 bold

  defineSymbol(symbols_math, main, mathord, _ch4, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch4, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDFE2 + symbols_i4); // 0-9 sans serif

  defineSymbol(symbols_math, main, mathord, _ch4, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch4, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDFEC + symbols_i4); // 0-9 bold sans

  defineSymbol(symbols_math, main, mathord, _ch4, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch4, symbols_wideChar);
  symbols_wideChar = String.fromCharCode(0xD835, 0xDFF6 + symbols_i4); // 0-9 monospace

  defineSymbol(symbols_math, main, mathord, _ch4, symbols_wideChar);
  defineSymbol(symbols_text, main, symbols_textord, _ch4, symbols_wideChar);
} // We add these Latin-1 letters as symbols for backwards-compatibility,
// but they are not actually in the font, nor are they supported by the
// Unicode accent mechanism, so they fall back to Times font and look ugly.
// TODO(edemaine): Fix this.


var extraLatin = "ÇÐÞçþ";

for (var _i5 = 0; _i5 < extraLatin.length; _i5++) {
  var _ch5 = extraLatin.charAt(_i5);

  defineSymbol(symbols_math, main, mathord, _ch5, _ch5);
  defineSymbol(symbols_text, main, symbols_textord, _ch5, _ch5);
}

defineSymbol(symbols_text, main, symbols_textord, "ð", "ð"); // Unicode versions of existing characters

defineSymbol(symbols_text, main, symbols_textord, "\u2013", "–");
defineSymbol(symbols_text, main, symbols_textord, "\u2014", "—");
defineSymbol(symbols_text, main, symbols_textord, "\u2018", "‘");
defineSymbol(symbols_text, main, symbols_textord, "\u2019", "’");
defineSymbol(symbols_text, main, symbols_textord, "\u201C", "“");
defineSymbol(symbols_text, main, symbols_textord, "\u201D", "”");
// CONCATENATED MODULE: ./src/wide-character.js
/**
 * This file provides support for Unicode range U+1D400 to U+1D7FF,
 * Mathematical Alphanumeric Symbols.
 *
 * Function wideCharacterFont takes a wide character as input and returns
 * the font information necessary to render it properly.
 */

/**
 * Data below is from https://www.unicode.org/charts/PDF/U1D400.pdf
 * That document sorts characters into groups by font type, say bold or italic.
 *
 * In the arrays below, each subarray consists three elements:
 *      * The CSS class of that group when in math mode.
 *      * The CSS class of that group when in text mode.
 *      * The font name, so that KaTeX can get font metrics.
 */

var wideLatinLetterData = [["mathbf", "textbf", "Main-Bold"], // A-Z bold upright
["mathbf", "textbf", "Main-Bold"], // a-z bold upright
["mathdefault", "textit", "Math-Italic"], // A-Z italic
["mathdefault", "textit", "Math-Italic"], // a-z italic
["boldsymbol", "boldsymbol", "Main-BoldItalic"], // A-Z bold italic
["boldsymbol", "boldsymbol", "Main-BoldItalic"], // a-z bold italic
// Map fancy A-Z letters to script, not calligraphic.
// This aligns with unicode-math and math fonts (except Cambria Math).
["mathscr", "textscr", "Script-Regular"], // A-Z script
["", "", ""], // a-z script.  No font
["", "", ""], // A-Z bold script. No font
["", "", ""], // a-z bold script. No font
["mathfrak", "textfrak", "Fraktur-Regular"], // A-Z Fraktur
["mathfrak", "textfrak", "Fraktur-Regular"], // a-z Fraktur
["mathbb", "textbb", "AMS-Regular"], // A-Z double-struck
["mathbb", "textbb", "AMS-Regular"], // k double-struck
["", "", ""], // A-Z bold Fraktur No font metrics
["", "", ""], // a-z bold Fraktur.   No font.
["mathsf", "textsf", "SansSerif-Regular"], // A-Z sans-serif
["mathsf", "textsf", "SansSerif-Regular"], // a-z sans-serif
["mathboldsf", "textboldsf", "SansSerif-Bold"], // A-Z bold sans-serif
["mathboldsf", "textboldsf", "SansSerif-Bold"], // a-z bold sans-serif
["mathitsf", "textitsf", "SansSerif-Italic"], // A-Z italic sans-serif
["mathitsf", "textitsf", "SansSerif-Italic"], // a-z italic sans-serif
["", "", ""], // A-Z bold italic sans. No font
["", "", ""], // a-z bold italic sans. No font
["mathtt", "texttt", "Typewriter-Regular"], // A-Z monospace
["mathtt", "texttt", "Typewriter-Regular"]];
var wideNumeralData = [["mathbf", "textbf", "Main-Bold"], // 0-9 bold
["", "", ""], // 0-9 double-struck. No KaTeX font.
["mathsf", "textsf", "SansSerif-Regular"], // 0-9 sans-serif
["mathboldsf", "textboldsf", "SansSerif-Bold"], // 0-9 bold sans-serif
["mathtt", "texttt", "Typewriter-Regular"]];
var wide_character_wideCharacterFont = function wideCharacterFont(wideChar, mode) {
  // IE doesn't support codePointAt(). So work with the surrogate pair.
  var H = wideChar.charCodeAt(0); // high surrogate

  var L = wideChar.charCodeAt(1); // low surrogate

  var codePoint = (H - 0xD800) * 0x400 + (L - 0xDC00) + 0x10000;
  var j = mode === "math" ? 0 : 1; // column index for CSS class.

  if (0x1D400 <= codePoint && codePoint < 0x1D6A4) {
    // wideLatinLetterData contains exactly 26 chars on each row.
    // So we can calculate the relevant row. No traverse necessary.
    var i = Math.floor((codePoint - 0x1D400) / 26);
    return [wideLatinLetterData[i][2], wideLatinLetterData[i][j]];
  } else if (0x1D7CE <= codePoint && codePoint <= 0x1D7FF) {
    // Numerals, ten per row.
    var _i = Math.floor((codePoint - 0x1D7CE) / 10);

    return [wideNumeralData[_i][2], wideNumeralData[_i][j]];
  } else if (codePoint === 0x1D6A5 || codePoint === 0x1D6A6) {
    // dotless i or j
    return [wideLatinLetterData[0][2], wideLatinLetterData[0][j]];
  } else if (0x1D6A6 < codePoint && codePoint < 0x1D7CE) {
    // Greek letters. Not supported, yet.
    return ["", ""];
  } else {
    // We don't support any wide characters outside 1D400–1D7FF.
    throw new src_ParseError("Unsupported character: " + wideChar);
  }
};
// CONCATENATED MODULE: ./src/Options.js
/**
 * This file contains information about the options that the Parser carries
 * around with it while parsing. Data is held in an `Options` object, and when
 * recursing, a new `Options` object can be created with the `.with*` and
 * `.reset` functions.
 */

var sizeStyleMap = [// Each element contains [textsize, scriptsize, scriptscriptsize].
// The size mappings are taken from TeX with \normalsize=10pt.
[1, 1, 1], // size1: [5, 5, 5]              \tiny
[2, 1, 1], // size2: [6, 5, 5]
[3, 1, 1], // size3: [7, 5, 5]              \scriptsize
[4, 2, 1], // size4: [8, 6, 5]              \footnotesize
[5, 2, 1], // size5: [9, 6, 5]              \small
[6, 3, 1], // size6: [10, 7, 5]             \normalsize
[7, 4, 2], // size7: [12, 8, 6]             \large
[8, 6, 3], // size8: [14.4, 10, 7]          \Large
[9, 7, 6], // size9: [17.28, 12, 10]        \LARGE
[10, 8, 7], // size10: [20.74, 14.4, 12]     \huge
[11, 10, 9]];
var sizeMultipliers = [// fontMetrics.js:getGlobalMetrics also uses size indexes, so if
// you change size indexes, change that function.
0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.2, 1.44, 1.728, 2.074, 2.488];

var sizeAtStyle = function sizeAtStyle(size, style) {
  return style.size < 2 ? size : sizeStyleMap[size - 1][style.size - 1];
}; // In these types, "" (empty string) means "no change".


/**
 * This is the main options class. It contains the current style, size, color,
 * and font.
 *
 * Options objects should not be modified. To create a new Options with
 * different properties, call a `.having*` method.
 */
var Options_Options =
/*#__PURE__*/
function () {
  // A font family applies to a group of fonts (i.e. SansSerif), while a font
  // represents a specific font (i.e. SansSerif Bold).
  // See: https://tex.stackexchange.com/questions/22350/difference-between-textrm-and-mathrm

  /**
   * The base size index.
   */
  function Options(data) {
    this.style = void 0;
    this.color = void 0;
    this.size = void 0;
    this.textSize = void 0;
    this.phantom = void 0;
    this.font = void 0;
    this.fontFamily = void 0;
    this.fontWeight = void 0;
    this.fontShape = void 0;
    this.sizeMultiplier = void 0;
    this.maxSize = void 0;
    this._fontMetrics = void 0;
    this.style = data.style;
    this.color = data.color;
    this.size = data.size || Options.BASESIZE;
    this.textSize = data.textSize || this.size;
    this.phantom = !!data.phantom;
    this.font = data.font || "";
    this.fontFamily = data.fontFamily || "";
    this.fontWeight = data.fontWeight || '';
    this.fontShape = data.fontShape || '';
    this.sizeMultiplier = sizeMultipliers[this.size - 1];
    this.maxSize = data.maxSize;
    this._fontMetrics = undefined;
  }
  /**
   * Returns a new options object with the same properties as "this".  Properties
   * from "extension" will be copied to the new options object.
   */


  var _proto = Options.prototype;

  _proto.extend = function extend(extension) {
    var data = {
      style: this.style,
      size: this.size,
      textSize: this.textSize,
      color: this.color,
      phantom: this.phantom,
      font: this.font,
      fontFamily: this.fontFamily,
      fontWeight: this.fontWeight,
      fontShape: this.fontShape,
      maxSize: this.maxSize
    };

    for (var key in extension) {
      if (extension.hasOwnProperty(key)) {
        data[key] = extension[key];
      }
    }

    return new Options(data);
  }
  /**
   * Return an options object with the given style. If `this.style === style`,
   * returns `this`.
   */
  ;

  _proto.havingStyle = function havingStyle(style) {
    if (this.style === style) {
      return this;
    } else {
      return this.extend({
        style: style,
        size: sizeAtStyle(this.textSize, style)
      });
    }
  }
  /**
   * Return an options object with a cramped version of the current style. If
   * the current style is cramped, returns `this`.
   */
  ;

  _proto.havingCrampedStyle = function havingCrampedStyle() {
    return this.havingStyle(this.style.cramp());
  }
  /**
   * Return an options object with the given size and in at least `\textstyle`.
   * Returns `this` if appropriate.
   */
  ;

  _proto.havingSize = function havingSize(size) {
    if (this.size === size && this.textSize === size) {
      return this;
    } else {
      return this.extend({
        style: this.style.text(),
        size: size,
        textSize: size,
        sizeMultiplier: sizeMultipliers[size - 1]
      });
    }
  }
  /**
   * Like `this.havingSize(BASESIZE).havingStyle(style)`. If `style` is omitted,
   * changes to at least `\textstyle`.
   */
  ;

  _proto.havingBaseStyle = function havingBaseStyle(style) {
    style = style || this.style.text();
    var wantSize = sizeAtStyle(Options.BASESIZE, style);

    if (this.size === wantSize && this.textSize === Options.BASESIZE && this.style === style) {
      return this;
    } else {
      return this.extend({
        style: style,
        size: wantSize
      });
    }
  }
  /**
   * Remove the effect of sizing changes such as \Huge.
   * Keep the effect of the current style, such as \scriptstyle.
   */
  ;

  _proto.havingBaseSizing = function havingBaseSizing() {
    var size;

    switch (this.style.id) {
      case 4:
      case 5:
        size = 3; // normalsize in scriptstyle

        break;

      case 6:
      case 7:
        size = 1; // normalsize in scriptscriptstyle

        break;

      default:
        size = 6;
      // normalsize in textstyle or displaystyle
    }

    return this.extend({
      style: this.style.text(),
      size: size
    });
  }
  /**
   * Create a new options object with the given color.
   */
  ;

  _proto.withColor = function withColor(color) {
    return this.extend({
      color: color
    });
  }
  /**
   * Create a new options object with "phantom" set to true.
   */
  ;

  _proto.withPhantom = function withPhantom() {
    return this.extend({
      phantom: true
    });
  }
  /**
   * Creates a new options object with the given math font or old text font.
   * @type {[type]}
   */
  ;

  _proto.withFont = function withFont(font) {
    return this.extend({
      font: font
    });
  }
  /**
   * Create a new options objects with the given fontFamily.
   */
  ;

  _proto.withTextFontFamily = function withTextFontFamily(fontFamily) {
    return this.extend({
      fontFamily: fontFamily,
      font: ""
    });
  }
  /**
   * Creates a new options object with the given font weight
   */
  ;

  _proto.withTextFontWeight = function withTextFontWeight(fontWeight) {
    return this.extend({
      fontWeight: fontWeight,
      font: ""
    });
  }
  /**
   * Creates a new options object with the given font weight
   */
  ;

  _proto.withTextFontShape = function withTextFontShape(fontShape) {
    return this.extend({
      fontShape: fontShape,
      font: ""
    });
  }
  /**
   * Return the CSS sizing classes required to switch from enclosing options
   * `oldOptions` to `this`. Returns an array of classes.
   */
  ;

  _proto.sizingClasses = function sizingClasses(oldOptions) {
    if (oldOptions.size !== this.size) {
      return ["sizing", "reset-size" + oldOptions.size, "size" + this.size];
    } else {
      return [];
    }
  }
  /**
   * Return the CSS sizing classes required to switch to the base size. Like
   * `this.havingSize(BASESIZE).sizingClasses(this)`.
   */
  ;

  _proto.baseSizingClasses = function baseSizingClasses() {
    if (this.size !== Options.BASESIZE) {
      return ["sizing", "reset-size" + this.size, "size" + Options.BASESIZE];
    } else {
      return [];
    }
  }
  /**
   * Return the font metrics for this size.
   */
  ;

  _proto.fontMetrics = function fontMetrics() {
    if (!this._fontMetrics) {
      this._fontMetrics = getGlobalMetrics(this.size);
    }

    return this._fontMetrics;
  }
  /**
   * Gets the CSS color of the current options object
   */
  ;

  _proto.getColor = function getColor() {
    if (this.phantom) {
      return "transparent";
    } else {
      return this.color;
    }
  };

  return Options;
}();

Options_Options.BASESIZE = 6;
/* harmony default export */ var src_Options = (Options_Options);
// CONCATENATED MODULE: ./src/units.js
/**
 * This file does conversion between units.  In particular, it provides
 * calculateSize to convert other units into ems.
 */

 // This table gives the number of TeX pts in one of each *absolute* TeX unit.
// Thus, multiplying a length by this number converts the length from units
// into pts.  Dividing the result by ptPerEm gives the number of ems
// *assuming* a font size of ptPerEm (normal size, normal style).

var ptPerUnit = {
  // https://en.wikibooks.org/wiki/LaTeX/Lengths and
  // https://tex.stackexchange.com/a/8263
  "pt": 1,
  // TeX point
  "mm": 7227 / 2540,
  // millimeter
  "cm": 7227 / 254,
  // centimeter
  "in": 72.27,
  // inch
  "bp": 803 / 800,
  // big (PostScript) points
  "pc": 12,
  // pica
  "dd": 1238 / 1157,
  // didot
  "cc": 14856 / 1157,
  // cicero (12 didot)
  "nd": 685 / 642,
  // new didot
  "nc": 1370 / 107,
  // new cicero (12 new didot)
  "sp": 1 / 65536,
  // scaled point (TeX's internal smallest unit)
  // https://tex.stackexchange.com/a/41371
  "px": 803 / 800 // \pdfpxdimen defaults to 1 bp in pdfTeX and LuaTeX

}; // Dictionary of relative units, for fast validity testing.

var relativeUnit = {
  "ex": true,
  "em": true,
  "mu": true
};

/**
 * Determine whether the specified unit (either a string defining the unit
 * or a "size" parse node containing a unit field) is valid.
 */
var validUnit = function validUnit(unit) {
  if (typeof unit !== "string") {
    unit = unit.unit;
  }

  return unit in ptPerUnit || unit in relativeUnit || unit === "ex";
};
/*
 * Convert a "size" parse node (with numeric "number" and string "unit" fields,
 * as parsed by functions.js argType "size") into a CSS em value for the
 * current style/scale.  `options` gives the current options.
 */

var units_calculateSize = function calculateSize(sizeValue, options) {
  var scale;

  if (sizeValue.unit in ptPerUnit) {
    // Absolute units
    scale = ptPerUnit[sizeValue.unit] // Convert unit to pt
    / options.fontMetrics().ptPerEm // Convert pt to CSS em
    / options.sizeMultiplier; // Unscale to make absolute units
  } else if (sizeValue.unit === "mu") {
    // `mu` units scale with scriptstyle/scriptscriptstyle.
    scale = options.fontMetrics().cssEmPerMu;
  } else {
    // Other relative units always refer to the *textstyle* font
    // in the current size.
    var unitOptions;

    if (options.style.isTight()) {
      // isTight() means current style is script/scriptscript.
      unitOptions = options.havingStyle(options.style.text());
    } else {
      unitOptions = options;
    } // TODO: In TeX these units are relative to the quad of the current
    // *text* font, e.g. cmr10. KaTeX instead uses values from the
    // comparably-sized *Computer Modern symbol* font. At 10pt, these
    // match. At 7pt and 5pt, they differ: cmr7=1.138894, cmsy7=1.170641;
    // cmr5=1.361133, cmsy5=1.472241. Consider $\scriptsize a\kern1emb$.
    // TeX \showlists shows a kern of 1.13889 * fontsize;
    // KaTeX shows a kern of 1.171 * fontsize.


    if (sizeValue.unit === "ex") {
      scale = unitOptions.fontMetrics().xHeight;
    } else if (sizeValue.unit === "em") {
      scale = unitOptions.fontMetrics().quad;
    } else {
      throw new src_ParseError("Invalid unit: '" + sizeValue.unit + "'");
    }

    if (unitOptions !== options) {
      scale *= unitOptions.sizeMultiplier / options.sizeMultiplier;
    }
  }

  return Math.min(sizeValue.number * scale, options.maxSize);
};
// CONCATENATED MODULE: ./src/buildCommon.js
/* eslint no-console:0 */

/**
 * This module contains general functions that can be used for building
 * different kinds of domTree nodes in a consistent manner.
 */







// The following have to be loaded from Main-Italic font, using class mathit
var mathitLetters = ["\\imath", "ı", // dotless i
"\\jmath", "ȷ", // dotless j
"\\pounds", "\\mathsterling", "\\textsterling", "£"];
/**
 * Looks up the given symbol in fontMetrics, after applying any symbol
 * replacements defined in symbol.js
 */

var buildCommon_lookupSymbol = function lookupSymbol(value, // TODO(#963): Use a union type for this.
fontName, mode) {
  // Replace the value with its replaced value from symbol.js
  if (src_symbols[mode][value] && src_symbols[mode][value].replace) {
    value = src_symbols[mode][value].replace;
  }

  return {
    value: value,
    metrics: getCharacterMetrics(value, fontName, mode)
  };
};
/**
 * Makes a symbolNode after translation via the list of symbols in symbols.js.
 * Correctly pulls out metrics for the character, and optionally takes a list of
 * classes to be attached to the node.
 *
 * TODO: make argument order closer to makeSpan
 * TODO: add a separate argument for math class (e.g. `mop`, `mbin`), which
 * should if present come first in `classes`.
 * TODO(#953): Make `options` mandatory and always pass it in.
 */


var buildCommon_makeSymbol = function makeSymbol(value, fontName, mode, options, classes) {
  var lookup = buildCommon_lookupSymbol(value, fontName, mode);
  var metrics = lookup.metrics;
  value = lookup.value;
  var symbolNode;

  if (metrics) {
    var italic = metrics.italic;

    if (mode === "text" || options && options.font === "mathit") {
      italic = 0;
    }

    symbolNode = new domTree_SymbolNode(value, metrics.height, metrics.depth, italic, metrics.skew, metrics.width, classes);
  } else {
    // TODO(emily): Figure out a good way to only print this in development
    typeof console !== "undefined" && console.warn("No character metrics for '" + value + "' in style '" + fontName + "'");
    symbolNode = new domTree_SymbolNode(value, 0, 0, 0, 0, 0, classes);
  }

  if (options) {
    symbolNode.maxFontSize = options.sizeMultiplier;

    if (options.style.isTight()) {
      symbolNode.classes.push("mtight");
    }

    var color = options.getColor();

    if (color) {
      symbolNode.style.color = color;
    }
  }

  return symbolNode;
};
/**
 * Makes a symbol in Main-Regular or AMS-Regular.
 * Used for rel, bin, open, close, inner, and punct.
 *
 * TODO(#953): Make `options` mandatory and always pass it in.
 */


var buildCommon_mathsym = function mathsym(value, mode, options, classes) {
  if (classes === void 0) {
    classes = [];
  }

  // Decide what font to render the symbol in by its entry in the symbols
  // table.
  // Have a special case for when the value = \ because the \ is used as a
  // textord in unsupported command errors but cannot be parsed as a regular
  // text ordinal and is therefore not present as a symbol in the symbols
  // table for text, as well as a special case for boldsymbol because it
  // can be used for bold + and -
  if (options && options.font && options.font === "boldsymbol" && buildCommon_lookupSymbol(value, "Main-Bold", mode).metrics) {
    return buildCommon_makeSymbol(value, "Main-Bold", mode, options, classes.concat(["mathbf"]));
  } else if (value === "\\" || src_symbols[mode][value].font === "main") {
    return buildCommon_makeSymbol(value, "Main-Regular", mode, options, classes);
  } else {
    return buildCommon_makeSymbol(value, "AMS-Regular", mode, options, classes.concat(["amsrm"]));
  }
};
/**
 * Determines which of the two font names (Main-Italic and Math-Italic) and
 * corresponding style tags (maindefault or mathit) to use for default math font,
 * depending on the symbol.
 */


var buildCommon_mathdefault = function mathdefault(value, mode, options, classes) {
  if (/[0-9]/.test(value.charAt(0)) || // glyphs for \imath and \jmath do not exist in Math-Italic so we
  // need to use Main-Italic instead
  utils.contains(mathitLetters, value)) {
    return {
      fontName: "Main-Italic",
      fontClass: "mathit"
    };
  } else {
    return {
      fontName: "Math-Italic",
      fontClass: "mathdefault"
    };
  }
};
/**
 * Determines which of the font names (Main-Italic, Math-Italic, and Caligraphic)
 * and corresponding style tags (mathit, mathdefault, or mathcal) to use for font
 * "mathnormal", depending on the symbol.  Use this function instead of fontMap for
 * font "mathnormal".
 */


var buildCommon_mathnormal = function mathnormal(value, mode, options, classes) {
  if (utils.contains(mathitLetters, value)) {
    return {
      fontName: "Main-Italic",
      fontClass: "mathit"
    };
  } else if (/[0-9]/.test(value.charAt(0))) {
    return {
      fontName: "Caligraphic-Regular",
      fontClass: "mathcal"
    };
  } else {
    return {
      fontName: "Math-Italic",
      fontClass: "mathdefault"
    };
  }
};
/**
 * Determines which of the two font names (Main-Bold and Math-BoldItalic) and
 * corresponding style tags (mathbf or boldsymbol) to use for font "boldsymbol",
 * depending on the symbol.  Use this function instead of fontMap for font
 * "boldsymbol".
 */


var boldsymbol = function boldsymbol(value, mode, options, classes) {
  if (buildCommon_lookupSymbol(value, "Math-BoldItalic", mode).metrics) {
    return {
      fontName: "Math-BoldItalic",
      fontClass: "boldsymbol"
    };
  } else {
    // Some glyphs do not exist in Math-BoldItalic so we need to use
    // Main-Bold instead.
    return {
      fontName: "Main-Bold",
      fontClass: "mathbf"
    };
  }
};
/**
 * Makes either a mathord or textord in the correct font and color.
 */


var buildCommon_makeOrd = function makeOrd(group, options, type) {
  var mode = group.mode;
  var text = group.text;
  var classes = ["mord"]; // Math mode or Old font (i.e. \rm)

  var isFont = mode === "math" || mode === "text" && options.font;
  var fontOrFamily = isFont ? options.font : options.fontFamily;

  if (text.charCodeAt(0) === 0xD835) {
    // surrogate pairs get special treatment
    var _wideCharacterFont = wide_character_wideCharacterFont(text, mode),
        wideFontName = _wideCharacterFont[0],
        wideFontClass = _wideCharacterFont[1];

    return buildCommon_makeSymbol(text, wideFontName, mode, options, classes.concat(wideFontClass));
  } else if (fontOrFamily) {
    var fontName;
    var fontClasses;

    if (fontOrFamily === "boldsymbol" || fontOrFamily === "mathnormal") {
      var fontData = fontOrFamily === "boldsymbol" ? boldsymbol(text, mode, options, classes) : buildCommon_mathnormal(text, mode, options, classes);
      fontName = fontData.fontName;
      fontClasses = [fontData.fontClass];
    } else if (utils.contains(mathitLetters, text)) {
      fontName = "Main-Italic";
      fontClasses = ["mathit"];
    } else if (isFont) {
      fontName = fontMap[fontOrFamily].fontName;
      fontClasses = [fontOrFamily];
    } else {
      fontName = retrieveTextFontName(fontOrFamily, options.fontWeight, options.fontShape);
      fontClasses = [fontOrFamily, options.fontWeight, options.fontShape];
    }

    if (buildCommon_lookupSymbol(text, fontName, mode).metrics) {
      return buildCommon_makeSymbol(text, fontName, mode, options, classes.concat(fontClasses));
    } else if (ligatures.hasOwnProperty(text) && fontName.substr(0, 10) === "Typewriter") {
      // Deconstruct ligatures in monospace fonts (\texttt, \tt).
      var parts = [];

      for (var i = 0; i < text.length; i++) {
        parts.push(buildCommon_makeSymbol(text[i], fontName, mode, options, classes.concat(fontClasses)));
      }

      return buildCommon_makeFragment(parts);
    }
  } // Makes a symbol in the default font for mathords and textords.


  if (type === "mathord") {
    var fontLookup = buildCommon_mathdefault(text, mode, options, classes);
    return buildCommon_makeSymbol(text, fontLookup.fontName, mode, options, classes.concat([fontLookup.fontClass]));
  } else if (type === "textord") {
    var font = src_symbols[mode][text] && src_symbols[mode][text].font;

    if (font === "ams") {
      var _fontName = retrieveTextFontName("amsrm", options.fontWeight, options.fontShape);

      return buildCommon_makeSymbol(text, _fontName, mode, options, classes.concat("amsrm", options.fontWeight, options.fontShape));
    } else if (font === "main" || !font) {
      var _fontName2 = retrieveTextFontName("textrm", options.fontWeight, options.fontShape);

      return buildCommon_makeSymbol(text, _fontName2, mode, options, classes.concat(options.fontWeight, options.fontShape));
    } else {
      // fonts added by plugins
      var _fontName3 = retrieveTextFontName(font, options.fontWeight, options.fontShape); // We add font name as a css class


      return buildCommon_makeSymbol(text, _fontName3, mode, options, classes.concat(_fontName3, options.fontWeight, options.fontShape));
    }
  } else {
    throw new Error("unexpected type: " + type + " in makeOrd");
  }
};
/**
 * Returns true if subsequent symbolNodes have the same classes, skew, maxFont,
 * and styles.
 */


var buildCommon_canCombine = function canCombine(prev, next) {
  if (createClass(prev.classes) !== createClass(next.classes) || prev.skew !== next.skew || prev.maxFontSize !== next.maxFontSize) {
    return false;
  }

  for (var style in prev.style) {
    if (prev.style.hasOwnProperty(style) && prev.style[style] !== next.style[style]) {
      return false;
    }
  }

  for (var _style in next.style) {
    if (next.style.hasOwnProperty(_style) && prev.style[_style] !== next.style[_style]) {
      return false;
    }
  }

  return true;
};
/**
 * Combine consequetive domTree.symbolNodes into a single symbolNode.
 * Note: this function mutates the argument.
 */


var buildCommon_tryCombineChars = function tryCombineChars(chars) {
  for (var i = 0; i < chars.length - 1; i++) {
    var prev = chars[i];
    var next = chars[i + 1];

    if (prev instanceof domTree_SymbolNode && next instanceof domTree_SymbolNode && buildCommon_canCombine(prev, next)) {
      prev.text += next.text;
      prev.height = Math.max(prev.height, next.height);
      prev.depth = Math.max(prev.depth, next.depth); // Use the last character's italic correction since we use
      // it to add padding to the right of the span created from
      // the combined characters.

      prev.italic = next.italic;
      chars.splice(i + 1, 1);
      i--;
    }
  }

  return chars;
};
/**
 * Calculate the height, depth, and maxFontSize of an element based on its
 * children.
 */


var sizeElementFromChildren = function sizeElementFromChildren(elem) {
  var height = 0;
  var depth = 0;
  var maxFontSize = 0;

  for (var i = 0; i < elem.children.length; i++) {
    var child = elem.children[i];

    if (child.height > height) {
      height = child.height;
    }

    if (child.depth > depth) {
      depth = child.depth;
    }

    if (child.maxFontSize > maxFontSize) {
      maxFontSize = child.maxFontSize;
    }
  }

  elem.height = height;
  elem.depth = depth;
  elem.maxFontSize = maxFontSize;
};
/**
 * Makes a span with the given list of classes, list of children, and options.
 *
 * TODO(#953): Ensure that `options` is always provided (currently some call
 * sites don't pass it) and make the type below mandatory.
 * TODO: add a separate argument for math class (e.g. `mop`, `mbin`), which
 * should if present come first in `classes`.
 */


var buildCommon_makeSpan = function makeSpan(classes, children, options, style) {
  var span = new domTree_Span(classes, children, options, style);
  sizeElementFromChildren(span);
  return span;
}; // SVG one is simpler -- doesn't require height, depth, max-font setting.
// This is also a separate method for typesafety.


var buildCommon_makeSvgSpan = function makeSvgSpan(classes, children, options, style) {
  return new domTree_Span(classes, children, options, style);
};

var makeLineSpan = function makeLineSpan(className, options, thickness) {
  var line = buildCommon_makeSpan([className], [], options);
  line.height = thickness || options.fontMetrics().defaultRuleThickness;
  line.style.borderBottomWidth = line.height + "em";
  line.maxFontSize = 1.0;
  return line;
};
/**
 * Makes an anchor with the given href, list of classes, list of children,
 * and options.
 */


var buildCommon_makeAnchor = function makeAnchor(href, classes, children, options) {
  var anchor = new domTree_Anchor(href, classes, children, options);
  sizeElementFromChildren(anchor);
  return anchor;
};
/**
 * Makes a document fragment with the given list of children.
 */


var buildCommon_makeFragment = function makeFragment(children) {
  var fragment = new tree_DocumentFragment(children);
  sizeElementFromChildren(fragment);
  return fragment;
};
/**
 * Wraps group in a span if it's a document fragment, allowing to apply classes
 * and styles
 */


var buildCommon_wrapFragment = function wrapFragment(group, options) {
  if (group instanceof tree_DocumentFragment) {
    return buildCommon_makeSpan([], [group], options);
  }

  return group;
}; // These are exact object types to catch typos in the names of the optional fields.


// Computes the updated `children` list and the overall depth.
//
// This helper function for makeVList makes it easier to enforce type safety by
// allowing early exits (returns) in the logic.
var getVListChildrenAndDepth = function getVListChildrenAndDepth(params) {
  if (params.positionType === "individualShift") {
    var oldChildren = params.children;
    var children = [oldChildren[0]]; // Add in kerns to the list of params.children to get each element to be
    // shifted to the correct specified shift

    var _depth = -oldChildren[0].shift - oldChildren[0].elem.depth;

    var currPos = _depth;

    for (var i = 1; i < oldChildren.length; i++) {
      var diff = -oldChildren[i].shift - currPos - oldChildren[i].elem.depth;
      var size = diff - (oldChildren[i - 1].elem.height + oldChildren[i - 1].elem.depth);
      currPos = currPos + diff;
      children.push({
        type: "kern",
        size: size
      });
      children.push(oldChildren[i]);
    }

    return {
      children: children,
      depth: _depth
    };
  }

  var depth;

  if (params.positionType === "top") {
    // We always start at the bottom, so calculate the bottom by adding up
    // all the sizes
    var bottom = params.positionData;

    for (var _i = 0; _i < params.children.length; _i++) {
      var child = params.children[_i];
      bottom -= child.type === "kern" ? child.size : child.elem.height + child.elem.depth;
    }

    depth = bottom;
  } else if (params.positionType === "bottom") {
    depth = -params.positionData;
  } else {
    var firstChild = params.children[0];

    if (firstChild.type !== "elem") {
      throw new Error('First child must have type "elem".');
    }

    if (params.positionType === "shift") {
      depth = -firstChild.elem.depth - params.positionData;
    } else if (params.positionType === "firstBaseline") {
      depth = -firstChild.elem.depth;
    } else {
      throw new Error("Invalid positionType " + params.positionType + ".");
    }
  }

  return {
    children: params.children,
    depth: depth
  };
};
/**
 * Makes a vertical list by stacking elements and kerns on top of each other.
 * Allows for many different ways of specifying the positioning method.
 *
 * See VListParam documentation above.
 */


var buildCommon_makeVList = function makeVList(params, options) {
  var _getVListChildrenAndD = getVListChildrenAndDepth(params),
      children = _getVListChildrenAndD.children,
      depth = _getVListChildrenAndD.depth; // Create a strut that is taller than any list item. The strut is added to
  // each item, where it will determine the item's baseline. Since it has
  // `overflow:hidden`, the strut's top edge will sit on the item's line box's
  // top edge and the strut's bottom edge will sit on the item's baseline,
  // with no additional line-height spacing. This allows the item baseline to
  // be positioned precisely without worrying about font ascent and
  // line-height.


  var pstrutSize = 0;

  for (var i = 0; i < children.length; i++) {
    var child = children[i];

    if (child.type === "elem") {
      var elem = child.elem;
      pstrutSize = Math.max(pstrutSize, elem.maxFontSize, elem.height);
    }
  }

  pstrutSize += 2;
  var pstrut = buildCommon_makeSpan(["pstrut"], []);
  pstrut.style.height = pstrutSize + "em"; // Create a new list of actual children at the correct offsets

  var realChildren = [];
  var minPos = depth;
  var maxPos = depth;
  var currPos = depth;

  for (var _i2 = 0; _i2 < children.length; _i2++) {
    var _child = children[_i2];

    if (_child.type === "kern") {
      currPos += _child.size;
    } else {
      var _elem = _child.elem;
      var classes = _child.wrapperClasses || [];
      var style = _child.wrapperStyle || {};
      var childWrap = buildCommon_makeSpan(classes, [pstrut, _elem], undefined, style);
      childWrap.style.top = -pstrutSize - currPos - _elem.depth + "em";

      if (_child.marginLeft) {
        childWrap.style.marginLeft = _child.marginLeft;
      }

      if (_child.marginRight) {
        childWrap.style.marginRight = _child.marginRight;
      }

      realChildren.push(childWrap);
      currPos += _elem.height + _elem.depth;
    }

    minPos = Math.min(minPos, currPos);
    maxPos = Math.max(maxPos, currPos);
  } // The vlist contents go in a table-cell with `vertical-align:bottom`.
  // This cell's bottom edge will determine the containing table's baseline
  // without overly expanding the containing line-box.


  var vlist = buildCommon_makeSpan(["vlist"], realChildren);
  vlist.style.height = maxPos + "em"; // A second row is used if necessary to represent the vlist's depth.

  var rows;

  if (minPos < 0) {
    // We will define depth in an empty span with display: table-cell.
    // It should render with the height that we define. But Chrome, in
    // contenteditable mode only, treats that span as if it contains some
    // text content. And that min-height over-rides our desired height.
    // So we put another empty span inside the depth strut span.
    var emptySpan = buildCommon_makeSpan([], []);
    var depthStrut = buildCommon_makeSpan(["vlist"], [emptySpan]);
    depthStrut.style.height = -minPos + "em"; // Safari wants the first row to have inline content; otherwise it
    // puts the bottom of the *second* row on the baseline.

    var topStrut = buildCommon_makeSpan(["vlist-s"], [new domTree_SymbolNode("\u200B")]);
    rows = [buildCommon_makeSpan(["vlist-r"], [vlist, topStrut]), buildCommon_makeSpan(["vlist-r"], [depthStrut])];
  } else {
    rows = [buildCommon_makeSpan(["vlist-r"], [vlist])];
  }

  var vtable = buildCommon_makeSpan(["vlist-t"], rows);

  if (rows.length === 2) {
    vtable.classes.push("vlist-t2");
  }

  vtable.height = maxPos;
  vtable.depth = -minPos;
  return vtable;
}; // Glue is a concept from TeX which is a flexible space between elements in
// either a vertical or horizontal list. In KaTeX, at least for now, it's
// static space between elements in a horizontal layout.


var buildCommon_makeGlue = function makeGlue(measurement, options) {
  // Make an empty span for the space
  var rule = buildCommon_makeSpan(["mspace"], [], options);
  var size = units_calculateSize(measurement, options);
  rule.style.marginRight = size + "em";
  return rule;
}; // Takes font options, and returns the appropriate fontLookup name


var retrieveTextFontName = function retrieveTextFontName(fontFamily, fontWeight, fontShape) {
  var baseFontName = "";

  switch (fontFamily) {
    case "amsrm":
      baseFontName = "AMS";
      break;

    case "textrm":
      baseFontName = "Main";
      break;

    case "textsf":
      baseFontName = "SansSerif";
      break;

    case "texttt":
      baseFontName = "Typewriter";
      break;

    default:
      baseFontName = fontFamily;
    // use fonts added by a plugin
  }

  var fontStylesName;

  if (fontWeight === "textbf" && fontShape === "textit") {
    fontStylesName = "BoldItalic";
  } else if (fontWeight === "textbf") {
    fontStylesName = "Bold";
  } else if (fontWeight === "textit") {
    fontStylesName = "Italic";
  } else {
    fontStylesName = "Regular";
  }

  return baseFontName + "-" + fontStylesName;
};
/**
 * Maps TeX font commands to objects containing:
 * - variant: string used for "mathvariant" attribute in buildMathML.js
 * - fontName: the "style" parameter to fontMetrics.getCharacterMetrics
 */
// A map between tex font commands an MathML mathvariant attribute values


var fontMap = {
  // styles
  "mathbf": {
    variant: "bold",
    fontName: "Main-Bold"
  },
  "mathrm": {
    variant: "normal",
    fontName: "Main-Regular"
  },
  "textit": {
    variant: "italic",
    fontName: "Main-Italic"
  },
  "mathit": {
    variant: "italic",
    fontName: "Main-Italic"
  },
  // Default math font, "mathnormal" and "boldsymbol" are missing because they
  // require the use of several fonts: Main-Italic and Math-Italic for default
  // math font, Main-Italic, Math-Italic, Caligraphic for "mathnormal", and
  // Math-BoldItalic and Main-Bold for "boldsymbol".  This is handled by a
  // special case in makeOrd which ends up calling mathdefault, mathnormal,
  // and boldsymbol.
  // families
  "mathbb": {
    variant: "double-struck",
    fontName: "AMS-Regular"
  },
  "mathcal": {
    variant: "script",
    fontName: "Caligraphic-Regular"
  },
  "mathfrak": {
    variant: "fraktur",
    fontName: "Fraktur-Regular"
  },
  "mathscr": {
    variant: "script",
    fontName: "Script-Regular"
  },
  "mathsf": {
    variant: "sans-serif",
    fontName: "SansSerif-Regular"
  },
  "mathtt": {
    variant: "monospace",
    fontName: "Typewriter-Regular"
  }
};
var svgData = {
  //   path, width, height
  vec: ["vec", 0.471, 0.714],
  // values from the font glyph
  oiintSize1: ["oiintSize1", 0.957, 0.499],
  // oval to overlay the integrand
  oiintSize2: ["oiintSize2", 1.472, 0.659],
  oiiintSize1: ["oiiintSize1", 1.304, 0.499],
  oiiintSize2: ["oiiintSize2", 1.98, 0.659]
};

var buildCommon_staticSvg = function staticSvg(value, options) {
  // Create a span with inline SVG for the element.
  var _svgData$value = svgData[value],
      pathName = _svgData$value[0],
      width = _svgData$value[1],
      height = _svgData$value[2];
  var path = new domTree_PathNode(pathName);
  var svgNode = new SvgNode([path], {
    "width": width + "em",
    "height": height + "em",
    // Override CSS rule `.katex svg { width: 100% }`
    "style": "width:" + width + "em",
    "viewBox": "0 0 " + 1000 * width + " " + 1000 * height,
    "preserveAspectRatio": "xMinYMin"
  });
  var span = buildCommon_makeSvgSpan(["overlay"], [svgNode], options);
  span.height = height;
  span.style.height = height + "em";
  span.style.width = width + "em";
  return span;
};

/* harmony default export */ var buildCommon = ({
  fontMap: fontMap,
  makeSymbol: buildCommon_makeSymbol,
  mathsym: buildCommon_mathsym,
  makeSpan: buildCommon_makeSpan,
  makeSvgSpan: buildCommon_makeSvgSpan,
  makeLineSpan: makeLineSpan,
  makeAnchor: buildCommon_makeAnchor,
  makeFragment: buildCommon_makeFragment,
  wrapFragment: buildCommon_wrapFragment,
  makeVList: buildCommon_makeVList,
  makeOrd: buildCommon_makeOrd,
  makeGlue: buildCommon_makeGlue,
  staticSvg: buildCommon_staticSvg,
  svgData: svgData,
  tryCombineChars: buildCommon_tryCombineChars
});
// CONCATENATED MODULE: ./src/parseNode.js


/**
 * Asserts that the node is of the given type and returns it with stricter
 * typing. Throws if the node's type does not match.
 */
function assertNodeType(node, type) {
  var typedNode = checkNodeType(node, type);

  if (!typedNode) {
    throw new Error("Expected node of type " + type + ", but got " + (node ? "node of type " + node.type : String(node)));
  } // $FlowFixMe: Unsure why.


  return typedNode;
}
/**
 * Returns the node more strictly typed iff it is of the given type. Otherwise,
 * returns null.
 */

function checkNodeType(node, type) {
  if (node && node.type === type) {
    // The definition of ParseNode<TYPE> doesn't communicate to flow that
    // `type: TYPE` (as that's not explicitly mentioned anywhere), though that
    // happens to be true for all our value types.
    // $FlowFixMe
    return node;
  }

  return null;
}
/**
 * Asserts that the node is of the given type and returns it with stricter
 * typing. Throws if the node's type does not match.
 */

function assertAtomFamily(node, family) {
  var typedNode = checkAtomFamily(node, family);

  if (!typedNode) {
    throw new Error("Expected node of type \"atom\" and family \"" + family + "\", but got " + (node ? node.type === "atom" ? "atom of family " + node.family : "node of type " + node.type : String(node)));
  }

  return typedNode;
}
/**
 * Returns the node more strictly typed iff it is of the given type. Otherwise,
 * returns null.
 */

function checkAtomFamily(node, family) {
  return node && node.type === "atom" && node.family === family ? node : null;
}
/**
 * Returns the node more strictly typed iff it is of the given type. Otherwise,
 * returns null.
 */

function assertSymbolNodeType(node) {
  var typedNode = checkSymbolNodeType(node);

  if (!typedNode) {
    throw new Error("Expected node of symbol group type, but got " + (node ? "node of type " + node.type : String(node)));
  }

  return typedNode;
}
/**
 * Returns the node more strictly typed iff it is of the given type. Otherwise,
 * returns null.
 */

function checkSymbolNodeType(node) {
  if (node && (node.type === "atom" || NON_ATOMS.hasOwnProperty(node.type))) {
    // $FlowFixMe
    return node;
  }

  return null;
}
// CONCATENATED MODULE: ./src/spacingData.js
/**
 * Describes spaces between different classes of atoms.
 */
var thinspace = {
  number: 3,
  unit: "mu"
};
var mediumspace = {
  number: 4,
  unit: "mu"
};
var thickspace = {
  number: 5,
  unit: "mu"
}; // Making the type below exact with all optional fields doesn't work due to
// - https://github.com/facebook/flow/issues/4582
// - https://github.com/facebook/flow/issues/5688
// However, since *all* fields are optional, $Shape<> works as suggested in 5688
// above.

// Spacing relationships for display and text styles
var spacings = {
  mord: {
    mop: thinspace,
    mbin: mediumspace,
    mrel: thickspace,
    minner: thinspace
  },
  mop: {
    mord: thinspace,
    mop: thinspace,
    mrel: thickspace,
    minner: thinspace
  },
  mbin: {
    mord: mediumspace,
    mop: mediumspace,
    mopen: mediumspace,
    minner: mediumspace
  },
  mrel: {
    mord: thickspace,
    mop: thickspace,
    mopen: thickspace,
    minner: thickspace
  },
  mopen: {},
  mclose: {
    mop: thinspace,
    mbin: mediumspace,
    mrel: thickspace,
    minner: thinspace
  },
  mpunct: {
    mord: thinspace,
    mop: thinspace,
    mrel: thickspace,
    mopen: thinspace,
    mclose: thinspace,
    mpunct: thinspace,
    minner: thinspace
  },
  minner: {
    mord: thinspace,
    mop: thinspace,
    mbin: mediumspace,
    mrel: thickspace,
    mopen: thinspace,
    mpunct: thinspace,
    minner: thinspace
  }
}; // Spacing relationships for script and scriptscript styles

var tightSpacings = {
  mord: {
    mop: thinspace
  },
  mop: {
    mord: thinspace,
    mop: thinspace
  },
  mbin: {},
  mrel: {},
  mopen: {},
  mclose: {
    mop: thinspace
  },
  mpunct: {},
  minner: {
    mop: thinspace
  }
};
// CONCATENATED MODULE: ./src/defineFunction.js


/**
 * All registered functions.
 * `functions.js` just exports this same dictionary again and makes it public.
 * `Parser.js` requires this dictionary.
 */
var _functions = {};
/**
 * All HTML builders. Should be only used in the `define*` and the `build*ML`
 * functions.
 */

var _htmlGroupBuilders = {};
/**
 * All MathML builders. Should be only used in the `define*` and the `build*ML`
 * functions.
 */

var _mathmlGroupBuilders = {};
function defineFunction(_ref) {
  var type = _ref.type,
      nodeType = _ref.nodeType,
      names = _ref.names,
      props = _ref.props,
      handler = _ref.handler,
      htmlBuilder = _ref.htmlBuilder,
      mathmlBuilder = _ref.mathmlBuilder;
  // Set default values of functions
  var data = {
    type: type,
    numArgs: props.numArgs,
    argTypes: props.argTypes,
    greediness: props.greediness === undefined ? 1 : props.greediness,
    allowedInText: !!props.allowedInText,
    allowedInMath: props.allowedInMath === undefined ? true : props.allowedInMath,
    numOptionalArgs: props.numOptionalArgs || 0,
    infix: !!props.infix,
    consumeMode: props.consumeMode,
    handler: handler
  };

  for (var i = 0; i < names.length; ++i) {
    // TODO: The value type of _functions should be a type union of all
    // possible `FunctionSpec<>` possibilities instead of `FunctionSpec<*>`,
    // which is an existential type.
    // $FlowFixMe
    _functions[names[i]] = data;
  }

  if (type) {
    if (htmlBuilder) {
      _htmlGroupBuilders[type] = htmlBuilder;
    }

    if (mathmlBuilder) {
      _mathmlGroupBuilders[type] = mathmlBuilder;
    }
  }
}
/**
 * Use this to register only the HTML and MathML builders for a function (e.g.
 * if the function's ParseNode is generated in Parser.js rather than via a
 * stand-alone handler provided to `defineFunction`).
 */

function defineFunctionBuilders(_ref2) {
  var type = _ref2.type,
      htmlBuilder = _ref2.htmlBuilder,
      mathmlBuilder = _ref2.mathmlBuilder;
  defineFunction({
    type: type,
    names: [],
    props: {
      numArgs: 0
    },
    handler: function handler() {
      throw new Error('Should never be called.');
    },
    htmlBuilder: htmlBuilder,
    mathmlBuilder: mathmlBuilder
  });
} // Since the corresponding buildHTML/buildMathML function expects a
// list of elements, we normalize for different kinds of arguments

var defineFunction_ordargument = function ordargument(arg) {
  var node = checkNodeType(arg, "ordgroup");
  return node ? node.body : [arg];
};
// CONCATENATED MODULE: ./src/buildHTML.js
/**
 * This file does the main work of building a domTree structure from a parse
 * tree. The entry point is the `buildHTML` function, which takes a parse tree.
 * Then, the buildExpression, buildGroup, and various groupBuilders functions
 * are called, to produce a final HTML tree.
 */









var buildHTML_makeSpan = buildCommon.makeSpan; // Binary atoms (first class `mbin`) change into ordinary atoms (`mord`)
// depending on their surroundings. See TeXbook pg. 442-446, Rules 5 and 6,
// and the text before Rule 19.

var binLeftCanceller = ["leftmost", "mbin", "mopen", "mrel", "mop", "mpunct"];
var binRightCanceller = ["rightmost", "mrel", "mclose", "mpunct"];
var buildHTML_styleMap = {
  "display": src_Style.DISPLAY,
  "text": src_Style.TEXT,
  "script": src_Style.SCRIPT,
  "scriptscript": src_Style.SCRIPTSCRIPT
};
var DomEnum = {
  mord: "mord",
  mop: "mop",
  mbin: "mbin",
  mrel: "mrel",
  mopen: "mopen",
  mclose: "mclose",
  mpunct: "mpunct",
  minner: "minner"
};

/**
 * Take a list of nodes, build them in order, and return a list of the built
 * nodes. documentFragments are flattened into their contents, so the
 * returned list contains no fragments. `isRealGroup` is true if `expression`
 * is a real group (no atoms will be added on either side), as opposed to
 * a partial group (e.g. one created by \color). `surrounding` is an array
 * consisting type of nodes that will be added to the left and right.
 */
var buildHTML_buildExpression = function buildExpression(expression, options, isRealGroup, surrounding) {
  if (surrounding === void 0) {
    surrounding = [null, null];
  }

  // Parse expressions into `groups`.
  var groups = [];

  for (var i = 0; i < expression.length; i++) {
    var output = buildHTML_buildGroup(expression[i], options);

    if (output instanceof tree_DocumentFragment) {
      var children = output.children;
      groups.push.apply(groups, children);
    } else {
      groups.push(output);
    }
  } // If `expression` is a partial group, let the parent handle spacings
  // to avoid processing groups multiple times.


  if (!isRealGroup) {
    return groups;
  }

  var glueOptions = options;

  if (expression.length === 1) {
    var node = checkNodeType(expression[0], "sizing") || checkNodeType(expression[0], "styling");

    if (!node) {// No match.
    } else if (node.type === "sizing") {
      glueOptions = options.havingSize(node.size);
    } else if (node.type === "styling") {
      glueOptions = options.havingStyle(buildHTML_styleMap[node.style]);
    }
  } // Dummy spans for determining spacings between surrounding atoms.
  // If `expression` has no atoms on the left or right, class "leftmost"
  // or "rightmost", respectively, is used to indicate it.


  var dummyPrev = buildHTML_makeSpan([surrounding[0] || "leftmost"], [], options);
  var dummyNext = buildHTML_makeSpan([surrounding[1] || "rightmost"], [], options); // TODO: These code assumes that a node's math class is the first element
  // of its `classes` array. A later cleanup should ensure this, for
  // instance by changing the signature of `makeSpan`.
  // Before determining what spaces to insert, perform bin cancellation.
  // Binary operators change to ordinary symbols in some contexts.

  traverseNonSpaceNodes(groups, function (node, prev) {
    var prevType = prev.classes[0];
    var type = node.classes[0];

    if (prevType === "mbin" && utils.contains(binRightCanceller, type)) {
      prev.classes[0] = "mord";
    } else if (type === "mbin" && utils.contains(binLeftCanceller, prevType)) {
      node.classes[0] = "mord";
    }
  }, {
    node: dummyPrev
  }, dummyNext);
  traverseNonSpaceNodes(groups, function (node, prev) {
    var prevType = getTypeOfDomTree(prev);
    var type = getTypeOfDomTree(node); // 'mtight' indicates that the node is script or scriptscript style.

    var space = prevType && type ? node.hasClass("mtight") ? tightSpacings[prevType][type] : spacings[prevType][type] : null;

    if (space) {
      // Insert glue (spacing) after the `prev`.
      return buildCommon.makeGlue(space, glueOptions);
    }
  }, {
    node: dummyPrev
  }, dummyNext);
  return groups;
}; // Depth-first traverse non-space `nodes`, calling `callback` with the current and
// previous node as arguments, optionally returning a node to insert after the
// previous node. `prev` is an object with the previous node and `insertAfter`
// function to insert after it. `next` is a node that will be added to the right.
// Used for bin cancellation and inserting spacings.

var traverseNonSpaceNodes = function traverseNonSpaceNodes(nodes, callback, prev, next) {
  if (next) {
    // temporarily append the right node, if exists
    nodes.push(next);
  }

  var i = 0;

  for (; i < nodes.length; i++) {
    var node = nodes[i];
    var partialGroup = buildHTML_checkPartialGroup(node);

    if (partialGroup) {
      // Recursive DFS
      traverseNonSpaceNodes(partialGroup.children, callback, prev);
      continue;
    } // Ignore explicit spaces (e.g., \;, \,) when determining what implicit
    // spacing should go between atoms of different classes


    if (node.classes[0] === "mspace") {
      continue;
    }

    var result = callback(node, prev.node);

    if (result) {
      if (prev.insertAfter) {
        prev.insertAfter(result);
      } else {
        // insert at front
        nodes.unshift(result);
        i++;
      }
    }

    prev.node = node;

    prev.insertAfter = function (index) {
      return function (n) {
        nodes.splice(index + 1, 0, n);
        i++;
      };
    }(i);
  }

  if (next) {
    nodes.pop();
  }
}; // Check if given node is a partial group, i.e., does not affect spacing around.


var buildHTML_checkPartialGroup = function checkPartialGroup(node) {
  if (node instanceof tree_DocumentFragment || node instanceof domTree_Anchor) {
    return node;
  }

  return null;
}; // Return the outermost node of a domTree.


var getOutermostNode = function getOutermostNode(node, side) {
  var partialGroup = buildHTML_checkPartialGroup(node);

  if (partialGroup) {
    var children = partialGroup.children;

    if (children.length) {
      if (side === "right") {
        return getOutermostNode(children[children.length - 1], "right");
      } else if (side === "left") {
        return getOutermostNode(children[0], "left");
      }
    }
  }

  return node;
}; // Return math atom class (mclass) of a domTree.
// If `side` is given, it will get the type of the outermost node at given side.


var getTypeOfDomTree = function getTypeOfDomTree(node, side) {
  if (!node) {
    return null;
  }

  if (side) {
    node = getOutermostNode(node, side);
  } // This makes a lot of assumptions as to where the type of atom
  // appears.  We should do a better job of enforcing this.


  return DomEnum[node.classes[0]] || null;
};
var makeNullDelimiter = function makeNullDelimiter(options, classes) {
  var moreClasses = ["nulldelimiter"].concat(options.baseSizingClasses());
  return buildHTML_makeSpan(classes.concat(moreClasses));
};
/**
 * buildGroup is the function that takes a group and calls the correct groupType
 * function for it. It also handles the interaction of size and style changes
 * between parents and children.
 */

var buildHTML_buildGroup = function buildGroup(group, options, baseOptions) {
  if (!group) {
    return buildHTML_makeSpan();
  }

  if (_htmlGroupBuilders[group.type]) {
    // Call the groupBuilders function
    var groupNode = _htmlGroupBuilders[group.type](group, options); // If the size changed between the parent and the current group, account
    // for that size difference.

    if (baseOptions && options.size !== baseOptions.size) {
      groupNode = buildHTML_makeSpan(options.sizingClasses(baseOptions), [groupNode], options);
      var multiplier = options.sizeMultiplier / baseOptions.sizeMultiplier;
      groupNode.height *= multiplier;
      groupNode.depth *= multiplier;
    }

    return groupNode;
  } else {
    throw new src_ParseError("Got group of unknown type: '" + group.type + "'");
  }
};
/**
 * Combine an array of HTML DOM nodes (e.g., the output of `buildExpression`)
 * into an unbreakable HTML node of class .base, with proper struts to
 * guarantee correct vertical extent.  `buildHTML` calls this repeatedly to
 * make up the entire expression as a sequence of unbreakable units.
 */

function buildHTMLUnbreakable(children, options) {
  // Compute height and depth of this chunk.
  var body = buildHTML_makeSpan(["base"], children, options); // Add strut, which ensures that the top of the HTML element falls at
  // the height of the expression, and the bottom of the HTML element
  // falls at the depth of the expression.
  // We used to have separate top and bottom struts, where the bottom strut
  // would like to use `vertical-align: top`, but in IE 9 this lowers the
  // baseline of the box to the bottom of this strut (instead of staying in
  // the normal place) so we use an absolute value for vertical-align instead.

  var strut = buildHTML_makeSpan(["strut"]);
  strut.style.height = body.height + body.depth + "em";
  strut.style.verticalAlign = -body.depth + "em";
  body.children.unshift(strut);
  return body;
}
/**
 * Take an entire parse tree, and build it into an appropriate set of HTML
 * nodes.
 */


function buildHTML(tree, options) {
  // Strip off outer tag wrapper for processing below.
  var tag = null;

  if (tree.length === 1 && tree[0].type === "tag") {
    tag = tree[0].tag;
    tree = tree[0].body;
  } // Build the expression contained in the tree


  var expression = buildHTML_buildExpression(tree, options, true);
  var children = []; // Create one base node for each chunk between potential line breaks.
  // The TeXBook [p.173] says "A formula will be broken only after a
  // relation symbol like $=$ or $<$ or $\rightarrow$, or after a binary
  // operation symbol like $+$ or $-$ or $\times$, where the relation or
  // binary operation is on the ``outer level'' of the formula (i.e., not
  // enclosed in {...} and not part of an \over construction)."

  var parts = [];

  for (var i = 0; i < expression.length; i++) {
    parts.push(expression[i]);

    if (expression[i].hasClass("mbin") || expression[i].hasClass("mrel") || expression[i].hasClass("allowbreak")) {
      // Put any post-operator glue on same line as operator.
      // Watch for \nobreak along the way, and stop at \newline.
      var nobreak = false;

      while (i < expression.length - 1 && expression[i + 1].hasClass("mspace") && !expression[i + 1].hasClass("newline")) {
        i++;
        parts.push(expression[i]);

        if (expression[i].hasClass("nobreak")) {
          nobreak = true;
        }
      } // Don't allow break if \nobreak among the post-operator glue.


      if (!nobreak) {
        children.push(buildHTMLUnbreakable(parts, options));
        parts = [];
      }
    } else if (expression[i].hasClass("newline")) {
      // Write the line except the newline
      parts.pop();

      if (parts.length > 0) {
        children.push(buildHTMLUnbreakable(parts, options));
        parts = [];
      } // Put the newline at the top level


      children.push(expression[i]);
    }
  }

  if (parts.length > 0) {
    children.push(buildHTMLUnbreakable(parts, options));
  } // Now, if there was a tag, build it too and append it as a final child.


  var tagChild;

  if (tag) {
    tagChild = buildHTMLUnbreakable(buildHTML_buildExpression(tag, options, true));
    tagChild.classes = ["tag"];
    children.push(tagChild);
  }

  var htmlNode = buildHTML_makeSpan(["katex-html"], children);
  htmlNode.setAttribute("aria-hidden", "true"); // Adjust the strut of the tag to be the maximum height of all children
  // (the height of the enclosing htmlNode) for proper vertical alignment.

  if (tagChild) {
    var strut = tagChild.children[0];
    strut.style.height = htmlNode.height + htmlNode.depth + "em";
    strut.style.verticalAlign = -htmlNode.depth + "em";
  }

  return htmlNode;
}
// CONCATENATED MODULE: ./src/mathMLTree.js
/**
 * These objects store data about MathML nodes. This is the MathML equivalent
 * of the types in domTree.js. Since MathML handles its own rendering, and
 * since we're mainly using MathML to improve accessibility, we don't manage
 * any of the styling state that the plain DOM nodes do.
 *
 * The `toNode` and `toMarkup` functions work simlarly to how they do in
 * domTree.js, creating namespaced DOM nodes and HTML text markup respectively.
 */


function newDocumentFragment(children) {
  return new tree_DocumentFragment(children);
}
/**
 * This node represents a general purpose MathML node of any type. The
 * constructor requires the type of node to create (for example, `"mo"` or
 * `"mspace"`, corresponding to `<mo>` and `<mspace>` tags).
 */

var mathMLTree_MathNode =
/*#__PURE__*/
function () {
  function MathNode(type, children) {
    this.type = void 0;
    this.attributes = void 0;
    this.children = void 0;
    this.type = type;
    this.attributes = {};
    this.children = children || [];
  }
  /**
   * Sets an attribute on a MathML node. MathML depends on attributes to convey a
   * semantic content, so this is used heavily.
   */


  var _proto = MathNode.prototype;

  _proto.setAttribute = function setAttribute(name, value) {
    this.attributes[name] = value;
  }
  /**
   * Gets an attribute on a MathML node.
   */
  ;

  _proto.getAttribute = function getAttribute(name) {
    return this.attributes[name];
  }
  /**
   * Converts the math node into a MathML-namespaced DOM element.
   */
  ;

  _proto.toNode = function toNode() {
    var node = document.createElementNS("http://www.w3.org/1998/Math/MathML", this.type);

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        node.setAttribute(attr, this.attributes[attr]);
      }
    }

    for (var i = 0; i < this.children.length; i++) {
      node.appendChild(this.children[i].toNode());
    }

    return node;
  }
  /**
   * Converts the math node into an HTML markup string.
   */
  ;

  _proto.toMarkup = function toMarkup() {
    var markup = "<" + this.type; // Add the attributes

    for (var attr in this.attributes) {
      if (Object.prototype.hasOwnProperty.call(this.attributes, attr)) {
        markup += " " + attr + "=\"";
        markup += utils.escape(this.attributes[attr]);
        markup += "\"";
      }
    }

    markup += ">";

    for (var i = 0; i < this.children.length; i++) {
      markup += this.children[i].toMarkup();
    }

    markup += "</" + this.type + ">";
    return markup;
  }
  /**
   * Converts the math node into a string, similar to innerText, but escaped.
   */
  ;

  _proto.toText = function toText() {
    return this.children.map(function (child) {
      return child.toText();
    }).join("");
  };

  return MathNode;
}();
/**
 * This node represents a piece of text.
 */

var mathMLTree_TextNode =
/*#__PURE__*/
function () {
  function TextNode(text) {
    this.text = void 0;
    this.text = text;
  }
  /**
   * Converts the text node into a DOM text node.
   */


  var _proto2 = TextNode.prototype;

  _proto2.toNode = function toNode() {
    return document.createTextNode(this.text);
  }
  /**
   * Converts the text node into escaped HTML markup
   * (representing the text itself).
   */
  ;

  _proto2.toMarkup = function toMarkup() {
    return utils.escape(this.toText());
  }
  /**
   * Converts the text node into a string
   * (representing the text iteself).
   */
  ;

  _proto2.toText = function toText() {
    return this.text;
  };

  return TextNode;
}();
/**
 * This node represents a space, but may render as <mspace.../> or as text,
 * depending on the width.
 */

var SpaceNode =
/*#__PURE__*/
function () {
  /**
   * Create a Space node with width given in CSS ems.
   */
  function SpaceNode(width) {
    this.width = void 0;
    this.character = void 0;
    this.width = width; // See https://www.w3.org/TR/2000/WD-MathML2-20000328/chapter6.html
    // for a table of space-like characters.  We use Unicode
    // representations instead of &LongNames; as it's not clear how to
    // make the latter via document.createTextNode.

    if (width >= 0.05555 && width <= 0.05556) {
      this.character = "\u200A"; // &VeryThinSpace;
    } else if (width >= 0.1666 && width <= 0.1667) {
      this.character = "\u2009"; // &ThinSpace;
    } else if (width >= 0.2222 && width <= 0.2223) {
      this.character = "\u2005"; // &MediumSpace;
    } else if (width >= 0.2777 && width <= 0.2778) {
      this.character = "\u2005\u200A"; // &ThickSpace;
    } else if (width >= -0.05556 && width <= -0.05555) {
      this.character = "\u200A\u2063"; // &NegativeVeryThinSpace;
    } else if (width >= -0.1667 && width <= -0.1666) {
      this.character = "\u2009\u2063"; // &NegativeThinSpace;
    } else if (width >= -0.2223 && width <= -0.2222) {
      this.character = "\u205F\u2063"; // &NegativeMediumSpace;
    } else if (width >= -0.2778 && width <= -0.2777) {
      this.character = "\u2005\u2063"; // &NegativeThickSpace;
    } else {
      this.character = null;
    }
  }
  /**
   * Converts the math node into a MathML-namespaced DOM element.
   */


  var _proto3 = SpaceNode.prototype;

  _proto3.toNode = function toNode() {
    if (this.character) {
      return document.createTextNode(this.character);
    } else {
      var node = document.createElementNS("http://www.w3.org/1998/Math/MathML", "mspace");
      node.setAttribute("width", this.width + "em");
      return node;
    }
  }
  /**
   * Converts the math node into an HTML markup string.
   */
  ;

  _proto3.toMarkup = function toMarkup() {
    if (this.character) {
      return "<mtext>" + this.character + "</mtext>";
    } else {
      return "<mspace width=\"" + this.width + "em\"/>";
    }
  }
  /**
   * Converts the math node into a string, similar to innerText.
   */
  ;

  _proto3.toText = function toText() {
    if (this.character) {
      return this.character;
    } else {
      return " ";
    }
  };

  return SpaceNode;
}();

/* harmony default export */ var mathMLTree = ({
  MathNode: mathMLTree_MathNode,
  TextNode: mathMLTree_TextNode,
  SpaceNode: SpaceNode,
  newDocumentFragment: newDocumentFragment
});
// CONCATENATED MODULE: ./src/buildMathML.js
/**
 * This file converts a parse tree into a cooresponding MathML tree. The main
 * entry point is the `buildMathML` function, which takes a parse tree from the
 * parser.
 */









/**
 * Takes a symbol and converts it into a MathML text node after performing
 * optional replacement from symbols.js.
 */
var buildMathML_makeText = function makeText(text, mode, options) {
  if (src_symbols[mode][text] && src_symbols[mode][text].replace && text.charCodeAt(0) !== 0xD835 && !(ligatures.hasOwnProperty(text) && options && (options.fontFamily && options.fontFamily.substr(4, 2) === "tt" || options.font && options.font.substr(4, 2) === "tt"))) {
    text = src_symbols[mode][text].replace;
  }

  return new mathMLTree.TextNode(text);
};
/**
 * Wrap the given array of nodes in an <mrow> node if needed, i.e.,
 * unless the array has length 1.  Always returns a single node.
 */

var buildMathML_makeRow = function makeRow(body) {
  if (body.length === 1) {
    return body[0];
  } else {
    return new mathMLTree.MathNode("mrow", body);
  }
};
/**
 * Returns the math variant as a string or null if none is required.
 */

var buildMathML_getVariant = function getVariant(group, options) {
  // Handle \text... font specifiers as best we can.
  // MathML has a limited list of allowable mathvariant specifiers; see
  // https://www.w3.org/TR/MathML3/chapter3.html#presm.commatt
  if (options.fontFamily === "texttt") {
    return "monospace";
  } else if (options.fontFamily === "textsf") {
    if (options.fontShape === "textit" && options.fontWeight === "textbf") {
      return "sans-serif-bold-italic";
    } else if (options.fontShape === "textit") {
      return "sans-serif-italic";
    } else if (options.fontWeight === "textbf") {
      return "bold-sans-serif";
    } else {
      return "sans-serif";
    }
  } else if (options.fontShape === "textit" && options.fontWeight === "textbf") {
    return "bold-italic";
  } else if (options.fontShape === "textit") {
    return "italic";
  } else if (options.fontWeight === "textbf") {
    return "bold";
  }

  var font = options.font;

  if (!font || font === "mathnormal") {
    return null;
  }

  var mode = group.mode;

  if (font === "mathit") {
    return "italic";
  } else if (font === "boldsymbol") {
    return "bold-italic";
  }

  var text = group.text;

  if (utils.contains(["\\imath", "\\jmath"], text)) {
    return null;
  }

  if (src_symbols[mode][text] && src_symbols[mode][text].replace) {
    text = src_symbols[mode][text].replace;
  }

  var fontName = buildCommon.fontMap[font].fontName;

  if (getCharacterMetrics(text, fontName, mode)) {
    return buildCommon.fontMap[font].variant;
  }

  return null;
};
/**
 * Takes a list of nodes, builds them, and returns a list of the generated
 * MathML nodes.  Also combine consecutive <mtext> outputs into a single
 * <mtext> tag.
 */

var buildMathML_buildExpression = function buildExpression(expression, options) {
  var groups = [];
  var lastGroup;

  for (var i = 0; i < expression.length; i++) {
    var group = buildMathML_buildGroup(expression[i], options);

    if (group instanceof mathMLTree_MathNode && lastGroup instanceof mathMLTree_MathNode) {
      // Concatenate adjacent <mtext>s
      if (group.type === 'mtext' && lastGroup.type === 'mtext' && group.getAttribute('mathvariant') === lastGroup.getAttribute('mathvariant')) {
        var _lastGroup$children;

        (_lastGroup$children = lastGroup.children).push.apply(_lastGroup$children, group.children);

        continue; // Concatenate adjacent <mn>s
      } else if (group.type === 'mn' && lastGroup.type === 'mn') {
        var _lastGroup$children2;

        (_lastGroup$children2 = lastGroup.children).push.apply(_lastGroup$children2, group.children);

        continue; // Concatenate <mn>...</mn> followed by <mi>.</mi>
      } else if (group.type === 'mi' && group.children.length === 1 && lastGroup.type === 'mn') {
        var child = group.children[0];

        if (child instanceof mathMLTree_TextNode && child.text === '.') {
          var _lastGroup$children3;

          (_lastGroup$children3 = lastGroup.children).push.apply(_lastGroup$children3, group.children);

          continue;
        }
      } else if (lastGroup.type === 'mi' && lastGroup.children.length === 1) {
        var lastChild = lastGroup.children[0];

        if (lastChild instanceof mathMLTree_TextNode && lastChild.text === "\u0338" && (group.type === 'mo' || group.type === 'mi' || group.type === 'mn')) {
          var _child = group.children[0];

          if (_child instanceof mathMLTree_TextNode && _child.text.length > 0) {
            // Overlay with combining character long solidus
            _child.text = _child.text.slice(0, 1) + "\u0338" + _child.text.slice(1);
            groups.pop();
          }
        }
      }
    }

    groups.push(group);
    lastGroup = group;
  }

  return groups;
};
/**
 * Equivalent to buildExpression, but wraps the elements in an <mrow>
 * if there's more than one.  Returns a single node instead of an array.
 */

var buildExpressionRow = function buildExpressionRow(expression, options) {
  return buildMathML_makeRow(buildMathML_buildExpression(expression, options));
};
/**
 * Takes a group from the parser and calls the appropriate groupBuilders function
 * on it to produce a MathML node.
 */

var buildMathML_buildGroup = function buildGroup(group, options) {
  if (!group) {
    return new mathMLTree.MathNode("mrow");
  }

  if (_mathmlGroupBuilders[group.type]) {
    // Call the groupBuilders function
    var result = _mathmlGroupBuilders[group.type](group, options);
    return result;
  } else {
    throw new src_ParseError("Got group of unknown type: '" + group.type + "'");
  }
};
/**
 * Takes a full parse tree and settings and builds a MathML representation of
 * it. In particular, we put the elements from building the parse tree into a
 * <semantics> tag so we can also include that TeX source as an annotation.
 *
 * Note that we actually return a domTree element with a `<math>` inside it so
 * we can do appropriate styling.
 */

function buildMathML(tree, texExpression, options) {
  var expression = buildMathML_buildExpression(tree, options); // Wrap up the expression in an mrow so it is presented in the semantics
  // tag correctly, unless it's a single <mrow> or <mtable>.

  var wrapper;

  if (expression.length === 1 && expression[0] instanceof mathMLTree_MathNode && utils.contains(["mrow", "mtable"], expression[0].type)) {
    wrapper = expression[0];
  } else {
    wrapper = new mathMLTree.MathNode("mrow", expression);
  } // Build a TeX annotation of the source


  var annotation = new mathMLTree.MathNode("annotation", [new mathMLTree.TextNode(texExpression)]);
  annotation.setAttribute("encoding", "application/x-tex");
  var semantics = new mathMLTree.MathNode("semantics", [wrapper, annotation]);
  var math = new mathMLTree.MathNode("math", [semantics]); // You can't style <math> nodes, so we wrap the node in a span.
  // NOTE: The span class is not typed to have <math> nodes as children, and
  // we don't want to make the children type more generic since the children
  // of span are expected to have more fields in `buildHtml` contexts.
  // $FlowFixMe

  return buildCommon.makeSpan(["katex-mathml"], [math]);
}
// CONCATENATED MODULE: ./src/buildTree.js







var buildTree_optionsFromSettings = function optionsFromSettings(settings) {
  return new src_Options({
    style: settings.displayMode ? src_Style.DISPLAY : src_Style.TEXT,
    maxSize: settings.maxSize
  });
};

var buildTree_displayWrap = function displayWrap(node, settings) {
  if (settings.displayMode) {
    var classes = ["katex-display"];

    if (settings.leqno) {
      classes.push("leqno");
    }

    if (settings.fleqn) {
      classes.push("fleqn");
    }

    node = buildCommon.makeSpan(classes, [node]);
  }

  return node;
};

var buildTree_buildTree = function buildTree(tree, expression, settings) {
  var options = buildTree_optionsFromSettings(settings);
  var mathMLNode = buildMathML(tree, expression, options);
  var htmlNode = buildHTML(tree, options);
  var katexNode = buildCommon.makeSpan(["katex"], [mathMLNode, htmlNode]);
  return buildTree_displayWrap(katexNode, settings);
};
var buildTree_buildHTMLTree = function buildHTMLTree(tree, expression, settings) {
  var options = buildTree_optionsFromSettings(settings);
  var htmlNode = buildHTML(tree, options);
  var katexNode = buildCommon.makeSpan(["katex"], [htmlNode]);
  return buildTree_displayWrap(katexNode, settings);
};
/* harmony default export */ var src_buildTree = (buildTree_buildTree);
// CONCATENATED MODULE: ./src/stretchy.js
/**
 * This file provides support to buildMathML.js and buildHTML.js
 * for stretchy wide elements rendered from SVG files
 * and other CSS trickery.
 */




var stretchyCodePoint = {
  widehat: "^",
  widecheck: "ˇ",
  widetilde: "~",
  utilde: "~",
  overleftarrow: "\u2190",
  underleftarrow: "\u2190",
  xleftarrow: "\u2190",
  overrightarrow: "\u2192",
  underrightarrow: "\u2192",
  xrightarrow: "\u2192",
  underbrace: "\u23DF",
  overbrace: "\u23DE",
  overgroup: "\u23E0",
  undergroup: "\u23E1",
  overleftrightarrow: "\u2194",
  underleftrightarrow: "\u2194",
  xleftrightarrow: "\u2194",
  Overrightarrow: "\u21D2",
  xRightarrow: "\u21D2",
  overleftharpoon: "\u21BC",
  xleftharpoonup: "\u21BC",
  overrightharpoon: "\u21C0",
  xrightharpoonup: "\u21C0",
  xLeftarrow: "\u21D0",
  xLeftrightarrow: "\u21D4",
  xhookleftarrow: "\u21A9",
  xhookrightarrow: "\u21AA",
  xmapsto: "\u21A6",
  xrightharpoondown: "\u21C1",
  xleftharpoondown: "\u21BD",
  xrightleftharpoons: "\u21CC",
  xleftrightharpoons: "\u21CB",
  xtwoheadleftarrow: "\u219E",
  xtwoheadrightarrow: "\u21A0",
  xlongequal: "=",
  xtofrom: "\u21C4",
  xrightleftarrows: "\u21C4",
  xrightequilibrium: "\u21CC",
  // Not a perfect match.
  xleftequilibrium: "\u21CB" // None better available.

};

var stretchy_mathMLnode = function mathMLnode(label) {
  var node = new mathMLTree.MathNode("mo", [new mathMLTree.TextNode(stretchyCodePoint[label.substr(1)])]);
  node.setAttribute("stretchy", "true");
  return node;
}; // Many of the KaTeX SVG images have been adapted from glyphs in KaTeX fonts.
// Copyright (c) 2009-2010, Design Science, Inc. (<www.mathjax.org>)
// Copyright (c) 2014-2017 Khan Academy (<www.khanacademy.org>)
// Licensed under the SIL Open Font License, Version 1.1.
// See \nhttp://scripts.sil.org/OFL
// Very Long SVGs
//    Many of the KaTeX stretchy wide elements use a long SVG image and an
//    overflow: hidden tactic to achieve a stretchy image while avoiding
//    distortion of arrowheads or brace corners.
//    The SVG typically contains a very long (400 em) arrow.
//    The SVG is in a container span that has overflow: hidden, so the span
//    acts like a window that exposes only part of the  SVG.
//    The SVG always has a longer, thinner aspect ratio than the container span.
//    After the SVG fills 100% of the height of the container span,
//    there is a long arrow shaft left over. That left-over shaft is not shown.
//    Instead, it is sliced off because the span's CSS has overflow: hidden.
//    Thus, the reader sees an arrow that matches the subject matter width
//    without distortion.
//    Some functions, such as \cancel, need to vary their aspect ratio. These
//    functions do not get the overflow SVG treatment.
// Second Brush Stroke
//    Low resolution monitors struggle to display images in fine detail.
//    So browsers apply anti-aliasing. A long straight arrow shaft therefore
//    will sometimes appear as if it has a blurred edge.
//    To mitigate this, these SVG files contain a second "brush-stroke" on the
//    arrow shafts. That is, a second long thin rectangular SVG path has been
//    written directly on top of each arrow shaft. This reinforcement causes
//    some of the screen pixels to display as black instead of the anti-aliased
//    gray pixel that a  single path would generate. So we get arrow shafts
//    whose edges appear to be sharper.
// In the katexImagesData object just below, the dimensions all
// correspond to path geometry inside the relevant SVG.
// For example, \overrightarrow uses the same arrowhead as glyph U+2192
// from the KaTeX Main font. The scaling factor is 1000.
// That is, inside the font, that arrowhead is 522 units tall, which
// corresponds to 0.522 em inside the document.


var katexImagesData = {
  //   path(s), minWidth, height, align
  overrightarrow: [["rightarrow"], 0.888, 522, "xMaxYMin"],
  overleftarrow: [["leftarrow"], 0.888, 522, "xMinYMin"],
  underrightarrow: [["rightarrow"], 0.888, 522, "xMaxYMin"],
  underleftarrow: [["leftarrow"], 0.888, 522, "xMinYMin"],
  xrightarrow: [["rightarrow"], 1.469, 522, "xMaxYMin"],
  xleftarrow: [["leftarrow"], 1.469, 522, "xMinYMin"],
  Overrightarrow: [["doublerightarrow"], 0.888, 560, "xMaxYMin"],
  xRightarrow: [["doublerightarrow"], 1.526, 560, "xMaxYMin"],
  xLeftarrow: [["doubleleftarrow"], 1.526, 560, "xMinYMin"],
  overleftharpoon: [["leftharpoon"], 0.888, 522, "xMinYMin"],
  xleftharpoonup: [["leftharpoon"], 0.888, 522, "xMinYMin"],
  xleftharpoondown: [["leftharpoondown"], 0.888, 522, "xMinYMin"],
  overrightharpoon: [["rightharpoon"], 0.888, 522, "xMaxYMin"],
  xrightharpoonup: [["rightharpoon"], 0.888, 522, "xMaxYMin"],
  xrightharpoondown: [["rightharpoondown"], 0.888, 522, "xMaxYMin"],
  xlongequal: [["longequal"], 0.888, 334, "xMinYMin"],
  xtwoheadleftarrow: [["twoheadleftarrow"], 0.888, 334, "xMinYMin"],
  xtwoheadrightarrow: [["twoheadrightarrow"], 0.888, 334, "xMaxYMin"],
  overleftrightarrow: [["leftarrow", "rightarrow"], 0.888, 522],
  overbrace: [["leftbrace", "midbrace", "rightbrace"], 1.6, 548],
  underbrace: [["leftbraceunder", "midbraceunder", "rightbraceunder"], 1.6, 548],
  underleftrightarrow: [["leftarrow", "rightarrow"], 0.888, 522],
  xleftrightarrow: [["leftarrow", "rightarrow"], 1.75, 522],
  xLeftrightarrow: [["doubleleftarrow", "doublerightarrow"], 1.75, 560],
  xrightleftharpoons: [["leftharpoondownplus", "rightharpoonplus"], 1.75, 716],
  xleftrightharpoons: [["leftharpoonplus", "rightharpoondownplus"], 1.75, 716],
  xhookleftarrow: [["leftarrow", "righthook"], 1.08, 522],
  xhookrightarrow: [["lefthook", "rightarrow"], 1.08, 522],
  overlinesegment: [["leftlinesegment", "rightlinesegment"], 0.888, 522],
  underlinesegment: [["leftlinesegment", "rightlinesegment"], 0.888, 522],
  overgroup: [["leftgroup", "rightgroup"], 0.888, 342],
  undergroup: [["leftgroupunder", "rightgroupunder"], 0.888, 342],
  xmapsto: [["leftmapsto", "rightarrow"], 1.5, 522],
  xtofrom: [["leftToFrom", "rightToFrom"], 1.75, 528],
  // The next three arrows are from the mhchem package.
  // In mhchem.sty, min-length is 2.0em. But these arrows might appear in the
  // document as \xrightarrow or \xrightleftharpoons. Those have
  // min-length = 1.75em, so we set min-length on these next three to match.
  xrightleftarrows: [["baraboveleftarrow", "rightarrowabovebar"], 1.75, 901],
  xrightequilibrium: [["baraboveshortleftharpoon", "rightharpoonaboveshortbar"], 1.75, 716],
  xleftequilibrium: [["shortbaraboveleftharpoon", "shortrightharpoonabovebar"], 1.75, 716]
};

var groupLength = function groupLength(arg) {
  if (arg.type === "ordgroup") {
    return arg.body.length;
  } else {
    return 1;
  }
};

var stretchy_svgSpan = function svgSpan(group, options) {
  // Create a span with inline SVG for the element.
  function buildSvgSpan_() {
    var viewBoxWidth = 400000; // default

    var label = group.label.substr(1);

    if (utils.contains(["widehat", "widecheck", "widetilde", "utilde"], label)) {
      // Each type in the `if` statement corresponds to one of the ParseNode
      // types below. This narrowing is required to access `grp.base`.
      var grp = group; // There are four SVG images available for each function.
      // Choose a taller image when there are more characters.

      var numChars = groupLength(grp.base);
      var viewBoxHeight;
      var pathName;

      var _height;

      if (numChars > 5) {
        if (label === "widehat" || label === "widecheck") {
          viewBoxHeight = 420;
          viewBoxWidth = 2364;
          _height = 0.42;
          pathName = label + "4";
        } else {
          viewBoxHeight = 312;
          viewBoxWidth = 2340;
          _height = 0.34;
          pathName = "tilde4";
        }
      } else {
        var imgIndex = [1, 1, 2, 2, 3, 3][numChars];

        if (label === "widehat" || label === "widecheck") {
          viewBoxWidth = [0, 1062, 2364, 2364, 2364][imgIndex];
          viewBoxHeight = [0, 239, 300, 360, 420][imgIndex];
          _height = [0, 0.24, 0.3, 0.3, 0.36, 0.42][imgIndex];
          pathName = label + imgIndex;
        } else {
          viewBoxWidth = [0, 600, 1033, 2339, 2340][imgIndex];
          viewBoxHeight = [0, 260, 286, 306, 312][imgIndex];
          _height = [0, 0.26, 0.286, 0.3, 0.306, 0.34][imgIndex];
          pathName = "tilde" + imgIndex;
        }
      }

      var path = new domTree_PathNode(pathName);
      var svgNode = new SvgNode([path], {
        "width": "100%",
        "height": _height + "em",
        "viewBox": "0 0 " + viewBoxWidth + " " + viewBoxHeight,
        "preserveAspectRatio": "none"
      });
      return {
        span: buildCommon.makeSvgSpan([], [svgNode], options),
        minWidth: 0,
        height: _height
      };
    } else {
      var spans = [];
      var data = katexImagesData[label];
      var paths = data[0],
          _minWidth = data[1],
          _viewBoxHeight = data[2];

      var _height2 = _viewBoxHeight / 1000;

      var numSvgChildren = paths.length;
      var widthClasses;
      var aligns;

      if (numSvgChildren === 1) {
        // $FlowFixMe: All these cases must be of the 4-tuple type.
        var align1 = data[3];
        widthClasses = ["hide-tail"];
        aligns = [align1];
      } else if (numSvgChildren === 2) {
        widthClasses = ["halfarrow-left", "halfarrow-right"];
        aligns = ["xMinYMin", "xMaxYMin"];
      } else if (numSvgChildren === 3) {
        widthClasses = ["brace-left", "brace-center", "brace-right"];
        aligns = ["xMinYMin", "xMidYMin", "xMaxYMin"];
      } else {
        throw new Error("Correct katexImagesData or update code here to support\n                    " + numSvgChildren + " children.");
      }

      for (var i = 0; i < numSvgChildren; i++) {
        var _path = new domTree_PathNode(paths[i]);

        var _svgNode = new SvgNode([_path], {
          "width": "400em",
          "height": _height2 + "em",
          "viewBox": "0 0 " + viewBoxWidth + " " + _viewBoxHeight,
          "preserveAspectRatio": aligns[i] + " slice"
        });

        var _span = buildCommon.makeSvgSpan([widthClasses[i]], [_svgNode], options);

        if (numSvgChildren === 1) {
          return {
            span: _span,
            minWidth: _minWidth,
            height: _height2
          };
        } else {
          _span.style.height = _height2 + "em";
          spans.push(_span);
        }
      }

      return {
        span: buildCommon.makeSpan(["stretchy"], spans, options),
        minWidth: _minWidth,
        height: _height2
      };
    }
  } // buildSvgSpan_()


  var _buildSvgSpan_ = buildSvgSpan_(),
      span = _buildSvgSpan_.span,
      minWidth = _buildSvgSpan_.minWidth,
      height = _buildSvgSpan_.height; // Note that we are returning span.depth = 0.
  // Any adjustments relative to the baseline must be done in buildHTML.


  span.height = height;
  span.style.height = height + "em";

  if (minWidth > 0) {
    span.style.minWidth = minWidth + "em";
  }

  return span;
};

var stretchy_encloseSpan = function encloseSpan(inner, label, pad, options) {
  // Return an image span for \cancel, \bcancel, \xcancel, or \fbox
  var img;
  var totalHeight = inner.height + inner.depth + 2 * pad;

  if (/fbox|color/.test(label)) {
    img = buildCommon.makeSpan(["stretchy", label], [], options);

    if (label === "fbox") {
      var color = options.color && options.getColor();

      if (color) {
        img.style.borderColor = color;
      }
    }
  } else {
    // \cancel, \bcancel, or \xcancel
    // Since \cancel's SVG is inline and it omits the viewBox attribute,
    // its stroke-width will not vary with span area.
    var lines = [];

    if (/^[bx]cancel$/.test(label)) {
      lines.push(new LineNode({
        "x1": "0",
        "y1": "0",
        "x2": "100%",
        "y2": "100%",
        "stroke-width": "0.046em"
      }));
    }

    if (/^x?cancel$/.test(label)) {
      lines.push(new LineNode({
        "x1": "0",
        "y1": "100%",
        "x2": "100%",
        "y2": "0",
        "stroke-width": "0.046em"
      }));
    }

    var svgNode = new SvgNode(lines, {
      "width": "100%",
      "height": totalHeight + "em"
    });
    img = buildCommon.makeSvgSpan([], [svgNode], options);
  }

  img.height = totalHeight;
  img.style.height = totalHeight + "em";
  return img;
};

/* harmony default export */ var stretchy = ({
  encloseSpan: stretchy_encloseSpan,
  mathMLnode: stretchy_mathMLnode,
  svgSpan: stretchy_svgSpan
});
// CONCATENATED MODULE: ./src/functions/accent.js









// NOTE: Unlike most `htmlBuilder`s, this one handles not only "accent", but
var accent_htmlBuilder = function htmlBuilder(grp, options) {
  // Accents are handled in the TeXbook pg. 443, rule 12.
  var base;
  var group;
  var supSub = checkNodeType(grp, "supsub");
  var supSubGroup;

  if (supSub) {
    // If our base is a character box, and we have superscripts and
    // subscripts, the supsub will defer to us. In particular, we want
    // to attach the superscripts and subscripts to the inner body (so
    // that the position of the superscripts and subscripts won't be
    // affected by the height of the accent). We accomplish this by
    // sticking the base of the accent into the base of the supsub, and
    // rendering that, while keeping track of where the accent is.
    // The real accent group is the base of the supsub group
    group = assertNodeType(supSub.base, "accent"); // The character box is the base of the accent group

    base = group.base; // Stick the character box into the base of the supsub group

    supSub.base = base; // Rerender the supsub group with its new base, and store that
    // result.

    supSubGroup = assertSpan(buildHTML_buildGroup(supSub, options)); // reset original base

    supSub.base = group;
  } else {
    group = assertNodeType(grp, "accent");
    base = group.base;
  } // Build the base group


  var body = buildHTML_buildGroup(base, options.havingCrampedStyle()); // Does the accent need to shift for the skew of a character?

  var mustShift = group.isShifty && utils.isCharacterBox(base); // Calculate the skew of the accent. This is based on the line "If the
  // nucleus is not a single character, let s = 0; otherwise set s to the
  // kern amount for the nucleus followed by the \skewchar of its font."
  // Note that our skew metrics are just the kern between each character
  // and the skewchar.

  var skew = 0;

  if (mustShift) {
    // If the base is a character box, then we want the skew of the
    // innermost character. To do that, we find the innermost character:
    var baseChar = utils.getBaseElem(base); // Then, we render its group to get the symbol inside it

    var baseGroup = buildHTML_buildGroup(baseChar, options.havingCrampedStyle()); // Finally, we pull the skew off of the symbol.

    skew = assertSymbolDomNode(baseGroup).skew; // Note that we now throw away baseGroup, because the layers we
    // removed with getBaseElem might contain things like \color which
    // we can't get rid of.
    // TODO(emily): Find a better way to get the skew
  } // calculate the amount of space between the body and the accent


  var clearance = Math.min(body.height, options.fontMetrics().xHeight); // Build the accent

  var accentBody;

  if (!group.isStretchy) {
    var accent;
    var width;

    if (group.label === "\\vec") {
      // Before version 0.9, \vec used the combining font glyph U+20D7.
      // But browsers, especially Safari, are not consistent in how they
      // render combining characters when not preceded by a character.
      // So now we use an SVG.
      // If Safari reforms, we should consider reverting to the glyph.
      accent = buildCommon.staticSvg("vec", options);
      width = buildCommon.svgData.vec[1];
    } else {
      accent = buildCommon.makeSymbol(group.label, "Main-Regular", group.mode, options); // Remove the italic correction of the accent, because it only serves to
      // shift the accent over to a place we don't want.

      accent.italic = 0;
      width = accent.width;
    }

    accentBody = buildCommon.makeSpan(["accent-body"], [accent]); // "Full" accents expand the width of the resulting symbol to be
    // at least the width of the accent, and overlap directly onto the
    // character without any vertical offset.

    var accentFull = group.label === "\\textcircled";

    if (accentFull) {
      accentBody.classes.push('accent-full');
      clearance = body.height;
    } // Shift the accent over by the skew.


    var left = skew; // CSS defines `.katex .accent .accent-body:not(.accent-full) { width: 0 }`
    // so that the accent doesn't contribute to the bounding box.
    // We need to shift the character by its width (effectively half
    // its width) to compensate.

    if (!accentFull) {
      left -= width / 2;
    }

    accentBody.style.left = left + "em"; // \textcircled uses the \bigcirc glyph, so it needs some
    // vertical adjustment to match LaTeX.

    if (group.label === "\\textcircled") {
      accentBody.style.top = ".2em";
    }

    accentBody = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: body
      }, {
        type: "kern",
        size: -clearance
      }, {
        type: "elem",
        elem: accentBody
      }]
    }, options);
  } else {
    accentBody = stretchy.svgSpan(group, options);
    accentBody = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: body
      }, {
        type: "elem",
        elem: accentBody,
        wrapperClasses: ["svg-align"],
        wrapperStyle: skew > 0 ? {
          width: "calc(100% - " + 2 * skew + "em)",
          marginLeft: 2 * skew + "em"
        } : undefined
      }]
    }, options);
  }

  var accentWrap = buildCommon.makeSpan(["mord", "accent"], [accentBody], options);

  if (supSubGroup) {
    // Here, we replace the "base" child of the supsub with our newly
    // generated accent.
    supSubGroup.children[0] = accentWrap; // Since we don't rerun the height calculation after replacing the
    // accent, we manually recalculate height.

    supSubGroup.height = Math.max(accentWrap.height, supSubGroup.height); // Accents should always be ords, even when their innards are not.

    supSubGroup.classes[0] = "mord";
    return supSubGroup;
  } else {
    return accentWrap;
  }
};

var accent_mathmlBuilder = function mathmlBuilder(group, options) {
  var accentNode = group.isStretchy ? stretchy.mathMLnode(group.label) : new mathMLTree.MathNode("mo", [buildMathML_makeText(group.label, group.mode)]);
  var node = new mathMLTree.MathNode("mover", [buildMathML_buildGroup(group.base, options), accentNode]);
  node.setAttribute("accent", "true");
  return node;
};

var NON_STRETCHY_ACCENT_REGEX = new RegExp(["\\acute", "\\grave", "\\ddot", "\\tilde", "\\bar", "\\breve", "\\check", "\\hat", "\\vec", "\\dot", "\\mathring"].map(function (accent) {
  return "\\" + accent;
}).join("|")); // Accents

defineFunction({
  type: "accent",
  names: ["\\acute", "\\grave", "\\ddot", "\\tilde", "\\bar", "\\breve", "\\check", "\\hat", "\\vec", "\\dot", "\\mathring", "\\widecheck", "\\widehat", "\\widetilde", "\\overrightarrow", "\\overleftarrow", "\\Overrightarrow", "\\overleftrightarrow", "\\overgroup", "\\overlinesegment", "\\overleftharpoon", "\\overrightharpoon"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    var base = args[0];
    var isStretchy = !NON_STRETCHY_ACCENT_REGEX.test(context.funcName);
    var isShifty = !isStretchy || context.funcName === "\\widehat" || context.funcName === "\\widetilde" || context.funcName === "\\widecheck";
    return {
      type: "accent",
      mode: context.parser.mode,
      label: context.funcName,
      isStretchy: isStretchy,
      isShifty: isShifty,
      base: base
    };
  },
  htmlBuilder: accent_htmlBuilder,
  mathmlBuilder: accent_mathmlBuilder
}); // Text-mode accents

defineFunction({
  type: "accent",
  names: ["\\'", "\\`", "\\^", "\\~", "\\=", "\\u", "\\.", '\\"', "\\r", "\\H", "\\v", "\\textcircled"],
  props: {
    numArgs: 1,
    allowedInText: true,
    allowedInMath: false
  },
  handler: function handler(context, args) {
    var base = args[0];
    return {
      type: "accent",
      mode: context.parser.mode,
      label: context.funcName,
      isStretchy: false,
      isShifty: true,
      base: base
    };
  },
  htmlBuilder: accent_htmlBuilder,
  mathmlBuilder: accent_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/accentunder.js
// Horizontal overlap functions






defineFunction({
  type: "accentUnder",
  names: ["\\underleftarrow", "\\underrightarrow", "\\underleftrightarrow", "\\undergroup", "\\underlinesegment", "\\utilde"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var base = args[0];
    return {
      type: "accentUnder",
      mode: parser.mode,
      label: funcName,
      base: base
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Treat under accents much like underlines.
    var innerGroup = buildHTML_buildGroup(group.base, options);
    var accentBody = stretchy.svgSpan(group, options);
    var kern = group.label === "\\utilde" ? 0.12 : 0; // Generate the vlist, with the appropriate kerns

    var vlist = buildCommon.makeVList({
      positionType: "bottom",
      positionData: accentBody.height + kern,
      children: [{
        type: "elem",
        elem: accentBody,
        wrapperClasses: ["svg-align"]
      }, {
        type: "kern",
        size: kern
      }, {
        type: "elem",
        elem: innerGroup
      }]
    }, options);
    return buildCommon.makeSpan(["mord", "accentunder"], [vlist], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var accentNode = stretchy.mathMLnode(group.label);
    var node = new mathMLTree.MathNode("munder", [buildMathML_buildGroup(group.base, options), accentNode]);
    node.setAttribute("accentunder", "true");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/arrow.js







// Helper function
var arrow_paddedNode = function paddedNode(group) {
  var node = new mathMLTree.MathNode("mpadded", group ? [group] : []);
  node.setAttribute("width", "+0.6em");
  node.setAttribute("lspace", "0.3em");
  return node;
}; // Stretchy arrows with an optional argument


defineFunction({
  type: "xArrow",
  names: ["\\xleftarrow", "\\xrightarrow", "\\xLeftarrow", "\\xRightarrow", "\\xleftrightarrow", "\\xLeftrightarrow", "\\xhookleftarrow", "\\xhookrightarrow", "\\xmapsto", "\\xrightharpoondown", "\\xrightharpoonup", "\\xleftharpoondown", "\\xleftharpoonup", "\\xrightleftharpoons", "\\xleftrightharpoons", "\\xlongequal", "\\xtwoheadrightarrow", "\\xtwoheadleftarrow", "\\xtofrom", // The next 3 functions are here to support the mhchem extension.
  // Direct use of these functions is discouraged and may break someday.
  "\\xrightleftarrows", "\\xrightequilibrium", "\\xleftequilibrium"],
  props: {
    numArgs: 1,
    numOptionalArgs: 1
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    return {
      type: "xArrow",
      mode: parser.mode,
      label: funcName,
      body: args[0],
      below: optArgs[0]
    };
  },
  // Flow is unable to correctly infer the type of `group`, even though it's
  // unamibiguously determined from the passed-in `type` above.
  htmlBuilder: function htmlBuilder(group, options) {
    var style = options.style; // Build the argument groups in the appropriate style.
    // Ref: amsmath.dtx:   \hbox{$\scriptstyle\mkern#3mu{#6}\mkern#4mu$}%
    // Some groups can return document fragments.  Handle those by wrapping
    // them in a span.

    var newOptions = options.havingStyle(style.sup());
    var upperGroup = buildCommon.wrapFragment(buildHTML_buildGroup(group.body, newOptions, options), options);
    upperGroup.classes.push("x-arrow-pad");
    var lowerGroup;

    if (group.below) {
      // Build the lower group
      newOptions = options.havingStyle(style.sub());
      lowerGroup = buildCommon.wrapFragment(buildHTML_buildGroup(group.below, newOptions, options), options);
      lowerGroup.classes.push("x-arrow-pad");
    }

    var arrowBody = stretchy.svgSpan(group, options); // Re shift: Note that stretchy.svgSpan returned arrowBody.depth = 0.
    // The point we want on the math axis is at 0.5 * arrowBody.height.

    var arrowShift = -options.fontMetrics().axisHeight + 0.5 * arrowBody.height; // 2 mu kern. Ref: amsmath.dtx: #7\if0#2\else\mkern#2mu\fi

    var upperShift = -options.fontMetrics().axisHeight - 0.5 * arrowBody.height - 0.111; // 0.111 em = 2 mu

    if (upperGroup.depth > 0.25 || group.label === "\\xleftequilibrium") {
      upperShift -= upperGroup.depth; // shift up if depth encroaches
    } // Generate the vlist


    var vlist;

    if (lowerGroup) {
      var lowerShift = -options.fontMetrics().axisHeight + lowerGroup.height + 0.5 * arrowBody.height + 0.111;
      vlist = buildCommon.makeVList({
        positionType: "individualShift",
        children: [{
          type: "elem",
          elem: upperGroup,
          shift: upperShift
        }, {
          type: "elem",
          elem: arrowBody,
          shift: arrowShift
        }, {
          type: "elem",
          elem: lowerGroup,
          shift: lowerShift
        }]
      }, options);
    } else {
      vlist = buildCommon.makeVList({
        positionType: "individualShift",
        children: [{
          type: "elem",
          elem: upperGroup,
          shift: upperShift
        }, {
          type: "elem",
          elem: arrowBody,
          shift: arrowShift
        }]
      }, options);
    } // $FlowFixMe: Replace this with passing "svg-align" into makeVList.


    vlist.children[0].children[0].children[1].classes.push("svg-align");
    return buildCommon.makeSpan(["mrel", "x-arrow"], [vlist], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var arrowNode = stretchy.mathMLnode(group.label);
    var node;

    if (group.body) {
      var upperNode = arrow_paddedNode(buildMathML_buildGroup(group.body, options));

      if (group.below) {
        var lowerNode = arrow_paddedNode(buildMathML_buildGroup(group.below, options));
        node = new mathMLTree.MathNode("munderover", [arrowNode, lowerNode, upperNode]);
      } else {
        node = new mathMLTree.MathNode("mover", [arrowNode, upperNode]);
      }
    } else if (group.below) {
      var _lowerNode = arrow_paddedNode(buildMathML_buildGroup(group.below, options));

      node = new mathMLTree.MathNode("munder", [arrowNode, _lowerNode]);
    } else {
      // This should never happen.
      // Parser.js throws an error if there is no argument.
      node = arrow_paddedNode();
      node = new mathMLTree.MathNode("mover", [arrowNode, node]);
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/char.js


 // \@char is an internal function that takes a grouped decimal argument like
// {123} and converts into symbol with code 123.  It is used by the *macro*
// \char defined in macros.js.

defineFunction({
  type: "textord",
  names: ["\\@char"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var arg = assertNodeType(args[0], "ordgroup");
    var group = arg.body;
    var number = "";

    for (var i = 0; i < group.length; i++) {
      var node = assertNodeType(group[i], "textord");
      number += node.text;
    }

    var code = parseInt(number);

    if (isNaN(code)) {
      throw new src_ParseError("\\@char has non-numeric argument " + number);
    }

    return {
      type: "textord",
      mode: parser.mode,
      text: String.fromCharCode(code)
    };
  }
});
// CONCATENATED MODULE: ./src/functions/color.js







var color_htmlBuilder = function htmlBuilder(group, options) {
  var elements = buildHTML_buildExpression(group.body, options.withColor(group.color), false); // \color isn't supposed to affect the type of the elements it contains.
  // To accomplish this, we wrap the results in a fragment, so the inner
  // elements will be able to directly interact with their neighbors. For
  // example, `\color{red}{2 +} 3` has the same spacing as `2 + 3`

  return buildCommon.makeFragment(elements);
};

var color_mathmlBuilder = function mathmlBuilder(group, options) {
  var inner = buildMathML_buildExpression(group.body, options.withColor(group.color));
  var node = new mathMLTree.MathNode("mstyle", inner);
  node.setAttribute("mathcolor", group.color);
  return node;
};

defineFunction({
  type: "color",
  names: ["\\textcolor"],
  props: {
    numArgs: 2,
    allowedInText: true,
    greediness: 3,
    argTypes: ["color", "original"]
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var color = assertNodeType(args[0], "color-token").color;
    var body = args[1];
    return {
      type: "color",
      mode: parser.mode,
      color: color,
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: color_htmlBuilder,
  mathmlBuilder: color_mathmlBuilder
});
defineFunction({
  type: "color",
  names: ["\\color"],
  props: {
    numArgs: 1,
    allowedInText: true,
    greediness: 3,
    argTypes: ["color"]
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser,
        breakOnTokenText = _ref2.breakOnTokenText;
    var color = assertNodeType(args[0], "color-token").color; // If we see a styling function, parse out the implicit body

    var body = parser.parseExpression(true, breakOnTokenText);
    return {
      type: "color",
      mode: parser.mode,
      color: color,
      body: body
    };
  },
  htmlBuilder: color_htmlBuilder,
  mathmlBuilder: color_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/cr.js
// Row breaks within tabular environments, and line breaks at top level





 // \\ is a macro mapping to either \cr or \newline.  Because they have the
// same signature, we implement them as one megafunction, with newRow
// indicating whether we're in the \cr case, and newLine indicating whether
// to break the line in the \newline case.

defineFunction({
  type: "cr",
  names: ["\\cr", "\\newline"],
  props: {
    numArgs: 0,
    numOptionalArgs: 1,
    argTypes: ["size"],
    allowedInText: true
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var size = optArgs[0];
    var newRow = funcName === "\\cr";
    var newLine = false;

    if (!newRow) {
      if (parser.settings.displayMode && parser.settings.useStrictBehavior("newLineInDisplayMode", "In LaTeX, \\\\ or \\newline " + "does nothing in display mode")) {
        newLine = false;
      } else {
        newLine = true;
      }
    }

    return {
      type: "cr",
      mode: parser.mode,
      newLine: newLine,
      newRow: newRow,
      size: size && assertNodeType(size, "size").value
    };
  },
  // The following builders are called only at the top level,
  // not within tabular/array environments.
  htmlBuilder: function htmlBuilder(group, options) {
    if (group.newRow) {
      throw new src_ParseError("\\cr valid only within a tabular/array environment");
    }

    var span = buildCommon.makeSpan(["mspace"], [], options);

    if (group.newLine) {
      span.classes.push("newline");

      if (group.size) {
        span.style.marginTop = units_calculateSize(group.size, options) + "em";
      }
    }

    return span;
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node = new mathMLTree.MathNode("mspace");

    if (group.newLine) {
      node.setAttribute("linebreak", "newline");

      if (group.size) {
        node.setAttribute("height", units_calculateSize(group.size, options) + "em");
      }
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/delimiter.js
/**
 * This file deals with creating delimiters of various sizes. The TeXbook
 * discusses these routines on page 441-442, in the "Another subroutine sets box
 * x to a specified variable delimiter" paragraph.
 *
 * There are three main routines here. `makeSmallDelim` makes a delimiter in the
 * normal font, but in either text, script, or scriptscript style.
 * `makeLargeDelim` makes a delimiter in textstyle, but in one of the Size1,
 * Size2, Size3, or Size4 fonts. `makeStackedDelim` makes a delimiter out of
 * smaller pieces that are stacked on top of one another.
 *
 * The functions take a parameter `center`, which determines if the delimiter
 * should be centered around the axis.
 *
 * Then, there are three exposed functions. `sizedDelim` makes a delimiter in
 * one of the given sizes. This is used for things like `\bigl`.
 * `customSizedDelim` makes a delimiter with a given total height+depth. It is
 * called in places like `\sqrt`. `leftRightDelim` makes an appropriate
 * delimiter which surrounds an expression of a given height an depth. It is
 * used in `\left` and `\right`.
 */








/**
 * Get the metrics for a given symbol and font, after transformation (i.e.
 * after following replacement from symbols.js)
 */
var delimiter_getMetrics = function getMetrics(symbol, font, mode) {
  var replace = src_symbols.math[symbol] && src_symbols.math[symbol].replace;
  var metrics = getCharacterMetrics(replace || symbol, font, mode);

  if (!metrics) {
    throw new Error("Unsupported symbol " + symbol + " and font size " + font + ".");
  }

  return metrics;
};
/**
 * Puts a delimiter span in a given style, and adds appropriate height, depth,
 * and maxFontSizes.
 */


var delimiter_styleWrap = function styleWrap(delim, toStyle, options, classes) {
  var newOptions = options.havingBaseStyle(toStyle);
  var span = buildCommon.makeSpan(classes.concat(newOptions.sizingClasses(options)), [delim], options);
  var delimSizeMultiplier = newOptions.sizeMultiplier / options.sizeMultiplier;
  span.height *= delimSizeMultiplier;
  span.depth *= delimSizeMultiplier;
  span.maxFontSize = newOptions.sizeMultiplier;
  return span;
};

var centerSpan = function centerSpan(span, options, style) {
  var newOptions = options.havingBaseStyle(style);
  var shift = (1 - options.sizeMultiplier / newOptions.sizeMultiplier) * options.fontMetrics().axisHeight;
  span.classes.push("delimcenter");
  span.style.top = shift + "em";
  span.height -= shift;
  span.depth += shift;
};
/**
 * Makes a small delimiter. This is a delimiter that comes in the Main-Regular
 * font, but is restyled to either be in textstyle, scriptstyle, or
 * scriptscriptstyle.
 */


var delimiter_makeSmallDelim = function makeSmallDelim(delim, style, center, options, mode, classes) {
  var text = buildCommon.makeSymbol(delim, "Main-Regular", mode, options);
  var span = delimiter_styleWrap(text, style, options, classes);

  if (center) {
    centerSpan(span, options, style);
  }

  return span;
};
/**
 * Builds a symbol in the given font size (note size is an integer)
 */


var delimiter_mathrmSize = function mathrmSize(value, size, mode, options) {
  return buildCommon.makeSymbol(value, "Size" + size + "-Regular", mode, options);
};
/**
 * Makes a large delimiter. This is a delimiter that comes in the Size1, Size2,
 * Size3, or Size4 fonts. It is always rendered in textstyle.
 */


var delimiter_makeLargeDelim = function makeLargeDelim(delim, size, center, options, mode, classes) {
  var inner = delimiter_mathrmSize(delim, size, mode, options);
  var span = delimiter_styleWrap(buildCommon.makeSpan(["delimsizing", "size" + size], [inner], options), src_Style.TEXT, options, classes);

  if (center) {
    centerSpan(span, options, src_Style.TEXT);
  }

  return span;
};
/**
 * Make an inner span with the given offset and in the given font. This is used
 * in `makeStackedDelim` to make the stacking pieces for the delimiter.
 */


var delimiter_makeInner = function makeInner(symbol, font, mode) {
  var sizeClass; // Apply the correct CSS class to choose the right font.

  if (font === "Size1-Regular") {
    sizeClass = "delim-size1";
  } else
    /* if (font === "Size4-Regular") */
    {
      sizeClass = "delim-size4";
    }

  var inner = buildCommon.makeSpan(["delimsizinginner", sizeClass], [buildCommon.makeSpan([], [buildCommon.makeSymbol(symbol, font, mode)])]); // Since this will be passed into `makeVList` in the end, wrap the element
  // in the appropriate tag that VList uses.

  return {
    type: "elem",
    elem: inner
  };
};
/**
 * Make a stacked delimiter out of a given delimiter, with the total height at
 * least `heightTotal`. This routine is mentioned on page 442 of the TeXbook.
 */


var delimiter_makeStackedDelim = function makeStackedDelim(delim, heightTotal, center, options, mode, classes) {
  // There are four parts, the top, an optional middle, a repeated part, and a
  // bottom.
  var top;
  var middle;
  var repeat;
  var bottom;
  top = repeat = bottom = delim;
  middle = null; // Also keep track of what font the delimiters are in

  var font = "Size1-Regular"; // We set the parts and font based on the symbol. Note that we use
  // '\u23d0' instead of '|' and '\u2016' instead of '\\|' for the
  // repeats of the arrows

  if (delim === "\\uparrow") {
    repeat = bottom = "\u23D0";
  } else if (delim === "\\Uparrow") {
    repeat = bottom = "\u2016";
  } else if (delim === "\\downarrow") {
    top = repeat = "\u23D0";
  } else if (delim === "\\Downarrow") {
    top = repeat = "\u2016";
  } else if (delim === "\\updownarrow") {
    top = "\\uparrow";
    repeat = "\u23D0";
    bottom = "\\downarrow";
  } else if (delim === "\\Updownarrow") {
    top = "\\Uparrow";
    repeat = "\u2016";
    bottom = "\\Downarrow";
  } else if (delim === "[" || delim === "\\lbrack") {
    top = "\u23A1";
    repeat = "\u23A2";
    bottom = "\u23A3";
    font = "Size4-Regular";
  } else if (delim === "]" || delim === "\\rbrack") {
    top = "\u23A4";
    repeat = "\u23A5";
    bottom = "\u23A6";
    font = "Size4-Regular";
  } else if (delim === "\\lfloor" || delim === "\u230A") {
    repeat = top = "\u23A2";
    bottom = "\u23A3";
    font = "Size4-Regular";
  } else if (delim === "\\lceil" || delim === "\u2308") {
    top = "\u23A1";
    repeat = bottom = "\u23A2";
    font = "Size4-Regular";
  } else if (delim === "\\rfloor" || delim === "\u230B") {
    repeat = top = "\u23A5";
    bottom = "\u23A6";
    font = "Size4-Regular";
  } else if (delim === "\\rceil" || delim === "\u2309") {
    top = "\u23A4";
    repeat = bottom = "\u23A5";
    font = "Size4-Regular";
  } else if (delim === "(" || delim === "\\lparen") {
    top = "\u239B";
    repeat = "\u239C";
    bottom = "\u239D";
    font = "Size4-Regular";
  } else if (delim === ")" || delim === "\\rparen") {
    top = "\u239E";
    repeat = "\u239F";
    bottom = "\u23A0";
    font = "Size4-Regular";
  } else if (delim === "\\{" || delim === "\\lbrace") {
    top = "\u23A7";
    middle = "\u23A8";
    bottom = "\u23A9";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } else if (delim === "\\}" || delim === "\\rbrace") {
    top = "\u23AB";
    middle = "\u23AC";
    bottom = "\u23AD";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } else if (delim === "\\lgroup" || delim === "\u27EE") {
    top = "\u23A7";
    bottom = "\u23A9";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } else if (delim === "\\rgroup" || delim === "\u27EF") {
    top = "\u23AB";
    bottom = "\u23AD";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } else if (delim === "\\lmoustache" || delim === "\u23B0") {
    top = "\u23A7";
    bottom = "\u23AD";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } else if (delim === "\\rmoustache" || delim === "\u23B1") {
    top = "\u23AB";
    bottom = "\u23A9";
    repeat = "\u23AA";
    font = "Size4-Regular";
  } // Get the metrics of the four sections


  var topMetrics = delimiter_getMetrics(top, font, mode);
  var topHeightTotal = topMetrics.height + topMetrics.depth;
  var repeatMetrics = delimiter_getMetrics(repeat, font, mode);
  var repeatHeightTotal = repeatMetrics.height + repeatMetrics.depth;
  var bottomMetrics = delimiter_getMetrics(bottom, font, mode);
  var bottomHeightTotal = bottomMetrics.height + bottomMetrics.depth;
  var middleHeightTotal = 0;
  var middleFactor = 1;

  if (middle !== null) {
    var middleMetrics = delimiter_getMetrics(middle, font, mode);
    middleHeightTotal = middleMetrics.height + middleMetrics.depth;
    middleFactor = 2; // repeat symmetrically above and below middle
  } // Calcuate the minimal height that the delimiter can have.
  // It is at least the size of the top, bottom, and optional middle combined.


  var minHeight = topHeightTotal + bottomHeightTotal + middleHeightTotal; // Compute the number of copies of the repeat symbol we will need

  var repeatCount = Math.ceil((heightTotal - minHeight) / (middleFactor * repeatHeightTotal)); // Compute the total height of the delimiter including all the symbols

  var realHeightTotal = minHeight + repeatCount * middleFactor * repeatHeightTotal; // The center of the delimiter is placed at the center of the axis. Note
  // that in this context, "center" means that the delimiter should be
  // centered around the axis in the current style, while normally it is
  // centered around the axis in textstyle.

  var axisHeight = options.fontMetrics().axisHeight;

  if (center) {
    axisHeight *= options.sizeMultiplier;
  } // Calculate the depth


  var depth = realHeightTotal / 2 - axisHeight; // Now, we start building the pieces that will go into the vlist
  // Keep a list of the inner pieces

  var inners = []; // Add the bottom symbol

  inners.push(delimiter_makeInner(bottom, font, mode));

  if (middle === null) {
    // Add that many symbols
    for (var i = 0; i < repeatCount; i++) {
      inners.push(delimiter_makeInner(repeat, font, mode));
    }
  } else {
    // When there is a middle bit, we need the middle part and two repeated
    // sections
    for (var _i = 0; _i < repeatCount; _i++) {
      inners.push(delimiter_makeInner(repeat, font, mode));
    }

    inners.push(delimiter_makeInner(middle, font, mode));

    for (var _i2 = 0; _i2 < repeatCount; _i2++) {
      inners.push(delimiter_makeInner(repeat, font, mode));
    }
  } // Add the top symbol


  inners.push(delimiter_makeInner(top, font, mode)); // Finally, build the vlist

  var newOptions = options.havingBaseStyle(src_Style.TEXT);
  var inner = buildCommon.makeVList({
    positionType: "bottom",
    positionData: depth,
    children: inners
  }, newOptions);
  return delimiter_styleWrap(buildCommon.makeSpan(["delimsizing", "mult"], [inner], newOptions), src_Style.TEXT, options, classes);
}; // All surds have 0.08em padding above the viniculum inside the SVG.
// That keeps browser span height rounding error from pinching the line.


var vbPad = 80; // padding above the surd, measured inside the viewBox.

var emPad = 0.08; // padding, in ems, measured in the document.

var delimiter_sqrtSvg = function sqrtSvg(sqrtName, height, viewBoxHeight, options) {
  var alternate;

  if (sqrtName === "sqrtTall") {
    // sqrtTall is from glyph U23B7 in the font KaTeX_Size4-Regular
    // One path edge has a variable length. It runs from the viniculumn
    // to a point near (14 units) the bottom of the surd. The viniculum
    // is 40 units thick. So the length of the line in question is:
    var vertSegment = viewBoxHeight - 54 - vbPad;
    alternate = "M702 " + vbPad + "H400000v40H742v" + vertSegment + "l-4 4-4 4c-.667.7\n-2 1.5-4 2.5s-4.167 1.833-6.5 2.5-5.5 1-9.5 1h-12l-28-84c-16.667-52-96.667\n-294.333-240-727l-212 -643 -85 170c-4-3.333-8.333-7.667-13 -13l-13-13l77-155\n 77-156c66 199.333 139 419.667 219 661 l218 661zM702 " + vbPad + "H400000v40H742z";
  }

  var pathNode = new domTree_PathNode(sqrtName, alternate);
  var svg = new SvgNode([pathNode], {
    // Note: 1000:1 ratio of viewBox to document em width.
    "width": "400em",
    "height": height + "em",
    "viewBox": "0 0 400000 " + viewBoxHeight,
    "preserveAspectRatio": "xMinYMin slice"
  });
  return buildCommon.makeSvgSpan(["hide-tail"], [svg], options);
};
/**
 * Make a sqrt image of the given height,
 */


var makeSqrtImage = function makeSqrtImage(height, options) {
  // Define a newOptions that removes the effect of size changes such as \Huge.
  // We don't pick different a height surd for \Huge. For it, we scale up.
  var newOptions = options.havingBaseSizing(); // Pick the desired surd glyph from a sequence of surds.

  var delim = traverseSequence("\\surd", height * newOptions.sizeMultiplier, stackLargeDelimiterSequence, newOptions);
  var sizeMultiplier = newOptions.sizeMultiplier; // default
  // Create a span containing an SVG image of a sqrt symbol.

  var span;
  var spanHeight = 0;
  var texHeight = 0;
  var viewBoxHeight = 0;
  var advanceWidth; // We create viewBoxes with 80 units of "padding" above each surd.
  // Then browser rounding error on the parent span height will not
  // encroach on the ink of the viniculum. But that padding is not
  // included in the TeX-like `height` used for calculation of
  // vertical alignment. So texHeight = span.height < span.style.height.

  if (delim.type === "small") {
    // Get an SVG that is derived from glyph U+221A in font KaTeX-Main.
    viewBoxHeight = 1000 + vbPad; // 1000 unit glyph height.

    if (height < 1.0) {
      sizeMultiplier = 1.0; // mimic a \textfont radical
    } else if (height < 1.4) {
      sizeMultiplier = 0.7; // mimic a \scriptfont radical
    }

    spanHeight = (1.0 + emPad) / sizeMultiplier;
    texHeight = 1.00 / sizeMultiplier;
    span = delimiter_sqrtSvg("sqrtMain", spanHeight, viewBoxHeight, options);
    span.style.minWidth = "0.853em";
    advanceWidth = 0.833 / sizeMultiplier; // from the font.
  } else if (delim.type === "large") {
    // These SVGs come from fonts: KaTeX_Size1, _Size2, etc.
    viewBoxHeight = (1000 + vbPad) * sizeToMaxHeight[delim.size];
    texHeight = sizeToMaxHeight[delim.size] / sizeMultiplier;
    spanHeight = (sizeToMaxHeight[delim.size] + emPad) / sizeMultiplier;
    span = delimiter_sqrtSvg("sqrtSize" + delim.size, spanHeight, viewBoxHeight, options);
    span.style.minWidth = "1.02em";
    advanceWidth = 1.0 / sizeMultiplier; // 1.0 from the font.
  } else {
    // Tall sqrt. In TeX, this would be stacked using multiple glyphs.
    // We'll use a single SVG to accomplish the same thing.
    spanHeight = height + emPad;
    texHeight = height;
    viewBoxHeight = Math.floor(1000 * height) + vbPad;
    span = delimiter_sqrtSvg("sqrtTall", spanHeight, viewBoxHeight, options);
    span.style.minWidth = "0.742em";
    advanceWidth = 1.056;
  }

  span.height = texHeight;
  span.style.height = spanHeight + "em";
  return {
    span: span,
    advanceWidth: advanceWidth,
    // Calculate the actual line width.
    // This actually should depend on the chosen font -- e.g. \boldmath
    // should use the thicker surd symbols from e.g. KaTeX_Main-Bold, and
    // have thicker rules.
    ruleWidth: options.fontMetrics().sqrtRuleThickness * sizeMultiplier
  };
}; // There are three kinds of delimiters, delimiters that stack when they become
// too large


var stackLargeDelimiters = ["(", "\\lparen", ")", "\\rparen", "[", "\\lbrack", "]", "\\rbrack", "\\{", "\\lbrace", "\\}", "\\rbrace", "\\lfloor", "\\rfloor", "\u230A", "\u230B", "\\lceil", "\\rceil", "\u2308", "\u2309", "\\surd"]; // delimiters that always stack

var stackAlwaysDelimiters = ["\\uparrow", "\\downarrow", "\\updownarrow", "\\Uparrow", "\\Downarrow", "\\Updownarrow", "|", "\\|", "\\vert", "\\Vert", "\\lvert", "\\rvert", "\\lVert", "\\rVert", "\\lgroup", "\\rgroup", "\u27EE", "\u27EF", "\\lmoustache", "\\rmoustache", "\u23B0", "\u23B1"]; // and delimiters that never stack

var stackNeverDelimiters = ["<", ">", "\\langle", "\\rangle", "/", "\\backslash", "\\lt", "\\gt"]; // Metrics of the different sizes. Found by looking at TeX's output of
// $\bigl| // \Bigl| \biggl| \Biggl| \showlists$
// Used to create stacked delimiters of appropriate sizes in makeSizedDelim.

var sizeToMaxHeight = [0, 1.2, 1.8, 2.4, 3.0];
/**
 * Used to create a delimiter of a specific size, where `size` is 1, 2, 3, or 4.
 */

var delimiter_makeSizedDelim = function makeSizedDelim(delim, size, options, mode, classes) {
  // < and > turn into \langle and \rangle in delimiters
  if (delim === "<" || delim === "\\lt" || delim === "\u27E8") {
    delim = "\\langle";
  } else if (delim === ">" || delim === "\\gt" || delim === "\u27E9") {
    delim = "\\rangle";
  } // Sized delimiters are never centered.


  if (utils.contains(stackLargeDelimiters, delim) || utils.contains(stackNeverDelimiters, delim)) {
    return delimiter_makeLargeDelim(delim, size, false, options, mode, classes);
  } else if (utils.contains(stackAlwaysDelimiters, delim)) {
    return delimiter_makeStackedDelim(delim, sizeToMaxHeight[size], false, options, mode, classes);
  } else {
    throw new src_ParseError("Illegal delimiter: '" + delim + "'");
  }
};
/**
 * There are three different sequences of delimiter sizes that the delimiters
 * follow depending on the kind of delimiter. This is used when creating custom
 * sized delimiters to decide whether to create a small, large, or stacked
 * delimiter.
 *
 * In real TeX, these sequences aren't explicitly defined, but are instead
 * defined inside the font metrics. Since there are only three sequences that
 * are possible for the delimiters that TeX defines, it is easier to just encode
 * them explicitly here.
 */


// Delimiters that never stack try small delimiters and large delimiters only
var stackNeverDelimiterSequence = [{
  type: "small",
  style: src_Style.SCRIPTSCRIPT
}, {
  type: "small",
  style: src_Style.SCRIPT
}, {
  type: "small",
  style: src_Style.TEXT
}, {
  type: "large",
  size: 1
}, {
  type: "large",
  size: 2
}, {
  type: "large",
  size: 3
}, {
  type: "large",
  size: 4
}]; // Delimiters that always stack try the small delimiters first, then stack

var stackAlwaysDelimiterSequence = [{
  type: "small",
  style: src_Style.SCRIPTSCRIPT
}, {
  type: "small",
  style: src_Style.SCRIPT
}, {
  type: "small",
  style: src_Style.TEXT
}, {
  type: "stack"
}]; // Delimiters that stack when large try the small and then large delimiters, and
// stack afterwards

var stackLargeDelimiterSequence = [{
  type: "small",
  style: src_Style.SCRIPTSCRIPT
}, {
  type: "small",
  style: src_Style.SCRIPT
}, {
  type: "small",
  style: src_Style.TEXT
}, {
  type: "large",
  size: 1
}, {
  type: "large",
  size: 2
}, {
  type: "large",
  size: 3
}, {
  type: "large",
  size: 4
}, {
  type: "stack"
}];
/**
 * Get the font used in a delimiter based on what kind of delimiter it is.
 * TODO(#963) Use more specific font family return type once that is introduced.
 */

var delimTypeToFont = function delimTypeToFont(type) {
  if (type.type === "small") {
    return "Main-Regular";
  } else if (type.type === "large") {
    return "Size" + type.size + "-Regular";
  } else if (type.type === "stack") {
    return "Size4-Regular";
  } else {
    throw new Error("Add support for delim type '" + type.type + "' here.");
  }
};
/**
 * Traverse a sequence of types of delimiters to decide what kind of delimiter
 * should be used to create a delimiter of the given height+depth.
 */


var traverseSequence = function traverseSequence(delim, height, sequence, options) {
  // Here, we choose the index we should start at in the sequences. In smaller
  // sizes (which correspond to larger numbers in style.size) we start earlier
  // in the sequence. Thus, scriptscript starts at index 3-3=0, script starts
  // at index 3-2=1, text starts at 3-1=2, and display starts at min(2,3-0)=2
  var start = Math.min(2, 3 - options.style.size);

  for (var i = start; i < sequence.length; i++) {
    if (sequence[i].type === "stack") {
      // This is always the last delimiter, so we just break the loop now.
      break;
    }

    var metrics = delimiter_getMetrics(delim, delimTypeToFont(sequence[i]), "math");
    var heightDepth = metrics.height + metrics.depth; // Small delimiters are scaled down versions of the same font, so we
    // account for the style change size.

    if (sequence[i].type === "small") {
      var newOptions = options.havingBaseStyle(sequence[i].style);
      heightDepth *= newOptions.sizeMultiplier;
    } // Check if the delimiter at this size works for the given height.


    if (heightDepth > height) {
      return sequence[i];
    }
  } // If we reached the end of the sequence, return the last sequence element.


  return sequence[sequence.length - 1];
};
/**
 * Make a delimiter of a given height+depth, with optional centering. Here, we
 * traverse the sequences, and create a delimiter that the sequence tells us to.
 */


var delimiter_makeCustomSizedDelim = function makeCustomSizedDelim(delim, height, center, options, mode, classes) {
  if (delim === "<" || delim === "\\lt" || delim === "\u27E8") {
    delim = "\\langle";
  } else if (delim === ">" || delim === "\\gt" || delim === "\u27E9") {
    delim = "\\rangle";
  } // Decide what sequence to use


  var sequence;

  if (utils.contains(stackNeverDelimiters, delim)) {
    sequence = stackNeverDelimiterSequence;
  } else if (utils.contains(stackLargeDelimiters, delim)) {
    sequence = stackLargeDelimiterSequence;
  } else {
    sequence = stackAlwaysDelimiterSequence;
  } // Look through the sequence


  var delimType = traverseSequence(delim, height, sequence, options); // Get the delimiter from font glyphs.
  // Depending on the sequence element we decided on, call the
  // appropriate function.

  if (delimType.type === "small") {
    return delimiter_makeSmallDelim(delim, delimType.style, center, options, mode, classes);
  } else if (delimType.type === "large") {
    return delimiter_makeLargeDelim(delim, delimType.size, center, options, mode, classes);
  } else
    /* if (delimType.type === "stack") */
    {
      return delimiter_makeStackedDelim(delim, height, center, options, mode, classes);
    }
};
/**
 * Make a delimiter for use with `\left` and `\right`, given a height and depth
 * of an expression that the delimiters surround.
 */


var makeLeftRightDelim = function makeLeftRightDelim(delim, height, depth, options, mode, classes) {
  // We always center \left/\right delimiters, so the axis is always shifted
  var axisHeight = options.fontMetrics().axisHeight * options.sizeMultiplier; // Taken from TeX source, tex.web, function make_left_right

  var delimiterFactor = 901;
  var delimiterExtend = 5.0 / options.fontMetrics().ptPerEm;
  var maxDistFromAxis = Math.max(height - axisHeight, depth + axisHeight);
  var totalHeight = Math.max( // In real TeX, calculations are done using integral values which are
  // 65536 per pt, or 655360 per em. So, the division here truncates in
  // TeX but doesn't here, producing different results. If we wanted to
  // exactly match TeX's calculation, we could do
  //   Math.floor(655360 * maxDistFromAxis / 500) *
  //    delimiterFactor / 655360
  // (To see the difference, compare
  //    x^{x^{\left(\rule{0.1em}{0.68em}\right)}}
  // in TeX and KaTeX)
  maxDistFromAxis / 500 * delimiterFactor, 2 * maxDistFromAxis - delimiterExtend); // Finally, we defer to `makeCustomSizedDelim` with our calculated total
  // height

  return delimiter_makeCustomSizedDelim(delim, totalHeight, true, options, mode, classes);
};

/* harmony default export */ var delimiter = ({
  sqrtImage: makeSqrtImage,
  sizedDelim: delimiter_makeSizedDelim,
  customSizedDelim: delimiter_makeCustomSizedDelim,
  leftRightDelim: makeLeftRightDelim
});
// CONCATENATED MODULE: ./src/functions/delimsizing.js









// Extra data needed for the delimiter handler down below
var delimiterSizes = {
  "\\bigl": {
    mclass: "mopen",
    size: 1
  },
  "\\Bigl": {
    mclass: "mopen",
    size: 2
  },
  "\\biggl": {
    mclass: "mopen",
    size: 3
  },
  "\\Biggl": {
    mclass: "mopen",
    size: 4
  },
  "\\bigr": {
    mclass: "mclose",
    size: 1
  },
  "\\Bigr": {
    mclass: "mclose",
    size: 2
  },
  "\\biggr": {
    mclass: "mclose",
    size: 3
  },
  "\\Biggr": {
    mclass: "mclose",
    size: 4
  },
  "\\bigm": {
    mclass: "mrel",
    size: 1
  },
  "\\Bigm": {
    mclass: "mrel",
    size: 2
  },
  "\\biggm": {
    mclass: "mrel",
    size: 3
  },
  "\\Biggm": {
    mclass: "mrel",
    size: 4
  },
  "\\big": {
    mclass: "mord",
    size: 1
  },
  "\\Big": {
    mclass: "mord",
    size: 2
  },
  "\\bigg": {
    mclass: "mord",
    size: 3
  },
  "\\Bigg": {
    mclass: "mord",
    size: 4
  }
};
var delimiters = ["(", "\\lparen", ")", "\\rparen", "[", "\\lbrack", "]", "\\rbrack", "\\{", "\\lbrace", "\\}", "\\rbrace", "\\lfloor", "\\rfloor", "\u230A", "\u230B", "\\lceil", "\\rceil", "\u2308", "\u2309", "<", ">", "\\langle", "\u27E8", "\\rangle", "\u27E9", "\\lt", "\\gt", "\\lvert", "\\rvert", "\\lVert", "\\rVert", "\\lgroup", "\\rgroup", "\u27EE", "\u27EF", "\\lmoustache", "\\rmoustache", "\u23B0", "\u23B1", "/", "\\backslash", "|", "\\vert", "\\|", "\\Vert", "\\uparrow", "\\Uparrow", "\\downarrow", "\\Downarrow", "\\updownarrow", "\\Updownarrow", "."];

// Delimiter functions
function checkDelimiter(delim, context) {
  var symDelim = checkSymbolNodeType(delim);

  if (symDelim && utils.contains(delimiters, symDelim.text)) {
    return symDelim;
  } else {
    throw new src_ParseError("Invalid delimiter: '" + (symDelim ? symDelim.text : JSON.stringify(delim)) + "' after '" + context.funcName + "'", delim);
  }
}

defineFunction({
  type: "delimsizing",
  names: ["\\bigl", "\\Bigl", "\\biggl", "\\Biggl", "\\bigr", "\\Bigr", "\\biggr", "\\Biggr", "\\bigm", "\\Bigm", "\\biggm", "\\Biggm", "\\big", "\\Big", "\\bigg", "\\Bigg"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    var delim = checkDelimiter(args[0], context);
    return {
      type: "delimsizing",
      mode: context.parser.mode,
      size: delimiterSizes[context.funcName].size,
      mclass: delimiterSizes[context.funcName].mclass,
      delim: delim.text
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    if (group.delim === ".") {
      // Empty delimiters still count as elements, even though they don't
      // show anything.
      return buildCommon.makeSpan([group.mclass]);
    } // Use delimiter.sizedDelim to generate the delimiter.


    return delimiter.sizedDelim(group.delim, group.size, options, group.mode, [group.mclass]);
  },
  mathmlBuilder: function mathmlBuilder(group) {
    var children = [];

    if (group.delim !== ".") {
      children.push(buildMathML_makeText(group.delim, group.mode));
    }

    var node = new mathMLTree.MathNode("mo", children);

    if (group.mclass === "mopen" || group.mclass === "mclose") {
      // Only some of the delimsizing functions act as fences, and they
      // return "mopen" or "mclose" mclass.
      node.setAttribute("fence", "true");
    } else {
      // Explicitly disable fencing if it's not a fence, to override the
      // defaults.
      node.setAttribute("fence", "false");
    }

    return node;
  }
});

function assertParsed(group) {
  if (!group.body) {
    throw new Error("Bug: The leftright ParseNode wasn't fully parsed.");
  }
}

defineFunction({
  type: "leftright-right",
  names: ["\\right"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    // \left case below triggers parsing of \right in
    //   `const right = parser.parseFunction();`
    // uses this return value.
    return {
      type: "leftright-right",
      mode: context.parser.mode,
      delim: checkDelimiter(args[0], context).text
    };
  }
});
defineFunction({
  type: "leftright",
  names: ["\\left"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    var delim = checkDelimiter(args[0], context);
    var parser = context.parser; // Parse out the implicit body

    ++parser.leftrightDepth; // parseExpression stops before '\\right'

    var body = parser.parseExpression(false);
    --parser.leftrightDepth; // Check the next token

    parser.expect("\\right", false);
    var right = assertNodeType(parser.parseFunction(), "leftright-right");
    return {
      type: "leftright",
      mode: parser.mode,
      body: body,
      left: delim.text,
      right: right.delim
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    assertParsed(group); // Build the inner expression

    var inner = buildHTML_buildExpression(group.body, options, true, ["mopen", "mclose"]);
    var innerHeight = 0;
    var innerDepth = 0;
    var hadMiddle = false; // Calculate its height and depth

    for (var i = 0; i < inner.length; i++) {
      // Property `isMiddle` not defined on `span`. See comment in
      // "middle"'s htmlBuilder.
      // $FlowFixMe
      if (inner[i].isMiddle) {
        hadMiddle = true;
      } else {
        innerHeight = Math.max(inner[i].height, innerHeight);
        innerDepth = Math.max(inner[i].depth, innerDepth);
      }
    } // The size of delimiters is the same, regardless of what style we are
    // in. Thus, to correctly calculate the size of delimiter we need around
    // a group, we scale down the inner size based on the size.


    innerHeight *= options.sizeMultiplier;
    innerDepth *= options.sizeMultiplier;
    var leftDelim;

    if (group.left === ".") {
      // Empty delimiters in \left and \right make null delimiter spaces.
      leftDelim = makeNullDelimiter(options, ["mopen"]);
    } else {
      // Otherwise, use leftRightDelim to generate the correct sized
      // delimiter.
      leftDelim = delimiter.leftRightDelim(group.left, innerHeight, innerDepth, options, group.mode, ["mopen"]);
    } // Add it to the beginning of the expression


    inner.unshift(leftDelim); // Handle middle delimiters

    if (hadMiddle) {
      for (var _i = 1; _i < inner.length; _i++) {
        var middleDelim = inner[_i]; // Property `isMiddle` not defined on `span`. See comment in
        // "middle"'s htmlBuilder.
        // $FlowFixMe

        var isMiddle = middleDelim.isMiddle;

        if (isMiddle) {
          // Apply the options that were active when \middle was called
          inner[_i] = delimiter.leftRightDelim(isMiddle.delim, innerHeight, innerDepth, isMiddle.options, group.mode, []);
        }
      }
    }

    var rightDelim; // Same for the right delimiter

    if (group.right === ".") {
      rightDelim = makeNullDelimiter(options, ["mclose"]);
    } else {
      rightDelim = delimiter.leftRightDelim(group.right, innerHeight, innerDepth, options, group.mode, ["mclose"]);
    } // Add it to the end of the expression.


    inner.push(rightDelim);
    return buildCommon.makeSpan(["minner"], inner, options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    assertParsed(group);
    var inner = buildMathML_buildExpression(group.body, options);

    if (group.left !== ".") {
      var leftNode = new mathMLTree.MathNode("mo", [buildMathML_makeText(group.left, group.mode)]);
      leftNode.setAttribute("fence", "true");
      inner.unshift(leftNode);
    }

    if (group.right !== ".") {
      var rightNode = new mathMLTree.MathNode("mo", [buildMathML_makeText(group.right, group.mode)]);
      rightNode.setAttribute("fence", "true");
      inner.push(rightNode);
    }

    return buildMathML_makeRow(inner);
  }
});
defineFunction({
  type: "middle",
  names: ["\\middle"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    var delim = checkDelimiter(args[0], context);

    if (!context.parser.leftrightDepth) {
      throw new src_ParseError("\\middle without preceding \\left", delim);
    }

    return {
      type: "middle",
      mode: context.parser.mode,
      delim: delim.text
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var middleDelim;

    if (group.delim === ".") {
      middleDelim = makeNullDelimiter(options, []);
    } else {
      middleDelim = delimiter.sizedDelim(group.delim, 1, options, group.mode, []);
      var isMiddle = {
        delim: group.delim,
        options: options
      }; // Property `isMiddle` not defined on `span`. It is only used in
      // this file above.
      // TODO: Fix this violation of the `span` type and possibly rename
      // things since `isMiddle` sounds like a boolean, but is a struct.
      // $FlowFixMe

      middleDelim.isMiddle = isMiddle;
    }

    return middleDelim;
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    // A Firefox \middle will strech a character vertically only if it
    // is in the fence part of the operator dictionary at:
    // https://www.w3.org/TR/MathML3/appendixc.html.
    // So we need to avoid U+2223 and use plain "|" instead.
    var textNode = group.delim === "\\vert" || group.delim === "|" ? buildMathML_makeText("|", "text") : buildMathML_makeText(group.delim, group.mode);
    var middleNode = new mathMLTree.MathNode("mo", [textNode]);
    middleNode.setAttribute("fence", "true"); // MathML gives 5/18em spacing to each <mo> element.
    // \middle should get delimiter spacing instead.

    middleNode.setAttribute("lspace", "0.05em");
    middleNode.setAttribute("rspace", "0.05em");
    return middleNode;
  }
});
// CONCATENATED MODULE: ./src/functions/enclose.js









var enclose_htmlBuilder = function htmlBuilder(group, options) {
  // \cancel, \bcancel, \xcancel, \sout, \fbox, \colorbox, \fcolorbox
  // Some groups can return document fragments.  Handle those by wrapping
  // them in a span.
  var inner = buildCommon.wrapFragment(buildHTML_buildGroup(group.body, options), options);
  var label = group.label.substr(1);
  var scale = options.sizeMultiplier;
  var img;
  var imgShift = 0; // In the LaTeX cancel package, line geometry is slightly different
  // depending on whether the subject is wider than it is tall, or vice versa.
  // We don't know the width of a group, so as a proxy, we test if
  // the subject is a single character. This captures most of the
  // subjects that should get the "tall" treatment.

  var isSingleChar = utils.isCharacterBox(group.body);

  if (label === "sout") {
    img = buildCommon.makeSpan(["stretchy", "sout"]);
    img.height = options.fontMetrics().defaultRuleThickness / scale;
    imgShift = -0.5 * options.fontMetrics().xHeight;
  } else {
    // Add horizontal padding
    if (/cancel/.test(label)) {
      if (!isSingleChar) {
        inner.classes.push("cancel-pad");
      }
    } else {
      inner.classes.push("boxpad");
    } // Add vertical padding


    var vertPad = 0; // ref: LaTeX source2e: \fboxsep = 3pt;  \fboxrule = .4pt
    // ref: cancel package: \advance\totalheight2\p@ % "+2"

    if (/box/.test(label)) {
      vertPad = label === "colorbox" ? 0.3 : 0.34;
    } else {
      vertPad = isSingleChar ? 0.2 : 0;
    }

    img = stretchy.encloseSpan(inner, label, vertPad, options);
    imgShift = inner.depth + vertPad;

    if (group.backgroundColor) {
      img.style.backgroundColor = group.backgroundColor;

      if (group.borderColor) {
        img.style.borderColor = group.borderColor;
      }
    }
  }

  var vlist;

  if (group.backgroundColor) {
    vlist = buildCommon.makeVList({
      positionType: "individualShift",
      children: [// Put the color background behind inner;
      {
        type: "elem",
        elem: img,
        shift: imgShift
      }, {
        type: "elem",
        elem: inner,
        shift: 0
      }]
    }, options);
  } else {
    vlist = buildCommon.makeVList({
      positionType: "individualShift",
      children: [// Write the \cancel stroke on top of inner.
      {
        type: "elem",
        elem: inner,
        shift: 0
      }, {
        type: "elem",
        elem: img,
        shift: imgShift,
        wrapperClasses: /cancel/.test(label) ? ["svg-align"] : []
      }]
    }, options);
  }

  if (/cancel/.test(label)) {
    // The cancel package documentation says that cancel lines add their height
    // to the expression, but tests show that isn't how it actually works.
    vlist.height = inner.height;
    vlist.depth = inner.depth;
  }

  if (/cancel/.test(label) && !isSingleChar) {
    // cancel does not create horiz space for its line extension.
    return buildCommon.makeSpan(["mord", "cancel-lap"], [vlist], options);
  } else {
    return buildCommon.makeSpan(["mord"], [vlist], options);
  }
};

var enclose_mathmlBuilder = function mathmlBuilder(group, options) {
  var node = new mathMLTree.MathNode(group.label.indexOf("colorbox") > -1 ? "mpadded" : "menclose", [buildMathML_buildGroup(group.body, options)]);

  switch (group.label) {
    case "\\cancel":
      node.setAttribute("notation", "updiagonalstrike");
      break;

    case "\\bcancel":
      node.setAttribute("notation", "downdiagonalstrike");
      break;

    case "\\sout":
      node.setAttribute("notation", "horizontalstrike");
      break;

    case "\\fbox":
      node.setAttribute("notation", "box");
      break;

    case "\\fcolorbox":
    case "\\colorbox":
      // <menclose> doesn't have a good notation option. So use <mpadded>
      // instead. Set some attributes that come included with <menclose>.
      node.setAttribute("width", "+6pt");
      node.setAttribute("height", "+6pt");
      node.setAttribute("lspace", "3pt"); // LaTeX source2e: \fboxsep = 3pt

      node.setAttribute("voffset", "3pt");

      if (group.label === "\\fcolorbox") {
        var thk = options.fontMetrics().defaultRuleThickness;
        node.setAttribute("style", "border: " + thk + "em solid " + String(group.borderColor));
      }

      break;

    case "\\xcancel":
      node.setAttribute("notation", "updiagonalstrike downdiagonalstrike");
      break;
  }

  if (group.backgroundColor) {
    node.setAttribute("mathbackground", group.backgroundColor);
  }

  return node;
};

defineFunction({
  type: "enclose",
  names: ["\\colorbox"],
  props: {
    numArgs: 2,
    allowedInText: true,
    greediness: 3,
    argTypes: ["color", "text"]
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var color = assertNodeType(args[0], "color-token").color;
    var body = args[1];
    return {
      type: "enclose",
      mode: parser.mode,
      label: funcName,
      backgroundColor: color,
      body: body
    };
  },
  htmlBuilder: enclose_htmlBuilder,
  mathmlBuilder: enclose_mathmlBuilder
});
defineFunction({
  type: "enclose",
  names: ["\\fcolorbox"],
  props: {
    numArgs: 3,
    allowedInText: true,
    greediness: 3,
    argTypes: ["color", "color", "text"]
  },
  handler: function handler(_ref2, args, optArgs) {
    var parser = _ref2.parser,
        funcName = _ref2.funcName;
    var borderColor = assertNodeType(args[0], "color-token").color;
    var backgroundColor = assertNodeType(args[1], "color-token").color;
    var body = args[2];
    return {
      type: "enclose",
      mode: parser.mode,
      label: funcName,
      backgroundColor: backgroundColor,
      borderColor: borderColor,
      body: body
    };
  },
  htmlBuilder: enclose_htmlBuilder,
  mathmlBuilder: enclose_mathmlBuilder
});
defineFunction({
  type: "enclose",
  names: ["\\fbox"],
  props: {
    numArgs: 1,
    argTypes: ["text"],
    allowedInText: true
  },
  handler: function handler(_ref3, args) {
    var parser = _ref3.parser;
    return {
      type: "enclose",
      mode: parser.mode,
      label: "\\fbox",
      body: args[0]
    };
  }
});
defineFunction({
  type: "enclose",
  names: ["\\cancel", "\\bcancel", "\\xcancel", "\\sout"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref4, args, optArgs) {
    var parser = _ref4.parser,
        funcName = _ref4.funcName;
    var body = args[0];
    return {
      type: "enclose",
      mode: parser.mode,
      label: funcName,
      body: body
    };
  },
  htmlBuilder: enclose_htmlBuilder,
  mathmlBuilder: enclose_mathmlBuilder
});
// CONCATENATED MODULE: ./src/defineEnvironment.js


/**
 * All registered environments.
 * `environments.js` exports this same dictionary again and makes it public.
 * `Parser.js` requires this dictionary via `environments.js`.
 */
var _environments = {};
function defineEnvironment(_ref) {
  var type = _ref.type,
      names = _ref.names,
      props = _ref.props,
      handler = _ref.handler,
      htmlBuilder = _ref.htmlBuilder,
      mathmlBuilder = _ref.mathmlBuilder;
  // Set default values of environments.
  var data = {
    type: type,
    numArgs: props.numArgs || 0,
    greediness: 1,
    allowedInText: false,
    numOptionalArgs: 0,
    handler: handler
  };

  for (var i = 0; i < names.length; ++i) {
    // TODO: The value type of _environments should be a type union of all
    // possible `EnvSpec<>` possibilities instead of `EnvSpec<*>`, which is
    // an existential type.
    // $FlowFixMe
    _environments[names[i]] = data;
  }

  if (htmlBuilder) {
    _htmlGroupBuilders[type] = htmlBuilder;
  }

  if (mathmlBuilder) {
    _mathmlGroupBuilders[type] = mathmlBuilder;
  }
}
// CONCATENATED MODULE: ./src/environments/array.js












function getHLines(parser) {
  // Return an array. The array length = number of hlines.
  // Each element in the array tells if the line is dashed.
  var hlineInfo = [];
  parser.consumeSpaces();
  var nxt = parser.nextToken.text;

  while (nxt === "\\hline" || nxt === "\\hdashline") {
    parser.consume();
    hlineInfo.push(nxt === "\\hdashline");
    parser.consumeSpaces();
    nxt = parser.nextToken.text;
  }

  return hlineInfo;
}
/**
 * Parse the body of the environment, with rows delimited by \\ and
 * columns delimited by &, and create a nested list in row-major order
 * with one group per cell.  If given an optional argument style
 * ("text", "display", etc.), then each cell is cast into that style.
 */


function parseArray(parser, _ref, style) {
  var hskipBeforeAndAfter = _ref.hskipBeforeAndAfter,
      addJot = _ref.addJot,
      cols = _ref.cols,
      arraystretch = _ref.arraystretch,
      colSeparationType = _ref.colSeparationType;
  // Parse body of array with \\ temporarily mapped to \cr
  parser.gullet.beginGroup();
  parser.gullet.macros.set("\\\\", "\\cr"); // Get current arraystretch if it's not set by the environment

  if (!arraystretch) {
    var stretch = parser.gullet.expandMacroAsText("\\arraystretch");

    if (stretch == null) {
      // Default \arraystretch from lttab.dtx
      arraystretch = 1;
    } else {
      arraystretch = parseFloat(stretch);

      if (!arraystretch || arraystretch < 0) {
        throw new src_ParseError("Invalid \\arraystretch: " + stretch);
      }
    }
  }

  var row = [];
  var body = [row];
  var rowGaps = [];
  var hLinesBeforeRow = []; // Test for \hline at the top of the array.

  hLinesBeforeRow.push(getHLines(parser));

  while (true) {
    // eslint-disable-line no-constant-condition
    var cell = parser.parseExpression(false, "\\cr");
    cell = {
      type: "ordgroup",
      mode: parser.mode,
      body: cell
    };

    if (style) {
      cell = {
        type: "styling",
        mode: parser.mode,
        style: style,
        body: [cell]
      };
    }

    row.push(cell);
    var next = parser.nextToken.text;

    if (next === "&") {
      parser.consume();
    } else if (next === "\\end") {
      // Arrays terminate newlines with `\crcr` which consumes a `\cr` if
      // the last line is empty.
      // NOTE: Currently, `cell` is the last item added into `row`.
      if (row.length === 1 && cell.type === "styling" && cell.body[0].body.length === 0) {
        body.pop();
      }

      if (hLinesBeforeRow.length < body.length + 1) {
        hLinesBeforeRow.push([]);
      }

      break;
    } else if (next === "\\cr") {
      var cr = assertNodeType(parser.parseFunction(), "cr");
      rowGaps.push(cr.size); // check for \hline(s) following the row separator

      hLinesBeforeRow.push(getHLines(parser));
      row = [];
      body.push(row);
    } else {
      throw new src_ParseError("Expected & or \\\\ or \\cr or \\end", parser.nextToken);
    }
  }

  parser.gullet.endGroup();
  return {
    type: "array",
    mode: parser.mode,
    addJot: addJot,
    arraystretch: arraystretch,
    body: body,
    cols: cols,
    rowGaps: rowGaps,
    hskipBeforeAndAfter: hskipBeforeAndAfter,
    hLinesBeforeRow: hLinesBeforeRow,
    colSeparationType: colSeparationType
  };
} // Decides on a style for cells in an array according to whether the given
// environment name starts with the letter 'd'.


function dCellStyle(envName) {
  if (envName.substr(0, 1) === "d") {
    return "display";
  } else {
    return "text";
  }
}

var array_htmlBuilder = function htmlBuilder(group, options) {
  var r;
  var c;
  var nr = group.body.length;
  var hLinesBeforeRow = group.hLinesBeforeRow;
  var nc = 0;
  var body = new Array(nr);
  var hlines = []; // Horizontal spacing

  var pt = 1 / options.fontMetrics().ptPerEm;
  var arraycolsep = 5 * pt; // \arraycolsep in article.cls
  // Vertical spacing

  var baselineskip = 12 * pt; // see size10.clo
  // Default \jot from ltmath.dtx
  // TODO(edemaine): allow overriding \jot via \setlength (#687)

  var jot = 3 * pt;
  var arrayskip = group.arraystretch * baselineskip;
  var arstrutHeight = 0.7 * arrayskip; // \strutbox in ltfsstrc.dtx and

  var arstrutDepth = 0.3 * arrayskip; // \@arstrutbox in lttab.dtx

  var totalHeight = 0; // Set a position for \hline(s) at the top of the array, if any.

  function setHLinePos(hlinesInGap) {
    for (var i = 0; i < hlinesInGap.length; ++i) {
      if (i > 0) {
        totalHeight += 0.25;
      }

      hlines.push({
        pos: totalHeight,
        isDashed: hlinesInGap[i]
      });
    }
  }

  setHLinePos(hLinesBeforeRow[0]);

  for (r = 0; r < group.body.length; ++r) {
    var inrow = group.body[r];
    var height = arstrutHeight; // \@array adds an \@arstrut

    var depth = arstrutDepth; // to each tow (via the template)

    if (nc < inrow.length) {
      nc = inrow.length;
    }

    var outrow = new Array(inrow.length);

    for (c = 0; c < inrow.length; ++c) {
      var elt = buildHTML_buildGroup(inrow[c], options);

      if (depth < elt.depth) {
        depth = elt.depth;
      }

      if (height < elt.height) {
        height = elt.height;
      }

      outrow[c] = elt;
    }

    var rowGap = group.rowGaps[r];
    var gap = 0;

    if (rowGap) {
      gap = units_calculateSize(rowGap, options);

      if (gap > 0) {
        // \@argarraycr
        gap += arstrutDepth;

        if (depth < gap) {
          depth = gap; // \@xargarraycr
        }

        gap = 0;
      }
    } // In AMS multiline environments such as aligned and gathered, rows
    // correspond to lines that have additional \jot added to the
    // \baselineskip via \openup.


    if (group.addJot) {
      depth += jot;
    }

    outrow.height = height;
    outrow.depth = depth;
    totalHeight += height;
    outrow.pos = totalHeight;
    totalHeight += depth + gap; // \@yargarraycr

    body[r] = outrow; // Set a position for \hline(s), if any.

    setHLinePos(hLinesBeforeRow[r + 1]);
  }

  var offset = totalHeight / 2 + options.fontMetrics().axisHeight;
  var colDescriptions = group.cols || [];
  var cols = [];
  var colSep;
  var colDescrNum;

  for (c = 0, colDescrNum = 0; // Continue while either there are more columns or more column
  // descriptions, so trailing separators don't get lost.
  c < nc || colDescrNum < colDescriptions.length; ++c, ++colDescrNum) {
    var colDescr = colDescriptions[colDescrNum] || {};
    var firstSeparator = true;

    while (colDescr.type === "separator") {
      // If there is more than one separator in a row, add a space
      // between them.
      if (!firstSeparator) {
        colSep = buildCommon.makeSpan(["arraycolsep"], []);
        colSep.style.width = options.fontMetrics().doubleRuleSep + "em";
        cols.push(colSep);
      }

      if (colDescr.separator === "|") {
        var separator = buildCommon.makeSpan(["vertical-separator"], [], options);
        separator.style.height = totalHeight + "em";
        separator.style.verticalAlign = -(totalHeight - offset) + "em";
        cols.push(separator);
      } else if (colDescr.separator === ":") {
        var _separator = buildCommon.makeSpan(["vertical-separator", "vs-dashed"], [], options);

        _separator.style.height = totalHeight + "em";
        _separator.style.verticalAlign = -(totalHeight - offset) + "em";
        cols.push(_separator);
      } else {
        throw new src_ParseError("Invalid separator type: " + colDescr.separator);
      }

      colDescrNum++;
      colDescr = colDescriptions[colDescrNum] || {};
      firstSeparator = false;
    }

    if (c >= nc) {
      continue;
    }

    var sepwidth = void 0;

    if (c > 0 || group.hskipBeforeAndAfter) {
      sepwidth = utils.deflt(colDescr.pregap, arraycolsep);

      if (sepwidth !== 0) {
        colSep = buildCommon.makeSpan(["arraycolsep"], []);
        colSep.style.width = sepwidth + "em";
        cols.push(colSep);
      }
    }

    var col = [];

    for (r = 0; r < nr; ++r) {
      var row = body[r];
      var elem = row[c];

      if (!elem) {
        continue;
      }

      var shift = row.pos - offset;
      elem.depth = row.depth;
      elem.height = row.height;
      col.push({
        type: "elem",
        elem: elem,
        shift: shift
      });
    }

    col = buildCommon.makeVList({
      positionType: "individualShift",
      children: col
    }, options);
    col = buildCommon.makeSpan(["col-align-" + (colDescr.align || "c")], [col]);
    cols.push(col);

    if (c < nc - 1 || group.hskipBeforeAndAfter) {
      sepwidth = utils.deflt(colDescr.postgap, arraycolsep);

      if (sepwidth !== 0) {
        colSep = buildCommon.makeSpan(["arraycolsep"], []);
        colSep.style.width = sepwidth + "em";
        cols.push(colSep);
      }
    }
  }

  body = buildCommon.makeSpan(["mtable"], cols); // Add \hline(s), if any.

  if (hlines.length > 0) {
    var line = buildCommon.makeLineSpan("hline", options, 0.05);
    var dashes = buildCommon.makeLineSpan("hdashline", options, 0.05);
    var vListElems = [{
      type: "elem",
      elem: body,
      shift: 0
    }];

    while (hlines.length > 0) {
      var hline = hlines.pop();
      var lineShift = hline.pos - offset;

      if (hline.isDashed) {
        vListElems.push({
          type: "elem",
          elem: dashes,
          shift: lineShift
        });
      } else {
        vListElems.push({
          type: "elem",
          elem: line,
          shift: lineShift
        });
      }
    }

    body = buildCommon.makeVList({
      positionType: "individualShift",
      children: vListElems
    }, options);
  }

  return buildCommon.makeSpan(["mord"], [body], options);
};

var alignMap = {
  c: "center ",
  l: "left ",
  r: "right "
};

var array_mathmlBuilder = function mathmlBuilder(group, options) {
  var table = new mathMLTree.MathNode("mtable", group.body.map(function (row) {
    return new mathMLTree.MathNode("mtr", row.map(function (cell) {
      return new mathMLTree.MathNode("mtd", [buildMathML_buildGroup(cell, options)]);
    }));
  })); // Set column alignment, row spacing, column spacing, and
  // array lines by setting attributes on the table element.
  // Set the row spacing. In MathML, we specify a gap distance.
  // We do not use rowGap[] because MathML automatically increases
  // cell height with the height/depth of the element content.
  // LaTeX \arraystretch multiplies the row baseline-to-baseline distance.
  // We simulate this by adding (arraystretch - 1)em to the gap. This
  // does a reasonable job of adjusting arrays containing 1 em tall content.
  // The 0.16 and 0.09 values are found emprically. They produce an array
  // similar to LaTeX and in which content does not interfere with \hines.

  var gap = 0.16 + group.arraystretch - 1 + (group.addJot ? 0.09 : 0);
  table.setAttribute("rowspacing", gap + "em"); // MathML table lines go only between cells.
  // To place a line on an edge we'll use <menclose>, if necessary.

  var menclose = "";
  var align = "";

  if (group.cols) {
    // Find column alignment, column spacing, and  vertical lines.
    var cols = group.cols;
    var columnLines = "";
    var prevTypeWasAlign = false;
    var iStart = 0;
    var iEnd = cols.length;

    if (cols[0].type === "separator") {
      menclose += "top ";
      iStart = 1;
    }

    if (cols[cols.length - 1].type === "separator") {
      menclose += "bottom ";
      iEnd -= 1;
    }

    for (var i = iStart; i < iEnd; i++) {
      if (cols[i].type === "align") {
        align += alignMap[cols[i].align];

        if (prevTypeWasAlign) {
          columnLines += "none ";
        }

        prevTypeWasAlign = true;
      } else if (cols[i].type === "separator") {
        // MathML accepts only single lines between cells.
        // So we read only the first of consecutive separators.
        if (prevTypeWasAlign) {
          columnLines += cols[i].separator === "|" ? "solid " : "dashed ";
          prevTypeWasAlign = false;
        }
      }
    }

    table.setAttribute("columnalign", align.trim());

    if (/[sd]/.test(columnLines)) {
      table.setAttribute("columnlines", columnLines.trim());
    }
  } // Set column spacing.


  if (group.colSeparationType === "align") {
    var _cols = group.cols || [];

    var spacing = "";

    for (var _i = 1; _i < _cols.length; _i++) {
      spacing += _i % 2 ? "0em " : "1em ";
    }

    table.setAttribute("columnspacing", spacing.trim());
  } else if (group.colSeparationType === "alignat") {
    table.setAttribute("columnspacing", "0em");
  } else {
    table.setAttribute("columnspacing", "1em");
  } // Address \hline and \hdashline


  var rowLines = "";
  var hlines = group.hLinesBeforeRow;
  menclose += hlines[0].length > 0 ? "left " : "";
  menclose += hlines[hlines.length - 1].length > 0 ? "right " : "";

  for (var _i2 = 1; _i2 < hlines.length - 1; _i2++) {
    rowLines += hlines[_i2].length === 0 ? "none " // MathML accepts only a single line between rows. Read one element.
    : hlines[_i2][0] ? "dashed " : "solid ";
  }

  if (/[sd]/.test(rowLines)) {
    table.setAttribute("rowlines", rowLines.trim());
  }

  if (menclose === "") {
    return table;
  } else {
    var wrapper = new mathMLTree.MathNode("menclose", [table]);
    wrapper.setAttribute("notation", menclose.trim());
    return wrapper;
  }
}; // Convenience function for aligned and alignedat environments.


var array_alignedHandler = function alignedHandler(context, args) {
  var cols = [];
  var res = parseArray(context.parser, {
    cols: cols,
    addJot: true
  }, "display"); // Determining number of columns.
  // 1. If the first argument is given, we use it as a number of columns,
  //    and makes sure that each row doesn't exceed that number.
  // 2. Otherwise, just count number of columns = maximum number
  //    of cells in each row ("aligned" mode -- isAligned will be true).
  //
  // At the same time, prepend empty group {} at beginning of every second
  // cell in each row (starting with second cell) so that operators become
  // binary.  This behavior is implemented in amsmath's \start@aligned.

  var numMaths;
  var numCols = 0;
  var emptyGroup = {
    type: "ordgroup",
    mode: context.mode,
    body: []
  };
  var ordgroup = checkNodeType(args[0], "ordgroup");

  if (ordgroup) {
    var arg0 = "";

    for (var i = 0; i < ordgroup.body.length; i++) {
      var textord = assertNodeType(ordgroup.body[i], "textord");
      arg0 += textord.text;
    }

    numMaths = Number(arg0);
    numCols = numMaths * 2;
  }

  var isAligned = !numCols;
  res.body.forEach(function (row) {
    for (var _i3 = 1; _i3 < row.length; _i3 += 2) {
      // Modify ordgroup node within styling node
      var styling = assertNodeType(row[_i3], "styling");

      var _ordgroup = assertNodeType(styling.body[0], "ordgroup");

      _ordgroup.body.unshift(emptyGroup);
    }

    if (!isAligned) {
      // Case 1
      var curMaths = row.length / 2;

      if (numMaths < curMaths) {
        throw new src_ParseError("Too many math in a row: " + ("expected " + numMaths + ", but got " + curMaths), row[0]);
      }
    } else if (numCols < row.length) {
      // Case 2
      numCols = row.length;
    }
  }); // Adjusting alignment.
  // In aligned mode, we add one \qquad between columns;
  // otherwise we add nothing.

  for (var _i4 = 0; _i4 < numCols; ++_i4) {
    var align = "r";
    var pregap = 0;

    if (_i4 % 2 === 1) {
      align = "l";
    } else if (_i4 > 0 && isAligned) {
      // "aligned" mode.
      pregap = 1; // add one \quad
    }

    cols[_i4] = {
      type: "align",
      align: align,
      pregap: pregap,
      postgap: 0
    };
  }

  res.colSeparationType = isAligned ? "align" : "alignat";
  return res;
}; // Arrays are part of LaTeX, defined in lttab.dtx so its documentation
// is part of the source2e.pdf file of LaTeX2e source documentation.
// {darray} is an {array} environment where cells are set in \displaystyle,
// as defined in nccmath.sty.


defineEnvironment({
  type: "array",
  names: ["array", "darray"],
  props: {
    numArgs: 1
  },
  handler: function handler(context, args) {
    // Since no types are specified above, the two possibilities are
    // - The argument is wrapped in {} or [], in which case Parser's
    //   parseGroup() returns an "ordgroup" wrapping some symbol node.
    // - The argument is a bare symbol node.
    var symNode = checkSymbolNodeType(args[0]);
    var colalign = symNode ? [args[0]] : assertNodeType(args[0], "ordgroup").body;
    var cols = colalign.map(function (nde) {
      var node = assertSymbolNodeType(nde);
      var ca = node.text;

      if ("lcr".indexOf(ca) !== -1) {
        return {
          type: "align",
          align: ca
        };
      } else if (ca === "|") {
        return {
          type: "separator",
          separator: "|"
        };
      } else if (ca === ":") {
        return {
          type: "separator",
          separator: ":"
        };
      }

      throw new src_ParseError("Unknown column alignment: " + ca, nde);
    });
    var res = {
      cols: cols,
      hskipBeforeAndAfter: true // \@preamble in lttab.dtx

    };
    return parseArray(context.parser, res, dCellStyle(context.envName));
  },
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // The matrix environments of amsmath builds on the array environment
// of LaTeX, which is discussed above.

defineEnvironment({
  type: "array",
  names: ["matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix"],
  props: {
    numArgs: 0
  },
  handler: function handler(context) {
    var delimiters = {
      "matrix": null,
      "pmatrix": ["(", ")"],
      "bmatrix": ["[", "]"],
      "Bmatrix": ["\\{", "\\}"],
      "vmatrix": ["|", "|"],
      "Vmatrix": ["\\Vert", "\\Vert"]
    }[context.envName]; // \hskip -\arraycolsep in amsmath

    var payload = {
      hskipBeforeAndAfter: false
    };
    var res = parseArray(context.parser, payload, dCellStyle(context.envName));
    return delimiters ? {
      type: "leftright",
      mode: context.mode,
      body: [res],
      left: delimiters[0],
      right: delimiters[1]
    } : res;
  },
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // A cases environment (in amsmath.sty) is almost equivalent to
// \def\arraystretch{1.2}%
// \left\{\begin{array}{@{}l@{\quad}l@{}} … \end{array}\right.
// {dcases} is a {cases} environment where cells are set in \displaystyle,
// as defined in mathtools.sty.

defineEnvironment({
  type: "array",
  names: ["cases", "dcases"],
  props: {
    numArgs: 0
  },
  handler: function handler(context) {
    var payload = {
      arraystretch: 1.2,
      cols: [{
        type: "align",
        align: "l",
        pregap: 0,
        // TODO(kevinb) get the current style.
        // For now we use the metrics for TEXT style which is what we were
        // doing before.  Before attempting to get the current style we
        // should look at TeX's behavior especially for \over and matrices.
        postgap: 1.0
        /* 1em quad */

      }, {
        type: "align",
        align: "l",
        pregap: 0,
        postgap: 0
      }]
    };
    var res = parseArray(context.parser, payload, dCellStyle(context.envName));
    return {
      type: "leftright",
      mode: context.mode,
      body: [res],
      left: "\\{",
      right: "."
    };
  },
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // An aligned environment is like the align* environment
// except it operates within math mode.
// Note that we assume \nomallineskiplimit to be zero,
// so that \strut@ is the same as \strut.

defineEnvironment({
  type: "array",
  names: ["aligned"],
  props: {
    numArgs: 0
  },
  handler: array_alignedHandler,
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // A gathered environment is like an array environment with one centered
// column, but where rows are considered lines so get \jot line spacing
// and contents are set in \displaystyle.

defineEnvironment({
  type: "array",
  names: ["gathered"],
  props: {
    numArgs: 0
  },
  handler: function handler(context) {
    var res = {
      cols: [{
        type: "align",
        align: "c"
      }],
      addJot: true
    };
    return parseArray(context.parser, res, "display");
  },
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // alignat environment is like an align environment, but one must explicitly
// specify maximum number of columns in each row, and can adjust spacing between
// each columns.

defineEnvironment({
  type: "array",
  names: ["alignedat"],
  // One for numbered and for unnumbered;
  // but, KaTeX doesn't supports math numbering yet,
  // they make no difference for now.
  props: {
    numArgs: 1
  },
  handler: array_alignedHandler,
  htmlBuilder: array_htmlBuilder,
  mathmlBuilder: array_mathmlBuilder
}); // Catch \hline outside array environment

defineFunction({
  type: "text",
  // Doesn't matter what this is.
  names: ["\\hline", "\\hdashline"],
  props: {
    numArgs: 0,
    allowedInText: true,
    allowedInMath: true
  },
  handler: function handler(context, args) {
    throw new src_ParseError(context.funcName + " valid only within array environment");
  }
});
// CONCATENATED MODULE: ./src/environments.js

var environments = _environments;
/* harmony default export */ var src_environments = (environments); // All environment definitions should be imported below


// CONCATENATED MODULE: ./src/functions/environment.js



 // Environment delimiters. HTML/MathML rendering is defined in the corresponding
// defineEnvironment definitions.
// $FlowFixMe, "environment" handler returns an environment ParseNode

defineFunction({
  type: "environment",
  names: ["\\begin", "\\end"],
  props: {
    numArgs: 1,
    argTypes: ["text"]
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var nameGroup = args[0];

    if (nameGroup.type !== "ordgroup") {
      throw new src_ParseError("Invalid environment name", nameGroup);
    }

    var envName = "";

    for (var i = 0; i < nameGroup.body.length; ++i) {
      envName += assertNodeType(nameGroup.body[i], "textord").text;
    }

    if (funcName === "\\begin") {
      // begin...end is similar to left...right
      if (!src_environments.hasOwnProperty(envName)) {
        throw new src_ParseError("No such environment: " + envName, nameGroup);
      } // Build the environment object. Arguments and other information will
      // be made available to the begin and end methods using properties.


      var env = src_environments[envName];

      var _parser$parseArgument = parser.parseArguments("\\begin{" + envName + "}", env),
          _args = _parser$parseArgument.args,
          optArgs = _parser$parseArgument.optArgs;

      var context = {
        mode: parser.mode,
        envName: envName,
        parser: parser
      };
      var result = env.handler(context, _args, optArgs);
      parser.expect("\\end", false);
      var endNameToken = parser.nextToken;
      var end = assertNodeType(parser.parseFunction(), "environment");

      if (end.name !== envName) {
        throw new src_ParseError("Mismatch: \\begin{" + envName + "} matched by \\end{" + end.name + "}", endNameToken);
      }

      return result;
    }

    return {
      type: "environment",
      mode: parser.mode,
      name: envName,
      nameGroup: nameGroup
    };
  }
});
// CONCATENATED MODULE: ./src/functions/mclass.js





var mclass_makeSpan = buildCommon.makeSpan;

function mclass_htmlBuilder(group, options) {
  var elements = buildHTML_buildExpression(group.body, options, true);
  return mclass_makeSpan([group.mclass], elements, options);
}

function mclass_mathmlBuilder(group, options) {
  var inner = buildMathML_buildExpression(group.body, options);
  return mathMLTree.newDocumentFragment(inner);
} // Math class commands except \mathop


defineFunction({
  type: "mclass",
  names: ["\\mathord", "\\mathbin", "\\mathrel", "\\mathopen", "\\mathclose", "\\mathpunct", "\\mathinner"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var body = args[0];
    return {
      type: "mclass",
      mode: parser.mode,
      mclass: "m" + funcName.substr(5),
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: mclass_htmlBuilder,
  mathmlBuilder: mclass_mathmlBuilder
});
var binrelClass = function binrelClass(arg) {
  // \binrel@ spacing varies with (bin|rel|ord) of the atom in the argument.
  // (by rendering separately and with {}s before and after, and measuring
  // the change in spacing).  We'll do roughly the same by detecting the
  // atom type directly.
  var atom = arg.type === "ordgroup" && arg.body.length ? arg.body[0] : arg;

  if (atom.type === "atom" && (atom.family === "bin" || atom.family === "rel")) {
    return "m" + atom.family;
  } else {
    return "mord";
  }
}; // \@binrel{x}{y} renders like y but as mbin/mrel/mord if x is mbin/mrel/mord.
// This is equivalent to \binrel@{x}\binrel@@{y} in AMSTeX.

defineFunction({
  type: "mclass",
  names: ["\\@binrel"],
  props: {
    numArgs: 2
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser;
    return {
      type: "mclass",
      mode: parser.mode,
      mclass: binrelClass(args[0]),
      body: [args[1]]
    };
  }
}); // Build a relation or stacked op by placing one symbol on top of another

defineFunction({
  type: "mclass",
  names: ["\\stackrel", "\\overset", "\\underset"],
  props: {
    numArgs: 2
  },
  handler: function handler(_ref3, args) {
    var parser = _ref3.parser,
        funcName = _ref3.funcName;
    var baseArg = args[1];
    var shiftedArg = args[0];
    var mclass;

    if (funcName !== "\\stackrel") {
      // LaTeX applies \binrel spacing to \overset and \underset.
      mclass = binrelClass(baseArg);
    } else {
      mclass = "mrel"; // for \stackrel
    }

    var baseOp = {
      type: "op",
      mode: baseArg.mode,
      limits: true,
      alwaysHandleSupSub: true,
      parentIsSupSub: false,
      symbol: false,
      suppressBaseShift: funcName !== "\\stackrel",
      body: defineFunction_ordargument(baseArg)
    };
    var supsub = {
      type: "supsub",
      mode: shiftedArg.mode,
      base: baseOp,
      sup: funcName === "\\underset" ? null : shiftedArg,
      sub: funcName === "\\underset" ? shiftedArg : null
    };
    return {
      type: "mclass",
      mode: parser.mode,
      mclass: mclass,
      body: [supsub]
    };
  },
  htmlBuilder: mclass_htmlBuilder,
  mathmlBuilder: mclass_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/font.js
// TODO(kevinb): implement \\sl and \\sc





var font_htmlBuilder = function htmlBuilder(group, options) {
  var font = group.font;
  var newOptions = options.withFont(font);
  return buildHTML_buildGroup(group.body, newOptions);
};

var font_mathmlBuilder = function mathmlBuilder(group, options) {
  var font = group.font;
  var newOptions = options.withFont(font);
  return buildMathML_buildGroup(group.body, newOptions);
};

var fontAliases = {
  "\\Bbb": "\\mathbb",
  "\\bold": "\\mathbf",
  "\\frak": "\\mathfrak",
  "\\bm": "\\boldsymbol"
};
defineFunction({
  type: "font",
  names: [// styles, except \boldsymbol defined below
  "\\mathrm", "\\mathit", "\\mathbf", "\\mathnormal", // families
  "\\mathbb", "\\mathcal", "\\mathfrak", "\\mathscr", "\\mathsf", "\\mathtt", // aliases, except \bm defined below
  "\\Bbb", "\\bold", "\\frak"],
  props: {
    numArgs: 1,
    greediness: 2
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var body = args[0];
    var func = funcName;

    if (func in fontAliases) {
      func = fontAliases[func];
    }

    return {
      type: "font",
      mode: parser.mode,
      font: func.slice(1),
      body: body
    };
  },
  htmlBuilder: font_htmlBuilder,
  mathmlBuilder: font_mathmlBuilder
});
defineFunction({
  type: "mclass",
  names: ["\\boldsymbol", "\\bm"],
  props: {
    numArgs: 1,
    greediness: 2
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser;
    var body = args[0]; // amsbsy.sty's \boldsymbol uses \binrel spacing to inherit the
    // argument's bin|rel|ord status

    return {
      type: "mclass",
      mode: parser.mode,
      mclass: binrelClass(body),
      body: [{
        type: "font",
        mode: parser.mode,
        font: "boldsymbol",
        body: body
      }]
    };
  }
}); // Old font changing functions

defineFunction({
  type: "font",
  names: ["\\rm", "\\sf", "\\tt", "\\bf", "\\it"],
  props: {
    numArgs: 0,
    allowedInText: true
  },
  handler: function handler(_ref3, args) {
    var parser = _ref3.parser,
        funcName = _ref3.funcName,
        breakOnTokenText = _ref3.breakOnTokenText;
    var mode = parser.mode;
    var body = parser.parseExpression(true, breakOnTokenText);
    var style = "math" + funcName.slice(1);
    return {
      type: "font",
      mode: mode,
      font: style,
      body: {
        type: "ordgroup",
        mode: parser.mode,
        body: body
      }
    };
  },
  htmlBuilder: font_htmlBuilder,
  mathmlBuilder: font_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/genfrac.js











var genfrac_adjustStyle = function adjustStyle(size, originalStyle) {
  // Figure out what style this fraction should be in based on the
  // function used
  var style = originalStyle;

  if (size === "display") {
    // Get display style as a default.
    // If incoming style is sub/sup, use style.text() to get correct size.
    style = style.id >= src_Style.SCRIPT.id ? style.text() : src_Style.DISPLAY;
  } else if (size === "text" && style.size === src_Style.DISPLAY.size) {
    // We're in a \tfrac but incoming style is displaystyle, so:
    style = src_Style.TEXT;
  } else if (size === "script") {
    style = src_Style.SCRIPT;
  } else if (size === "scriptscript") {
    style = src_Style.SCRIPTSCRIPT;
  }

  return style;
};

var genfrac_htmlBuilder = function htmlBuilder(group, options) {
  // Fractions are handled in the TeXbook on pages 444-445, rules 15(a-e).
  var style = genfrac_adjustStyle(group.size, options.style);
  var nstyle = style.fracNum();
  var dstyle = style.fracDen();
  var newOptions;
  newOptions = options.havingStyle(nstyle);
  var numerm = buildHTML_buildGroup(group.numer, newOptions, options);

  if (group.continued) {
    // \cfrac inserts a \strut into the numerator.
    // Get \strut dimensions from TeXbook page 353.
    var hStrut = 8.5 / options.fontMetrics().ptPerEm;
    var dStrut = 3.5 / options.fontMetrics().ptPerEm;
    numerm.height = numerm.height < hStrut ? hStrut : numerm.height;
    numerm.depth = numerm.depth < dStrut ? dStrut : numerm.depth;
  }

  newOptions = options.havingStyle(dstyle);
  var denomm = buildHTML_buildGroup(group.denom, newOptions, options);
  var rule;
  var ruleWidth;
  var ruleSpacing;

  if (group.hasBarLine) {
    if (group.barSize) {
      ruleWidth = units_calculateSize(group.barSize, options);
      rule = buildCommon.makeLineSpan("frac-line", options, ruleWidth);
    } else {
      rule = buildCommon.makeLineSpan("frac-line", options);
    }

    ruleWidth = rule.height;
    ruleSpacing = rule.height;
  } else {
    rule = null;
    ruleWidth = 0;
    ruleSpacing = options.fontMetrics().defaultRuleThickness;
  } // Rule 15b


  var numShift;
  var clearance;
  var denomShift;

  if (style.size === src_Style.DISPLAY.size || group.size === "display") {
    numShift = options.fontMetrics().num1;

    if (ruleWidth > 0) {
      clearance = 3 * ruleSpacing;
    } else {
      clearance = 7 * ruleSpacing;
    }

    denomShift = options.fontMetrics().denom1;
  } else {
    if (ruleWidth > 0) {
      numShift = options.fontMetrics().num2;
      clearance = ruleSpacing;
    } else {
      numShift = options.fontMetrics().num3;
      clearance = 3 * ruleSpacing;
    }

    denomShift = options.fontMetrics().denom2;
  }

  var frac;

  if (!rule) {
    // Rule 15c
    var candidateClearance = numShift - numerm.depth - (denomm.height - denomShift);

    if (candidateClearance < clearance) {
      numShift += 0.5 * (clearance - candidateClearance);
      denomShift += 0.5 * (clearance - candidateClearance);
    }

    frac = buildCommon.makeVList({
      positionType: "individualShift",
      children: [{
        type: "elem",
        elem: denomm,
        shift: denomShift
      }, {
        type: "elem",
        elem: numerm,
        shift: -numShift
      }]
    }, options);
  } else {
    // Rule 15d
    var axisHeight = options.fontMetrics().axisHeight;

    if (numShift - numerm.depth - (axisHeight + 0.5 * ruleWidth) < clearance) {
      numShift += clearance - (numShift - numerm.depth - (axisHeight + 0.5 * ruleWidth));
    }

    if (axisHeight - 0.5 * ruleWidth - (denomm.height - denomShift) < clearance) {
      denomShift += clearance - (axisHeight - 0.5 * ruleWidth - (denomm.height - denomShift));
    }

    var midShift = -(axisHeight - 0.5 * ruleWidth);
    frac = buildCommon.makeVList({
      positionType: "individualShift",
      children: [{
        type: "elem",
        elem: denomm,
        shift: denomShift
      }, {
        type: "elem",
        elem: rule,
        shift: midShift
      }, {
        type: "elem",
        elem: numerm,
        shift: -numShift
      }]
    }, options);
  } // Since we manually change the style sometimes (with \dfrac or \tfrac),
  // account for the possible size change here.


  newOptions = options.havingStyle(style);
  frac.height *= newOptions.sizeMultiplier / options.sizeMultiplier;
  frac.depth *= newOptions.sizeMultiplier / options.sizeMultiplier; // Rule 15e

  var delimSize;

  if (style.size === src_Style.DISPLAY.size) {
    delimSize = options.fontMetrics().delim1;
  } else {
    delimSize = options.fontMetrics().delim2;
  }

  var leftDelim;
  var rightDelim;

  if (group.leftDelim == null) {
    leftDelim = makeNullDelimiter(options, ["mopen"]);
  } else {
    leftDelim = delimiter.customSizedDelim(group.leftDelim, delimSize, true, options.havingStyle(style), group.mode, ["mopen"]);
  }

  if (group.continued) {
    rightDelim = buildCommon.makeSpan([]); // zero width for \cfrac
  } else if (group.rightDelim == null) {
    rightDelim = makeNullDelimiter(options, ["mclose"]);
  } else {
    rightDelim = delimiter.customSizedDelim(group.rightDelim, delimSize, true, options.havingStyle(style), group.mode, ["mclose"]);
  }

  return buildCommon.makeSpan(["mord"].concat(newOptions.sizingClasses(options)), [leftDelim, buildCommon.makeSpan(["mfrac"], [frac]), rightDelim], options);
};

var genfrac_mathmlBuilder = function mathmlBuilder(group, options) {
  var node = new mathMLTree.MathNode("mfrac", [buildMathML_buildGroup(group.numer, options), buildMathML_buildGroup(group.denom, options)]);

  if (!group.hasBarLine) {
    node.setAttribute("linethickness", "0px");
  } else if (group.barSize) {
    var ruleWidth = units_calculateSize(group.barSize, options);
    node.setAttribute("linethickness", ruleWidth + "em");
  }

  var style = genfrac_adjustStyle(group.size, options.style);

  if (style.size !== options.style.size) {
    node = new mathMLTree.MathNode("mstyle", [node]);
    var isDisplay = style.size === src_Style.DISPLAY.size ? "true" : "false";
    node.setAttribute("displaystyle", isDisplay);
    node.setAttribute("scriptlevel", "0");
  }

  if (group.leftDelim != null || group.rightDelim != null) {
    var withDelims = [];

    if (group.leftDelim != null) {
      var leftOp = new mathMLTree.MathNode("mo", [new mathMLTree.TextNode(group.leftDelim.replace("\\", ""))]);
      leftOp.setAttribute("fence", "true");
      withDelims.push(leftOp);
    }

    withDelims.push(node);

    if (group.rightDelim != null) {
      var rightOp = new mathMLTree.MathNode("mo", [new mathMLTree.TextNode(group.rightDelim.replace("\\", ""))]);
      rightOp.setAttribute("fence", "true");
      withDelims.push(rightOp);
    }

    return buildMathML_makeRow(withDelims);
  }

  return node;
};

defineFunction({
  type: "genfrac",
  names: ["\\cfrac", "\\dfrac", "\\frac", "\\tfrac", "\\dbinom", "\\binom", "\\tbinom", "\\\\atopfrac", // can’t be entered directly
  "\\\\bracefrac", "\\\\brackfrac"],
  props: {
    numArgs: 2,
    greediness: 2
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var numer = args[0];
    var denom = args[1];
    var hasBarLine;
    var leftDelim = null;
    var rightDelim = null;
    var size = "auto";

    switch (funcName) {
      case "\\cfrac":
      case "\\dfrac":
      case "\\frac":
      case "\\tfrac":
        hasBarLine = true;
        break;

      case "\\\\atopfrac":
        hasBarLine = false;
        break;

      case "\\dbinom":
      case "\\binom":
      case "\\tbinom":
        hasBarLine = false;
        leftDelim = "(";
        rightDelim = ")";
        break;

      case "\\\\bracefrac":
        hasBarLine = false;
        leftDelim = "\\{";
        rightDelim = "\\}";
        break;

      case "\\\\brackfrac":
        hasBarLine = false;
        leftDelim = "[";
        rightDelim = "]";
        break;

      default:
        throw new Error("Unrecognized genfrac command");
    }

    switch (funcName) {
      case "\\cfrac":
      case "\\dfrac":
      case "\\dbinom":
        size = "display";
        break;

      case "\\tfrac":
      case "\\tbinom":
        size = "text";
        break;
    }

    return {
      type: "genfrac",
      mode: parser.mode,
      continued: funcName === "\\cfrac",
      numer: numer,
      denom: denom,
      hasBarLine: hasBarLine,
      leftDelim: leftDelim,
      rightDelim: rightDelim,
      size: size,
      barSize: null
    };
  },
  htmlBuilder: genfrac_htmlBuilder,
  mathmlBuilder: genfrac_mathmlBuilder
}); // Infix generalized fractions -- these are not rendered directly, but replaced
// immediately by one of the variants above.

defineFunction({
  type: "infix",
  names: ["\\over", "\\choose", "\\atop", "\\brace", "\\brack"],
  props: {
    numArgs: 0,
    infix: true
  },
  handler: function handler(_ref2) {
    var parser = _ref2.parser,
        funcName = _ref2.funcName,
        token = _ref2.token;
    var replaceWith;

    switch (funcName) {
      case "\\over":
        replaceWith = "\\frac";
        break;

      case "\\choose":
        replaceWith = "\\binom";
        break;

      case "\\atop":
        replaceWith = "\\\\atopfrac";
        break;

      case "\\brace":
        replaceWith = "\\\\bracefrac";
        break;

      case "\\brack":
        replaceWith = "\\\\brackfrac";
        break;

      default:
        throw new Error("Unrecognized infix genfrac command");
    }

    return {
      type: "infix",
      mode: parser.mode,
      replaceWith: replaceWith,
      token: token
    };
  }
});
var stylArray = ["display", "text", "script", "scriptscript"];

var delimFromValue = function delimFromValue(delimString) {
  var delim = null;

  if (delimString.length > 0) {
    delim = delimString;
    delim = delim === "." ? null : delim;
  }

  return delim;
};

defineFunction({
  type: "genfrac",
  names: ["\\genfrac"],
  props: {
    numArgs: 6,
    greediness: 6,
    argTypes: ["math", "math", "size", "text", "math", "math"]
  },
  handler: function handler(_ref3, args) {
    var parser = _ref3.parser;
    var numer = args[4];
    var denom = args[5]; // Look into the parse nodes to get the desired delimiters.

    var leftNode = checkNodeType(args[0], "atom");

    if (leftNode) {
      leftNode = assertAtomFamily(args[0], "open");
    }

    var leftDelim = leftNode ? delimFromValue(leftNode.text) : null;
    var rightNode = checkNodeType(args[1], "atom");

    if (rightNode) {
      rightNode = assertAtomFamily(args[1], "close");
    }

    var rightDelim = rightNode ? delimFromValue(rightNode.text) : null;
    var barNode = assertNodeType(args[2], "size");
    var hasBarLine;
    var barSize = null;

    if (barNode.isBlank) {
      // \genfrac acts differently than \above.
      // \genfrac treats an empty size group as a signal to use a
      // standard bar size. \above would see size = 0 and omit the bar.
      hasBarLine = true;
    } else {
      barSize = barNode.value;
      hasBarLine = barSize.number > 0;
    } // Find out if we want displaystyle, textstyle, etc.


    var size = "auto";
    var styl = checkNodeType(args[3], "ordgroup");

    if (styl) {
      if (styl.body.length > 0) {
        var textOrd = assertNodeType(styl.body[0], "textord");
        size = stylArray[Number(textOrd.text)];
      }
    } else {
      styl = assertNodeType(args[3], "textord");
      size = stylArray[Number(styl.text)];
    }

    return {
      type: "genfrac",
      mode: parser.mode,
      numer: numer,
      denom: denom,
      continued: false,
      hasBarLine: hasBarLine,
      barSize: barSize,
      leftDelim: leftDelim,
      rightDelim: rightDelim,
      size: size
    };
  },
  htmlBuilder: genfrac_htmlBuilder,
  mathmlBuilder: genfrac_mathmlBuilder
}); // \above is an infix fraction that also defines a fraction bar size.

defineFunction({
  type: "infix",
  names: ["\\above"],
  props: {
    numArgs: 1,
    argTypes: ["size"],
    infix: true
  },
  handler: function handler(_ref4, args) {
    var parser = _ref4.parser,
        funcName = _ref4.funcName,
        token = _ref4.token;
    return {
      type: "infix",
      mode: parser.mode,
      replaceWith: "\\\\abovefrac",
      size: assertNodeType(args[0], "size").value,
      token: token
    };
  }
});
defineFunction({
  type: "genfrac",
  names: ["\\\\abovefrac"],
  props: {
    numArgs: 3,
    argTypes: ["math", "size", "math"]
  },
  handler: function handler(_ref5, args) {
    var parser = _ref5.parser,
        funcName = _ref5.funcName;
    var numer = args[0];
    var barSize = assert(assertNodeType(args[1], "infix").size);
    var denom = args[2];
    var hasBarLine = barSize.number > 0;
    return {
      type: "genfrac",
      mode: parser.mode,
      numer: numer,
      denom: denom,
      continued: false,
      hasBarLine: hasBarLine,
      barSize: barSize,
      leftDelim: null,
      rightDelim: null,
      size: "auto"
    };
  },
  htmlBuilder: genfrac_htmlBuilder,
  mathmlBuilder: genfrac_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/horizBrace.js








// NOTE: Unlike most `htmlBuilder`s, this one handles not only "horizBrace", but
var horizBrace_htmlBuilder = function htmlBuilder(grp, options) {
  var style = options.style; // Pull out the `ParseNode<"horizBrace">` if `grp` is a "supsub" node.

  var supSubGroup;
  var group;
  var supSub = checkNodeType(grp, "supsub");

  if (supSub) {
    // Ref: LaTeX source2e: }}}}\limits}
    // i.e. LaTeX treats the brace similar to an op and passes it
    // with \limits, so we need to assign supsub style.
    supSubGroup = supSub.sup ? buildHTML_buildGroup(supSub.sup, options.havingStyle(style.sup()), options) : buildHTML_buildGroup(supSub.sub, options.havingStyle(style.sub()), options);
    group = assertNodeType(supSub.base, "horizBrace");
  } else {
    group = assertNodeType(grp, "horizBrace");
  } // Build the base group


  var body = buildHTML_buildGroup(group.base, options.havingBaseStyle(src_Style.DISPLAY)); // Create the stretchy element

  var braceBody = stretchy.svgSpan(group, options); // Generate the vlist, with the appropriate kerns        ┏━━━━━━━━┓
  // This first vlist contains the content and the brace:   equation

  var vlist;

  if (group.isOver) {
    vlist = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: body
      }, {
        type: "kern",
        size: 0.1
      }, {
        type: "elem",
        elem: braceBody
      }]
    }, options); // $FlowFixMe: Replace this with passing "svg-align" into makeVList.

    vlist.children[0].children[0].children[1].classes.push("svg-align");
  } else {
    vlist = buildCommon.makeVList({
      positionType: "bottom",
      positionData: body.depth + 0.1 + braceBody.height,
      children: [{
        type: "elem",
        elem: braceBody
      }, {
        type: "kern",
        size: 0.1
      }, {
        type: "elem",
        elem: body
      }]
    }, options); // $FlowFixMe: Replace this with passing "svg-align" into makeVList.

    vlist.children[0].children[0].children[0].classes.push("svg-align");
  }

  if (supSubGroup) {
    // To write the supsub, wrap the first vlist in another vlist:
    // They can't all go in the same vlist, because the note might be
    // wider than the equation. We want the equation to control the
    // brace width.
    //      note          long note           long note
    //   ┏━━━━━━━━┓   or    ┏━━━┓     not    ┏━━━━━━━━━┓
    //    equation           eqn                 eqn
    var vSpan = buildCommon.makeSpan(["mord", group.isOver ? "mover" : "munder"], [vlist], options);

    if (group.isOver) {
      vlist = buildCommon.makeVList({
        positionType: "firstBaseline",
        children: [{
          type: "elem",
          elem: vSpan
        }, {
          type: "kern",
          size: 0.2
        }, {
          type: "elem",
          elem: supSubGroup
        }]
      }, options);
    } else {
      vlist = buildCommon.makeVList({
        positionType: "bottom",
        positionData: vSpan.depth + 0.2 + supSubGroup.height + supSubGroup.depth,
        children: [{
          type: "elem",
          elem: supSubGroup
        }, {
          type: "kern",
          size: 0.2
        }, {
          type: "elem",
          elem: vSpan
        }]
      }, options);
    }
  }

  return buildCommon.makeSpan(["mord", group.isOver ? "mover" : "munder"], [vlist], options);
};

var horizBrace_mathmlBuilder = function mathmlBuilder(group, options) {
  var accentNode = stretchy.mathMLnode(group.label);
  return new mathMLTree.MathNode(group.isOver ? "mover" : "munder", [buildMathML_buildGroup(group.base, options), accentNode]);
}; // Horizontal stretchy braces


defineFunction({
  type: "horizBrace",
  names: ["\\overbrace", "\\underbrace"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    return {
      type: "horizBrace",
      mode: parser.mode,
      label: funcName,
      isOver: /^\\over/.test(funcName),
      base: args[0]
    };
  },
  htmlBuilder: horizBrace_htmlBuilder,
  mathmlBuilder: horizBrace_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/href.js






defineFunction({
  type: "href",
  names: ["\\href"],
  props: {
    numArgs: 2,
    argTypes: ["url", "original"],
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var body = args[1];
    var href = assertNodeType(args[0], "url").url;
    return {
      type: "href",
      mode: parser.mode,
      href: href,
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var elements = buildHTML_buildExpression(group.body, options, false);
    return buildCommon.makeAnchor(group.href, [], elements, options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var math = buildExpressionRow(group.body, options);

    if (!(math instanceof mathMLTree_MathNode)) {
      math = new mathMLTree_MathNode("mrow", [math]);
    }

    math.setAttribute("href", group.href);
    return math;
  }
});
defineFunction({
  type: "href",
  names: ["\\url"],
  props: {
    numArgs: 1,
    argTypes: ["url"],
    allowedInText: true
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser;
    var href = assertNodeType(args[0], "url").url;
    var chars = [];

    for (var i = 0; i < href.length; i++) {
      var c = href[i];

      if (c === "~") {
        c = "\\textasciitilde";
      }

      chars.push({
        type: "textord",
        mode: "text",
        text: c
      });
    }

    var body = {
      type: "text",
      mode: parser.mode,
      font: "\\texttt",
      body: chars
    };
    return {
      type: "href",
      mode: parser.mode,
      href: href,
      body: defineFunction_ordargument(body)
    };
  }
});
// CONCATENATED MODULE: ./src/functions/htmlmathml.js




defineFunction({
  type: "htmlmathml",
  names: ["\\html@mathml"],
  props: {
    numArgs: 2,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    return {
      type: "htmlmathml",
      mode: parser.mode,
      html: defineFunction_ordargument(args[0]),
      mathml: defineFunction_ordargument(args[1])
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var elements = buildHTML_buildExpression(group.html, options, false);
    return buildCommon.makeFragment(elements);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    return buildExpressionRow(group.mathml, options);
  }
});
// CONCATENATED MODULE: ./src/functions/kern.js
// Horizontal spacing commands




 // TODO: \hskip and \mskip should support plus and minus in lengths

defineFunction({
  type: "kern",
  names: ["\\kern", "\\mkern", "\\hskip", "\\mskip"],
  props: {
    numArgs: 1,
    argTypes: ["size"],
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var size = assertNodeType(args[0], "size");

    if (parser.settings.strict) {
      var mathFunction = funcName[1] === 'm'; // \mkern, \mskip

      var muUnit = size.value.unit === 'mu';

      if (mathFunction) {
        if (!muUnit) {
          parser.settings.reportNonstrict("mathVsTextUnits", "LaTeX's " + funcName + " supports only mu units, " + ("not " + size.value.unit + " units"));
        }

        if (parser.mode !== "math") {
          parser.settings.reportNonstrict("mathVsTextUnits", "LaTeX's " + funcName + " works only in math mode");
        }
      } else {
        // !mathFunction
        if (muUnit) {
          parser.settings.reportNonstrict("mathVsTextUnits", "LaTeX's " + funcName + " doesn't support mu units");
        }
      }
    }

    return {
      type: "kern",
      mode: parser.mode,
      dimension: size.value
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    return buildCommon.makeGlue(group.dimension, options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var dimension = units_calculateSize(group.dimension, options);
    return new mathMLTree.SpaceNode(dimension);
  }
});
// CONCATENATED MODULE: ./src/functions/lap.js
// Horizontal overlap functions





defineFunction({
  type: "lap",
  names: ["\\mathllap", "\\mathrlap", "\\mathclap"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var body = args[0];
    return {
      type: "lap",
      mode: parser.mode,
      alignment: funcName.slice(5),
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // mathllap, mathrlap, mathclap
    var inner;

    if (group.alignment === "clap") {
      // ref: https://www.math.lsu.edu/~aperlis/publications/mathclap/
      inner = buildCommon.makeSpan([], [buildHTML_buildGroup(group.body, options)]); // wrap, since CSS will center a .clap > .inner > span

      inner = buildCommon.makeSpan(["inner"], [inner], options);
    } else {
      inner = buildCommon.makeSpan(["inner"], [buildHTML_buildGroup(group.body, options)]);
    }

    var fix = buildCommon.makeSpan(["fix"], []);
    var node = buildCommon.makeSpan([group.alignment], [inner, fix], options); // At this point, we have correctly set horizontal alignment of the
    // two items involved in the lap.
    // Next, use a strut to set the height of the HTML bounding box.
    // Otherwise, a tall argument may be misplaced.

    var strut = buildCommon.makeSpan(["strut"]);
    strut.style.height = node.height + node.depth + "em";
    strut.style.verticalAlign = -node.depth + "em";
    node.children.unshift(strut); // Next, prevent vertical misplacement when next to something tall.

    node = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: node
      }]
    }, options); // Get the horizontal spacing correct relative to adjacent items.

    return buildCommon.makeSpan(["mord"], [node], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    // mathllap, mathrlap, mathclap
    var node = new mathMLTree.MathNode("mpadded", [buildMathML_buildGroup(group.body, options)]);

    if (group.alignment !== "rlap") {
      var offset = group.alignment === "llap" ? "-1" : "-0.5";
      node.setAttribute("lspace", offset + "width");
    }

    node.setAttribute("width", "0px");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/math.js

 // Switching from text mode back to math mode

defineFunction({
  type: "styling",
  names: ["\\(", "$"],
  props: {
    numArgs: 0,
    allowedInText: true,
    allowedInMath: false,
    consumeMode: "math"
  },
  handler: function handler(_ref, args) {
    var funcName = _ref.funcName,
        parser = _ref.parser;
    var outerMode = parser.mode;
    parser.switchMode("math");
    var close = funcName === "\\(" ? "\\)" : "$";
    var body = parser.parseExpression(false, close); // We can't expand the next symbol after the closing $ until after
    // switching modes back.  So don't consume within expect.

    parser.expect(close, false);
    parser.switchMode(outerMode);
    parser.consume();
    return {
      type: "styling",
      mode: parser.mode,
      style: "text",
      body: body
    };
  }
}); // Check for extra closing math delimiters

defineFunction({
  type: "text",
  // Doesn't matter what this is.
  names: ["\\)", "\\]"],
  props: {
    numArgs: 0,
    allowedInText: true,
    allowedInMath: false
  },
  handler: function handler(context, args) {
    throw new src_ParseError("Mismatched " + context.funcName);
  }
});
// CONCATENATED MODULE: ./src/functions/mathchoice.js






var mathchoice_chooseMathStyle = function chooseMathStyle(group, options) {
  switch (options.style.size) {
    case src_Style.DISPLAY.size:
      return group.display;

    case src_Style.TEXT.size:
      return group.text;

    case src_Style.SCRIPT.size:
      return group.script;

    case src_Style.SCRIPTSCRIPT.size:
      return group.scriptscript;

    default:
      return group.text;
  }
};

defineFunction({
  type: "mathchoice",
  names: ["\\mathchoice"],
  props: {
    numArgs: 4
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    return {
      type: "mathchoice",
      mode: parser.mode,
      display: defineFunction_ordargument(args[0]),
      text: defineFunction_ordargument(args[1]),
      script: defineFunction_ordargument(args[2]),
      scriptscript: defineFunction_ordargument(args[3])
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var body = mathchoice_chooseMathStyle(group, options);
    var elements = buildHTML_buildExpression(body, options, false);
    return buildCommon.makeFragment(elements);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var body = mathchoice_chooseMathStyle(group, options);
    return buildExpressionRow(body, options);
  }
});
// CONCATENATED MODULE: ./src/functions/op.js
// Limits, symbols









// Most operators have a large successor symbol, but these don't.
var noSuccessor = ["\\smallint"]; // NOTE: Unlike most `htmlBuilder`s, this one handles not only "op", but also
// "supsub" since some of them (like \int) can affect super/subscripting.

var op_htmlBuilder = function htmlBuilder(grp, options) {
  // Operators are handled in the TeXbook pg. 443-444, rule 13(a).
  var supGroup;
  var subGroup;
  var hasLimits = false;
  var group;
  var supSub = checkNodeType(grp, "supsub");

  if (supSub) {
    // If we have limits, supsub will pass us its group to handle. Pull
    // out the superscript and subscript and set the group to the op in
    // its base.
    supGroup = supSub.sup;
    subGroup = supSub.sub;
    group = assertNodeType(supSub.base, "op");
    hasLimits = true;
  } else {
    group = assertNodeType(grp, "op");
  }

  var style = options.style;
  var large = false;

  if (style.size === src_Style.DISPLAY.size && group.symbol && !utils.contains(noSuccessor, group.name)) {
    // Most symbol operators get larger in displaystyle (rule 13)
    large = true;
  }

  var base;

  if (group.symbol) {
    // If this is a symbol, create the symbol.
    var fontName = large ? "Size2-Regular" : "Size1-Regular";
    var stash = "";

    if (group.name === "\\oiint" || group.name === "\\oiiint") {
      // No font glyphs yet, so use a glyph w/o the oval.
      // TODO: When font glyphs are available, delete this code.
      stash = group.name.substr(1); // $FlowFixMe

      group.name = stash === "oiint" ? "\\iint" : "\\iiint";
    }

    base = buildCommon.makeSymbol(group.name, fontName, "math", options, ["mop", "op-symbol", large ? "large-op" : "small-op"]);

    if (stash.length > 0) {
      // We're in \oiint or \oiiint. Overlay the oval.
      // TODO: When font glyphs are available, delete this code.
      var italic = base.italic;
      var oval = buildCommon.staticSvg(stash + "Size" + (large ? "2" : "1"), options);
      base = buildCommon.makeVList({
        positionType: "individualShift",
        children: [{
          type: "elem",
          elem: base,
          shift: 0
        }, {
          type: "elem",
          elem: oval,
          shift: large ? 0.08 : 0
        }]
      }, options); // $FlowFixMe

      group.name = "\\" + stash;
      base.classes.unshift("mop"); // $FlowFixMe

      base.italic = italic;
    }
  } else if (group.body) {
    // If this is a list, compose that list.
    var inner = buildHTML_buildExpression(group.body, options, true);

    if (inner.length === 1 && inner[0] instanceof domTree_SymbolNode) {
      base = inner[0];
      base.classes[0] = "mop"; // replace old mclass
    } else {
      base = buildCommon.makeSpan(["mop"], buildCommon.tryCombineChars(inner), options);
    }
  } else {
    // Otherwise, this is a text operator. Build the text from the
    // operator's name.
    // TODO(emily): Add a space in the middle of some of these
    // operators, like \limsup
    var output = [];

    for (var i = 1; i < group.name.length; i++) {
      output.push(buildCommon.mathsym(group.name[i], group.mode));
    }

    base = buildCommon.makeSpan(["mop"], output, options);
  } // If content of op is a single symbol, shift it vertically.


  var baseShift = 0;
  var slant = 0;

  if ((base instanceof domTree_SymbolNode || group.name === "\\oiint" || group.name === "\\oiiint") && !group.suppressBaseShift) {
    // We suppress the shift of the base of \overset and \underset. Otherwise,
    // shift the symbol so its center lies on the axis (rule 13). It
    // appears that our fonts have the centers of the symbols already
    // almost on the axis, so these numbers are very small. Note we
    // don't actually apply this here, but instead it is used either in
    // the vlist creation or separately when there are no limits.
    baseShift = (base.height - base.depth) / 2 - options.fontMetrics().axisHeight; // The slant of the symbol is just its italic correction.
    // $FlowFixMe

    slant = base.italic;
  }

  if (hasLimits) {
    // IE 8 clips \int if it is in a display: inline-block. We wrap it
    // in a new span so it is an inline, and works.
    base = buildCommon.makeSpan([], [base]);
    var sub;
    var sup; // We manually have to handle the superscripts and subscripts. This,
    // aside from the kern calculations, is copied from supsub.

    if (supGroup) {
      var elem = buildHTML_buildGroup(supGroup, options.havingStyle(style.sup()), options);
      sup = {
        elem: elem,
        kern: Math.max(options.fontMetrics().bigOpSpacing1, options.fontMetrics().bigOpSpacing3 - elem.depth)
      };
    }

    if (subGroup) {
      var _elem = buildHTML_buildGroup(subGroup, options.havingStyle(style.sub()), options);

      sub = {
        elem: _elem,
        kern: Math.max(options.fontMetrics().bigOpSpacing2, options.fontMetrics().bigOpSpacing4 - _elem.height)
      };
    } // Build the final group as a vlist of the possible subscript, base,
    // and possible superscript.


    var finalGroup;

    if (sup && sub) {
      var bottom = options.fontMetrics().bigOpSpacing5 + sub.elem.height + sub.elem.depth + sub.kern + base.depth + baseShift;
      finalGroup = buildCommon.makeVList({
        positionType: "bottom",
        positionData: bottom,
        children: [{
          type: "kern",
          size: options.fontMetrics().bigOpSpacing5
        }, {
          type: "elem",
          elem: sub.elem,
          marginLeft: -slant + "em"
        }, {
          type: "kern",
          size: sub.kern
        }, {
          type: "elem",
          elem: base
        }, {
          type: "kern",
          size: sup.kern
        }, {
          type: "elem",
          elem: sup.elem,
          marginLeft: slant + "em"
        }, {
          type: "kern",
          size: options.fontMetrics().bigOpSpacing5
        }]
      }, options);
    } else if (sub) {
      var top = base.height - baseShift; // Shift the limits by the slant of the symbol. Note
      // that we are supposed to shift the limits by 1/2 of the slant,
      // but since we are centering the limits adding a full slant of
      // margin will shift by 1/2 that.

      finalGroup = buildCommon.makeVList({
        positionType: "top",
        positionData: top,
        children: [{
          type: "kern",
          size: options.fontMetrics().bigOpSpacing5
        }, {
          type: "elem",
          elem: sub.elem,
          marginLeft: -slant + "em"
        }, {
          type: "kern",
          size: sub.kern
        }, {
          type: "elem",
          elem: base
        }]
      }, options);
    } else if (sup) {
      var _bottom = base.depth + baseShift;

      finalGroup = buildCommon.makeVList({
        positionType: "bottom",
        positionData: _bottom,
        children: [{
          type: "elem",
          elem: base
        }, {
          type: "kern",
          size: sup.kern
        }, {
          type: "elem",
          elem: sup.elem,
          marginLeft: slant + "em"
        }, {
          type: "kern",
          size: options.fontMetrics().bigOpSpacing5
        }]
      }, options);
    } else {
      // This case probably shouldn't occur (this would mean the
      // supsub was sending us a group with no superscript or
      // subscript) but be safe.
      return base;
    }

    return buildCommon.makeSpan(["mop", "op-limits"], [finalGroup], options);
  } else {
    if (baseShift) {
      base.style.position = "relative";
      base.style.top = baseShift + "em";
    }

    return base;
  }
};

var op_mathmlBuilder = function mathmlBuilder(group, options) {
  var node;

  if (group.symbol) {
    // This is a symbol. Just add the symbol.
    node = new mathMLTree_MathNode("mo", [buildMathML_makeText(group.name, group.mode)]);

    if (utils.contains(noSuccessor, group.name)) {
      node.setAttribute("largeop", "false");
    }
  } else if (group.body) {
    // This is an operator with children. Add them.
    node = new mathMLTree_MathNode("mo", buildMathML_buildExpression(group.body, options));
  } else {
    // This is a text operator. Add all of the characters from the
    // operator's name.
    // TODO(emily): Add a space in the middle of some of these
    // operators, like \limsup.
    node = new mathMLTree_MathNode("mi", [new mathMLTree_TextNode(group.name.slice(1))]); // Append an <mo>&ApplyFunction;</mo>.
    // ref: https://www.w3.org/TR/REC-MathML/chap3_2.html#sec3.2.4

    var operator = new mathMLTree_MathNode("mo", [buildMathML_makeText("\u2061", "text")]);

    if (group.parentIsSupSub) {
      node = new mathMLTree_MathNode("mo", [node, operator]);
    } else {
      node = newDocumentFragment([node, operator]);
    }
  }

  return node;
};

var singleCharBigOps = {
  "\u220F": "\\prod",
  "\u2210": "\\coprod",
  "\u2211": "\\sum",
  "\u22C0": "\\bigwedge",
  "\u22C1": "\\bigvee",
  "\u22C2": "\\bigcap",
  "\u22C3": "\\bigcup",
  "\u2A00": "\\bigodot",
  "\u2A01": "\\bigoplus",
  "\u2A02": "\\bigotimes",
  "\u2A04": "\\biguplus",
  "\u2A06": "\\bigsqcup"
};
defineFunction({
  type: "op",
  names: ["\\coprod", "\\bigvee", "\\bigwedge", "\\biguplus", "\\bigcap", "\\bigcup", "\\intop", "\\prod", "\\sum", "\\bigotimes", "\\bigoplus", "\\bigodot", "\\bigsqcup", "\\smallint", "\u220F", "\u2210", "\u2211", "\u22C0", "\u22C1", "\u22C2", "\u22C3", "\u2A00", "\u2A01", "\u2A02", "\u2A04", "\u2A06"],
  props: {
    numArgs: 0
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var fName = funcName;

    if (fName.length === 1) {
      fName = singleCharBigOps[fName];
    }

    return {
      type: "op",
      mode: parser.mode,
      limits: true,
      parentIsSupSub: false,
      symbol: true,
      name: fName
    };
  },
  htmlBuilder: op_htmlBuilder,
  mathmlBuilder: op_mathmlBuilder
}); // Note: calling defineFunction with a type that's already been defined only
// works because the same htmlBuilder and mathmlBuilder are being used.

defineFunction({
  type: "op",
  names: ["\\mathop"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser;
    var body = args[0];
    return {
      type: "op",
      mode: parser.mode,
      limits: false,
      parentIsSupSub: false,
      symbol: false,
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: op_htmlBuilder,
  mathmlBuilder: op_mathmlBuilder
}); // There are 2 flags for operators; whether they produce limits in
// displaystyle, and whether they are symbols and should grow in
// displaystyle. These four groups cover the four possible choices.

var singleCharIntegrals = {
  "\u222B": "\\int",
  "\u222C": "\\iint",
  "\u222D": "\\iiint",
  "\u222E": "\\oint",
  "\u222F": "\\oiint",
  "\u2230": "\\oiiint"
}; // No limits, not symbols

defineFunction({
  type: "op",
  names: ["\\arcsin", "\\arccos", "\\arctan", "\\arctg", "\\arcctg", "\\arg", "\\ch", "\\cos", "\\cosec", "\\cosh", "\\cot", "\\cotg", "\\coth", "\\csc", "\\ctg", "\\cth", "\\deg", "\\dim", "\\exp", "\\hom", "\\ker", "\\lg", "\\ln", "\\log", "\\sec", "\\sin", "\\sinh", "\\sh", "\\tan", "\\tanh", "\\tg", "\\th"],
  props: {
    numArgs: 0
  },
  handler: function handler(_ref3) {
    var parser = _ref3.parser,
        funcName = _ref3.funcName;
    return {
      type: "op",
      mode: parser.mode,
      limits: false,
      parentIsSupSub: false,
      symbol: false,
      name: funcName
    };
  },
  htmlBuilder: op_htmlBuilder,
  mathmlBuilder: op_mathmlBuilder
}); // Limits, not symbols

defineFunction({
  type: "op",
  names: ["\\det", "\\gcd", "\\inf", "\\lim", "\\max", "\\min", "\\Pr", "\\sup"],
  props: {
    numArgs: 0
  },
  handler: function handler(_ref4) {
    var parser = _ref4.parser,
        funcName = _ref4.funcName;
    return {
      type: "op",
      mode: parser.mode,
      limits: true,
      parentIsSupSub: false,
      symbol: false,
      name: funcName
    };
  },
  htmlBuilder: op_htmlBuilder,
  mathmlBuilder: op_mathmlBuilder
}); // No limits, symbols

defineFunction({
  type: "op",
  names: ["\\int", "\\iint", "\\iiint", "\\oint", "\\oiint", "\\oiiint", "\u222B", "\u222C", "\u222D", "\u222E", "\u222F", "\u2230"],
  props: {
    numArgs: 0
  },
  handler: function handler(_ref5) {
    var parser = _ref5.parser,
        funcName = _ref5.funcName;
    var fName = funcName;

    if (fName.length === 1) {
      fName = singleCharIntegrals[fName];
    }

    return {
      type: "op",
      mode: parser.mode,
      limits: false,
      parentIsSupSub: false,
      symbol: true,
      name: fName
    };
  },
  htmlBuilder: op_htmlBuilder,
  mathmlBuilder: op_mathmlBuilder
});
// CONCATENATED MODULE: ./src/functions/operatorname.js





 // \operatorname
// amsopn.dtx: \mathop{#1\kern\z@\operator@font#3}\newmcodes@

defineFunction({
  type: "operatorname",
  names: ["\\operatorname"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var body = args[0];
    return {
      type: "operatorname",
      mode: parser.mode,
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    if (group.body.length > 0) {
      var body = group.body.map(function (child) {
        // $FlowFixMe: Check if the node has a string `text` property.
        var childText = child.text;

        if (typeof childText === "string") {
          return {
            type: "textord",
            mode: child.mode,
            text: childText
          };
        } else {
          return child;
        }
      }); // Consolidate function names into symbol characters.

      var expression = buildHTML_buildExpression(body, options.withFont("mathrm"), true);

      for (var i = 0; i < expression.length; i++) {
        var child = expression[i];

        if (child instanceof domTree_SymbolNode) {
          // Per amsopn package,
          // change minus to hyphen and \ast to asterisk
          child.text = child.text.replace(/\u2212/, "-").replace(/\u2217/, "*");
        }
      }

      return buildCommon.makeSpan(["mop"], expression, options);
    } else {
      return buildCommon.makeSpan(["mop"], [], options);
    }
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    // The steps taken here are similar to the html version.
    var expression = buildMathML_buildExpression(group.body, options.withFont("mathrm")); // Is expression a string or has it something like a fraction?

    var isAllString = true; // default

    for (var i = 0; i < expression.length; i++) {
      var node = expression[i];

      if (node instanceof mathMLTree.SpaceNode) {// Do nothing
      } else if (node instanceof mathMLTree.MathNode) {
        switch (node.type) {
          case "mi":
          case "mn":
          case "ms":
          case "mspace":
          case "mtext":
            break;
          // Do nothing yet.

          case "mo":
            {
              var child = node.children[0];

              if (node.children.length === 1 && child instanceof mathMLTree.TextNode) {
                child.text = child.text.replace(/\u2212/, "-").replace(/\u2217/, "*");
              } else {
                isAllString = false;
              }

              break;
            }

          default:
            isAllString = false;
        }
      } else {
        isAllString = false;
      }
    }

    if (isAllString) {
      // Write a single TextNode instead of multiple nested tags.
      var word = expression.map(function (node) {
        return node.toText();
      }).join("");
      expression = [new mathMLTree.TextNode(word)];
    }

    var identifier = new mathMLTree.MathNode("mi", expression);
    identifier.setAttribute("mathvariant", "normal"); // \u2061 is the same as &ApplyFunction;
    // ref: https://www.w3schools.com/charsets/ref_html_entities_a.asp

    var operator = new mathMLTree.MathNode("mo", [buildMathML_makeText("\u2061", "text")]);
    return mathMLTree.newDocumentFragment([identifier, operator]);
  }
});
// CONCATENATED MODULE: ./src/functions/ordgroup.js




defineFunctionBuilders({
  type: "ordgroup",
  htmlBuilder: function htmlBuilder(group, options) {
    if (group.semisimple) {
      return buildCommon.makeFragment(buildHTML_buildExpression(group.body, options, false));
    }

    return buildCommon.makeSpan(["mord"], buildHTML_buildExpression(group.body, options, true), options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    return buildExpressionRow(group.body, options);
  }
});
// CONCATENATED MODULE: ./src/functions/overline.js





defineFunction({
  type: "overline",
  names: ["\\overline"],
  props: {
    numArgs: 1
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var body = args[0];
    return {
      type: "overline",
      mode: parser.mode,
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Overlines are handled in the TeXbook pg 443, Rule 9.
    // Build the inner group in the cramped style.
    var innerGroup = buildHTML_buildGroup(group.body, options.havingCrampedStyle()); // Create the line above the body

    var line = buildCommon.makeLineSpan("overline-line", options); // Generate the vlist, with the appropriate kerns

    var vlist = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: innerGroup
      }, {
        type: "kern",
        size: 3 * line.height
      }, {
        type: "elem",
        elem: line
      }, {
        type: "kern",
        size: line.height
      }]
    }, options);
    return buildCommon.makeSpan(["mord", "overline"], [vlist], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var operator = new mathMLTree.MathNode("mo", [new mathMLTree.TextNode("\u203E")]);
    operator.setAttribute("stretchy", "true");
    var node = new mathMLTree.MathNode("mover", [buildMathML_buildGroup(group.body, options), operator]);
    node.setAttribute("accent", "true");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/phantom.js





defineFunction({
  type: "phantom",
  names: ["\\phantom"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var body = args[0];
    return {
      type: "phantom",
      mode: parser.mode,
      body: defineFunction_ordargument(body)
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var elements = buildHTML_buildExpression(group.body, options.withPhantom(), false); // \phantom isn't supposed to affect the elements it contains.
    // See "color" for more details.

    return buildCommon.makeFragment(elements);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var inner = buildMathML_buildExpression(group.body, options);
    return new mathMLTree.MathNode("mphantom", inner);
  }
});
defineFunction({
  type: "hphantom",
  names: ["\\hphantom"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref2, args) {
    var parser = _ref2.parser;
    var body = args[0];
    return {
      type: "hphantom",
      mode: parser.mode,
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var node = buildCommon.makeSpan([], [buildHTML_buildGroup(group.body, options.withPhantom())]);
    node.height = 0;
    node.depth = 0;

    if (node.children) {
      for (var i = 0; i < node.children.length; i++) {
        node.children[i].height = 0;
        node.children[i].depth = 0;
      }
    } // See smash for comment re: use of makeVList


    node = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: node
      }]
    }, options); // For spacing, TeX treats \smash as a math group (same spacing as ord).

    return buildCommon.makeSpan(["mord"], [node], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var inner = buildMathML_buildExpression(defineFunction_ordargument(group.body), options);
    var phantom = new mathMLTree.MathNode("mphantom", inner);
    var node = new mathMLTree.MathNode("mpadded", [phantom]);
    node.setAttribute("height", "0px");
    node.setAttribute("depth", "0px");
    return node;
  }
});
defineFunction({
  type: "vphantom",
  names: ["\\vphantom"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref3, args) {
    var parser = _ref3.parser;
    var body = args[0];
    return {
      type: "vphantom",
      mode: parser.mode,
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var inner = buildCommon.makeSpan(["inner"], [buildHTML_buildGroup(group.body, options.withPhantom())]);
    var fix = buildCommon.makeSpan(["fix"], []);
    return buildCommon.makeSpan(["mord", "rlap"], [inner, fix], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var inner = buildMathML_buildExpression(defineFunction_ordargument(group.body), options);
    var phantom = new mathMLTree.MathNode("mphantom", inner);
    var node = new mathMLTree.MathNode("mpadded", [phantom]);
    node.setAttribute("width", "0px");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/sizing.js





function sizingGroup(value, options, baseOptions) {
  var inner = buildHTML_buildExpression(value, options, false);
  var multiplier = options.sizeMultiplier / baseOptions.sizeMultiplier; // Add size-resetting classes to the inner list and set maxFontSize
  // manually. Handle nested size changes.

  for (var i = 0; i < inner.length; i++) {
    var pos = inner[i].classes.indexOf("sizing");

    if (pos < 0) {
      Array.prototype.push.apply(inner[i].classes, options.sizingClasses(baseOptions));
    } else if (inner[i].classes[pos + 1] === "reset-size" + options.size) {
      // This is a nested size change: e.g., inner[i] is the "b" in
      // `\Huge a \small b`. Override the old size (the `reset-` class)
      // but not the new size.
      inner[i].classes[pos + 1] = "reset-size" + baseOptions.size;
    }

    inner[i].height *= multiplier;
    inner[i].depth *= multiplier;
  }

  return buildCommon.makeFragment(inner);
}
var sizeFuncs = ["\\tiny", "\\sixptsize", "\\scriptsize", "\\footnotesize", "\\small", "\\normalsize", "\\large", "\\Large", "\\LARGE", "\\huge", "\\Huge"];
var sizing_htmlBuilder = function htmlBuilder(group, options) {
  // Handle sizing operators like \Huge. Real TeX doesn't actually allow
  // these functions inside of math expressions, so we do some special
  // handling.
  var newOptions = options.havingSize(group.size);
  return sizingGroup(group.body, newOptions, options);
};
defineFunction({
  type: "sizing",
  names: sizeFuncs,
  props: {
    numArgs: 0,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var breakOnTokenText = _ref.breakOnTokenText,
        funcName = _ref.funcName,
        parser = _ref.parser;
    var body = parser.parseExpression(false, breakOnTokenText);
    return {
      type: "sizing",
      mode: parser.mode,
      // Figure out what size to use based on the list of functions above
      size: sizeFuncs.indexOf(funcName) + 1,
      body: body
    };
  },
  htmlBuilder: sizing_htmlBuilder,
  mathmlBuilder: function mathmlBuilder(group, options) {
    var newOptions = options.havingSize(group.size);
    var inner = buildMathML_buildExpression(group.body, newOptions);
    var node = new mathMLTree.MathNode("mstyle", inner); // TODO(emily): This doesn't produce the correct size for nested size
    // changes, because we don't keep state of what style we're currently
    // in, so we can't reset the size to normal before changing it.  Now
    // that we're passing an options parameter we should be able to fix
    // this.

    node.setAttribute("mathsize", newOptions.sizeMultiplier + "em");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/raisebox.js






 // Box manipulation

defineFunction({
  type: "raisebox",
  names: ["\\raisebox"],
  props: {
    numArgs: 2,
    argTypes: ["size", "text"],
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    var amount = assertNodeType(args[0], "size").value;
    var body = args[1];
    return {
      type: "raisebox",
      mode: parser.mode,
      dy: amount,
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var text = {
      type: "text",
      mode: group.mode,
      body: defineFunction_ordargument(group.body),
      font: "mathrm" // simulate \textrm

    };
    var sizedText = {
      type: "sizing",
      mode: group.mode,
      body: [text],
      size: 6 // simulate \normalsize

    };
    var body = sizing_htmlBuilder(sizedText, options);
    var dy = units_calculateSize(group.dy, options);
    return buildCommon.makeVList({
      positionType: "shift",
      positionData: -dy,
      children: [{
        type: "elem",
        elem: body
      }]
    }, options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node = new mathMLTree.MathNode("mpadded", [buildMathML_buildGroup(group.body, options)]);
    var dy = group.dy.number + group.dy.unit;
    node.setAttribute("voffset", dy);
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/rule.js





defineFunction({
  type: "rule",
  names: ["\\rule"],
  props: {
    numArgs: 2,
    numOptionalArgs: 1,
    argTypes: ["size", "size", "size"]
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser;
    var shift = optArgs[0];
    var width = assertNodeType(args[0], "size");
    var height = assertNodeType(args[1], "size");
    return {
      type: "rule",
      mode: parser.mode,
      shift: shift && assertNodeType(shift, "size").value,
      width: width.value,
      height: height.value
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Make an empty span for the rule
    var rule = buildCommon.makeSpan(["mord", "rule"], [], options); // Calculate the shift, width, and height of the rule, and account for units

    var width = units_calculateSize(group.width, options);
    var height = units_calculateSize(group.height, options);
    var shift = group.shift ? units_calculateSize(group.shift, options) : 0; // Style the rule to the right size

    rule.style.borderRightWidth = width + "em";
    rule.style.borderTopWidth = height + "em";
    rule.style.bottom = shift + "em"; // Record the height and width

    rule.width = width;
    rule.height = height + shift;
    rule.depth = -shift; // Font size is the number large enough that the browser will
    // reserve at least `absHeight` space above the baseline.
    // The 1.125 factor was empirically determined

    rule.maxFontSize = height * 1.125 * options.sizeMultiplier;
    return rule;
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var width = units_calculateSize(group.width, options);
    var height = units_calculateSize(group.height, options);
    var shift = group.shift ? units_calculateSize(group.shift, options) : 0;
    var color = options.color && options.getColor() || "black";
    var rule = new mathMLTree.MathNode("mspace");
    rule.setAttribute("mathbackground", color);
    rule.setAttribute("width", width + "em");
    rule.setAttribute("height", height + "em");
    var wrapper = new mathMLTree.MathNode("mpadded", [rule]);

    if (shift >= 0) {
      wrapper.setAttribute("height", "+" + shift + "em");
    } else {
      wrapper.setAttribute("height", shift + "em");
      wrapper.setAttribute("depth", "+" + -shift + "em");
    }

    wrapper.setAttribute("voffset", shift + "em");
    return wrapper;
  }
});
// CONCATENATED MODULE: ./src/functions/smash.js
// smash, with optional [tb], as in AMS






defineFunction({
  type: "smash",
  names: ["\\smash"],
  props: {
    numArgs: 1,
    numOptionalArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser;
    var smashHeight = false;
    var smashDepth = false;
    var tbArg = optArgs[0] && assertNodeType(optArgs[0], "ordgroup");

    if (tbArg) {
      // Optional [tb] argument is engaged.
      // ref: amsmath: \renewcommand{\smash}[1][tb]{%
      //               def\mb@t{\ht}\def\mb@b{\dp}\def\mb@tb{\ht\z@\z@\dp}%
      var letter = "";

      for (var i = 0; i < tbArg.body.length; ++i) {
        var node = tbArg.body[i]; // $FlowFixMe: Not every node type has a `text` property.

        letter = node.text;

        if (letter === "t") {
          smashHeight = true;
        } else if (letter === "b") {
          smashDepth = true;
        } else {
          smashHeight = false;
          smashDepth = false;
          break;
        }
      }
    } else {
      smashHeight = true;
      smashDepth = true;
    }

    var body = args[0];
    return {
      type: "smash",
      mode: parser.mode,
      body: body,
      smashHeight: smashHeight,
      smashDepth: smashDepth
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var node = buildCommon.makeSpan([], [buildHTML_buildGroup(group.body, options)]);

    if (!group.smashHeight && !group.smashDepth) {
      return node;
    }

    if (group.smashHeight) {
      node.height = 0; // In order to influence makeVList, we have to reset the children.

      if (node.children) {
        for (var i = 0; i < node.children.length; i++) {
          node.children[i].height = 0;
        }
      }
    }

    if (group.smashDepth) {
      node.depth = 0;

      if (node.children) {
        for (var _i = 0; _i < node.children.length; _i++) {
          node.children[_i].depth = 0;
        }
      }
    } // At this point, we've reset the TeX-like height and depth values.
    // But the span still has an HTML line height.
    // makeVList applies "display: table-cell", which prevents the browser
    // from acting on that line height. So we'll call makeVList now.


    var smashedNode = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: node
      }]
    }, options); // For spacing, TeX treats \hphantom as a math group (same spacing as ord).

    return buildCommon.makeSpan(["mord"], [smashedNode], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node = new mathMLTree.MathNode("mpadded", [buildMathML_buildGroup(group.body, options)]);

    if (group.smashHeight) {
      node.setAttribute("height", "0px");
    }

    if (group.smashDepth) {
      node.setAttribute("depth", "0px");
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/sqrt.js







defineFunction({
  type: "sqrt",
  names: ["\\sqrt"],
  props: {
    numArgs: 1,
    numOptionalArgs: 1
  },
  handler: function handler(_ref, args, optArgs) {
    var parser = _ref.parser;
    var index = optArgs[0];
    var body = args[0];
    return {
      type: "sqrt",
      mode: parser.mode,
      body: body,
      index: index
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Square roots are handled in the TeXbook pg. 443, Rule 11.
    // First, we do the same steps as in overline to build the inner group
    // and line
    var inner = buildHTML_buildGroup(group.body, options.havingCrampedStyle());

    if (inner.height === 0) {
      // Render a small surd.
      inner.height = options.fontMetrics().xHeight;
    } // Some groups can return document fragments.  Handle those by wrapping
    // them in a span.


    inner = buildCommon.wrapFragment(inner, options); // Calculate the minimum size for the \surd delimiter

    var metrics = options.fontMetrics();
    var theta = metrics.defaultRuleThickness;
    var phi = theta;

    if (options.style.id < src_Style.TEXT.id) {
      phi = options.fontMetrics().xHeight;
    } // Calculate the clearance between the body and line


    var lineClearance = theta + phi / 4;
    var minDelimiterHeight = inner.height + inner.depth + lineClearance + theta; // Create a sqrt SVG of the required minimum size

    var _delimiter$sqrtImage = delimiter.sqrtImage(minDelimiterHeight, options),
        img = _delimiter$sqrtImage.span,
        ruleWidth = _delimiter$sqrtImage.ruleWidth,
        advanceWidth = _delimiter$sqrtImage.advanceWidth;

    var delimDepth = img.height - ruleWidth; // Adjust the clearance based on the delimiter size

    if (delimDepth > inner.height + inner.depth + lineClearance) {
      lineClearance = (lineClearance + delimDepth - inner.height - inner.depth) / 2;
    } // Shift the sqrt image


    var imgShift = img.height - inner.height - lineClearance - ruleWidth;
    inner.style.paddingLeft = advanceWidth + "em"; // Overlay the image and the argument.

    var body = buildCommon.makeVList({
      positionType: "firstBaseline",
      children: [{
        type: "elem",
        elem: inner,
        wrapperClasses: ["svg-align"]
      }, {
        type: "kern",
        size: -(inner.height + imgShift)
      }, {
        type: "elem",
        elem: img
      }, {
        type: "kern",
        size: ruleWidth
      }]
    }, options);

    if (!group.index) {
      return buildCommon.makeSpan(["mord", "sqrt"], [body], options);
    } else {
      // Handle the optional root index
      // The index is always in scriptscript style
      var newOptions = options.havingStyle(src_Style.SCRIPTSCRIPT);
      var rootm = buildHTML_buildGroup(group.index, newOptions, options); // The amount the index is shifted by. This is taken from the TeX
      // source, in the definition of `\r@@t`.

      var toShift = 0.6 * (body.height - body.depth); // Build a VList with the superscript shifted up correctly

      var rootVList = buildCommon.makeVList({
        positionType: "shift",
        positionData: -toShift,
        children: [{
          type: "elem",
          elem: rootm
        }]
      }, options); // Add a class surrounding it so we can add on the appropriate
      // kerning

      var rootVListWrap = buildCommon.makeSpan(["root"], [rootVList]);
      return buildCommon.makeSpan(["mord", "sqrt"], [rootVListWrap, body], options);
    }
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var body = group.body,
        index = group.index;
    return index ? new mathMLTree.MathNode("mroot", [buildMathML_buildGroup(body, options), buildMathML_buildGroup(index, options)]) : new mathMLTree.MathNode("msqrt", [buildMathML_buildGroup(body, options)]);
  }
});
// CONCATENATED MODULE: ./src/functions/styling.js





var styling_styleMap = {
  "display": src_Style.DISPLAY,
  "text": src_Style.TEXT,
  "script": src_Style.SCRIPT,
  "scriptscript": src_Style.SCRIPTSCRIPT
};
defineFunction({
  type: "styling",
  names: ["\\displaystyle", "\\textstyle", "\\scriptstyle", "\\scriptscriptstyle"],
  props: {
    numArgs: 0,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var breakOnTokenText = _ref.breakOnTokenText,
        funcName = _ref.funcName,
        parser = _ref.parser;
    // parse out the implicit body
    var body = parser.parseExpression(true, breakOnTokenText); // TODO: Refactor to avoid duplicating styleMap in multiple places (e.g.
    // here and in buildHTML and de-dupe the enumeration of all the styles).
    // $FlowFixMe: The names above exactly match the styles.

    var style = funcName.slice(1, funcName.length - 5);
    return {
      type: "styling",
      mode: parser.mode,
      // Figure out what style to use by pulling out the style from
      // the function name
      style: style,
      body: body
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Style changes are handled in the TeXbook on pg. 442, Rule 3.
    var newStyle = styling_styleMap[group.style];
    var newOptions = options.havingStyle(newStyle).withFont('');
    return sizingGroup(group.body, newOptions, options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    // Figure out what style we're changing to.
    // TODO(kevinb): dedupe this with buildHTML.js
    // This will be easier of handling of styling nodes is in the same file.
    var styleMap = {
      "display": src_Style.DISPLAY,
      "text": src_Style.TEXT,
      "script": src_Style.SCRIPT,
      "scriptscript": src_Style.SCRIPTSCRIPT
    };
    var newStyle = styleMap[group.style];
    var newOptions = options.havingStyle(newStyle);
    var inner = buildMathML_buildExpression(group.body, newOptions);
    var node = new mathMLTree.MathNode("mstyle", inner);
    var styleAttributes = {
      "display": ["0", "true"],
      "text": ["0", "false"],
      "script": ["1", "false"],
      "scriptscript": ["2", "false"]
    };
    var attr = styleAttributes[group.style];
    node.setAttribute("scriptlevel", attr[0]);
    node.setAttribute("displaystyle", attr[1]);
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/supsub.js













/**
 * Sometimes, groups perform special rules when they have superscripts or
 * subscripts attached to them. This function lets the `supsub` group know that
 * Sometimes, groups perform special rules when they have superscripts or
 * its inner element should handle the superscripts and subscripts instead of
 * handling them itself.
 */
var supsub_htmlBuilderDelegate = function htmlBuilderDelegate(group, options) {
  var base = group.base;

  if (!base) {
    return null;
  } else if (base.type === "op") {
    // Operators handle supsubs differently when they have limits
    // (e.g. `\displaystyle\sum_2^3`)
    var delegate = base.limits && (options.style.size === src_Style.DISPLAY.size || base.alwaysHandleSupSub);
    return delegate ? op_htmlBuilder : null;
  } else if (base.type === "accent") {
    return utils.isCharacterBox(base.base) ? accent_htmlBuilder : null;
  } else if (base.type === "horizBrace") {
    var isSup = !group.sub;
    return isSup === base.isOver ? horizBrace_htmlBuilder : null;
  } else {
    return null;
  }
}; // Super scripts and subscripts, whose precise placement can depend on other
// functions that precede them.


defineFunctionBuilders({
  type: "supsub",
  htmlBuilder: function htmlBuilder(group, options) {
    // Superscript and subscripts are handled in the TeXbook on page
    // 445-446, rules 18(a-f).
    // Here is where we defer to the inner group if it should handle
    // superscripts and subscripts itself.
    var builderDelegate = supsub_htmlBuilderDelegate(group, options);

    if (builderDelegate) {
      return builderDelegate(group, options);
    }

    var valueBase = group.base,
        valueSup = group.sup,
        valueSub = group.sub;
    var base = buildHTML_buildGroup(valueBase, options);
    var supm;
    var subm;
    var metrics = options.fontMetrics(); // Rule 18a

    var supShift = 0;
    var subShift = 0;
    var isCharacterBox = valueBase && utils.isCharacterBox(valueBase);

    if (valueSup) {
      var newOptions = options.havingStyle(options.style.sup());
      supm = buildHTML_buildGroup(valueSup, newOptions, options);

      if (!isCharacterBox) {
        supShift = base.height - newOptions.fontMetrics().supDrop * newOptions.sizeMultiplier / options.sizeMultiplier;
      }
    }

    if (valueSub) {
      var _newOptions = options.havingStyle(options.style.sub());

      subm = buildHTML_buildGroup(valueSub, _newOptions, options);

      if (!isCharacterBox) {
        subShift = base.depth + _newOptions.fontMetrics().subDrop * _newOptions.sizeMultiplier / options.sizeMultiplier;
      }
    } // Rule 18c


    var minSupShift;

    if (options.style === src_Style.DISPLAY) {
      minSupShift = metrics.sup1;
    } else if (options.style.cramped) {
      minSupShift = metrics.sup3;
    } else {
      minSupShift = metrics.sup2;
    } // scriptspace is a font-size-independent size, so scale it
    // appropriately for use as the marginRight.


    var multiplier = options.sizeMultiplier;
    var marginRight = 0.5 / metrics.ptPerEm / multiplier + "em";
    var marginLeft = null;

    if (subm) {
      // Subscripts shouldn't be shifted by the base's italic correction.
      // Account for that by shifting the subscript back the appropriate
      // amount. Note we only do this when the base is a single symbol.
      var isOiint = group.base && group.base.type === "op" && group.base.name && (group.base.name === "\\oiint" || group.base.name === "\\oiiint");

      if (base instanceof domTree_SymbolNode || isOiint) {
        // $FlowFixMe
        marginLeft = -base.italic + "em";
      }
    }

    var supsub;

    if (supm && subm) {
      supShift = Math.max(supShift, minSupShift, supm.depth + 0.25 * metrics.xHeight);
      subShift = Math.max(subShift, metrics.sub2);
      var ruleWidth = metrics.defaultRuleThickness; // Rule 18e

      var maxWidth = 4 * ruleWidth;

      if (supShift - supm.depth - (subm.height - subShift) < maxWidth) {
        subShift = maxWidth - (supShift - supm.depth) + subm.height;
        var psi = 0.8 * metrics.xHeight - (supShift - supm.depth);

        if (psi > 0) {
          supShift += psi;
          subShift -= psi;
        }
      }

      var vlistElem = [{
        type: "elem",
        elem: subm,
        shift: subShift,
        marginRight: marginRight,
        marginLeft: marginLeft
      }, {
        type: "elem",
        elem: supm,
        shift: -supShift,
        marginRight: marginRight
      }];
      supsub = buildCommon.makeVList({
        positionType: "individualShift",
        children: vlistElem
      }, options);
    } else if (subm) {
      // Rule 18b
      subShift = Math.max(subShift, metrics.sub1, subm.height - 0.8 * metrics.xHeight);
      var _vlistElem = [{
        type: "elem",
        elem: subm,
        marginLeft: marginLeft,
        marginRight: marginRight
      }];
      supsub = buildCommon.makeVList({
        positionType: "shift",
        positionData: subShift,
        children: _vlistElem
      }, options);
    } else if (supm) {
      // Rule 18c, d
      supShift = Math.max(supShift, minSupShift, supm.depth + 0.25 * metrics.xHeight);
      supsub = buildCommon.makeVList({
        positionType: "shift",
        positionData: -supShift,
        children: [{
          type: "elem",
          elem: supm,
          marginRight: marginRight
        }]
      }, options);
    } else {
      throw new Error("supsub must have either sup or sub.");
    } // Wrap the supsub vlist in a span.msupsub to reset text-align.


    var mclass = getTypeOfDomTree(base, "right") || "mord";
    return buildCommon.makeSpan([mclass], [base, buildCommon.makeSpan(["msupsub"], [supsub])], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    // Is the inner group a relevant horizonal brace?
    var isBrace = false;
    var isOver;
    var isSup;
    var horizBrace = checkNodeType(group.base, "horizBrace");

    if (horizBrace) {
      isSup = !!group.sup;

      if (isSup === horizBrace.isOver) {
        isBrace = true;
        isOver = horizBrace.isOver;
      }
    }

    if (group.base && group.base.type === "op") {
      group.base.parentIsSupSub = true;
    }

    var children = [buildMathML_buildGroup(group.base, options)];

    if (group.sub) {
      children.push(buildMathML_buildGroup(group.sub, options));
    }

    if (group.sup) {
      children.push(buildMathML_buildGroup(group.sup, options));
    }

    var nodeType;

    if (isBrace) {
      nodeType = isOver ? "mover" : "munder";
    } else if (!group.sub) {
      var base = group.base;

      if (base && base.type === "op" && base.limits && (options.style === src_Style.DISPLAY || base.alwaysHandleSupSub)) {
        nodeType = "mover";
      } else {
        nodeType = "msup";
      }
    } else if (!group.sup) {
      var _base = group.base;

      if (_base && _base.type === "op" && _base.limits && (options.style === src_Style.DISPLAY || _base.alwaysHandleSupSub)) {
        nodeType = "munder";
      } else {
        nodeType = "msub";
      }
    } else {
      var _base2 = group.base;

      if (_base2 && _base2.type === "op" && _base2.limits && options.style === src_Style.DISPLAY) {
        nodeType = "munderover";
      } else {
        nodeType = "msubsup";
      }
    }

    var node = new mathMLTree.MathNode(nodeType, children);
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/symbolsOp.js



 // Operator ParseNodes created in Parser.js from symbol Groups in src/symbols.js.

defineFunctionBuilders({
  type: "atom",
  htmlBuilder: function htmlBuilder(group, options) {
    return buildCommon.mathsym(group.text, group.mode, options, ["m" + group.family]);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node = new mathMLTree.MathNode("mo", [buildMathML_makeText(group.text, group.mode)]);

    if (group.family === "bin") {
      var variant = buildMathML_getVariant(group, options);

      if (variant === "bold-italic") {
        node.setAttribute("mathvariant", variant);
      }
    } else if (group.family === "punct") {
      node.setAttribute("separator", "true");
    } else if (group.family === "open" || group.family === "close") {
      // Delims built here should not stretch vertically.
      // See delimsizing.js for stretchy delims.
      node.setAttribute("stretchy", "false");
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/symbolsOrd.js




// "mathord" and "textord" ParseNodes created in Parser.js from symbol Groups in
var defaultVariant = {
  "mi": "italic",
  "mn": "normal",
  "mtext": "normal"
};
defineFunctionBuilders({
  type: "mathord",
  htmlBuilder: function htmlBuilder(group, options) {
    return buildCommon.makeOrd(group, options, "mathord");
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node = new mathMLTree.MathNode("mi", [buildMathML_makeText(group.text, group.mode, options)]);
    var variant = buildMathML_getVariant(group, options) || "italic";

    if (variant !== defaultVariant[node.type]) {
      node.setAttribute("mathvariant", variant);
    }

    return node;
  }
});
defineFunctionBuilders({
  type: "textord",
  htmlBuilder: function htmlBuilder(group, options) {
    return buildCommon.makeOrd(group, options, "textord");
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var text = buildMathML_makeText(group.text, group.mode, options);
    var variant = buildMathML_getVariant(group, options) || "normal";
    var node;

    if (group.mode === 'text') {
      node = new mathMLTree.MathNode("mtext", [text]);
    } else if (/[0-9]/.test(group.text)) {
      // TODO(kevinb) merge adjacent <mn> nodes
      // do it as a post processing step
      node = new mathMLTree.MathNode("mn", [text]);
    } else if (group.text === "\\prime") {
      node = new mathMLTree.MathNode("mo", [text]);
    } else {
      node = new mathMLTree.MathNode("mi", [text]);
    }

    if (variant !== defaultVariant[node.type]) {
      node.setAttribute("mathvariant", variant);
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/symbolsSpacing.js



 // A map of CSS-based spacing functions to their CSS class.

var cssSpace = {
  "\\nobreak": "nobreak",
  "\\allowbreak": "allowbreak"
}; // A lookup table to determine whether a spacing function/symbol should be
// treated like a regular space character.  If a symbol or command is a key
// in this table, then it should be a regular space character.  Furthermore,
// the associated value may have a `className` specifying an extra CSS class
// to add to the created `span`.

var regularSpace = {
  " ": {},
  "\\ ": {},
  "~": {
    className: "nobreak"
  },
  "\\space": {},
  "\\nobreakspace": {
    className: "nobreak"
  }
}; // ParseNode<"spacing"> created in Parser.js from the "spacing" symbol Groups in
// src/symbols.js.

defineFunctionBuilders({
  type: "spacing",
  htmlBuilder: function htmlBuilder(group, options) {
    if (regularSpace.hasOwnProperty(group.text)) {
      var className = regularSpace[group.text].className || ""; // Spaces are generated by adding an actual space. Each of these
      // things has an entry in the symbols table, so these will be turned
      // into appropriate outputs.

      if (group.mode === "text") {
        var ord = buildCommon.makeOrd(group, options, "textord");
        ord.classes.push(className);
        return ord;
      } else {
        return buildCommon.makeSpan(["mspace", className], [buildCommon.mathsym(group.text, group.mode, options)], options);
      }
    } else if (cssSpace.hasOwnProperty(group.text)) {
      // Spaces based on just a CSS class.
      return buildCommon.makeSpan(["mspace", cssSpace[group.text]], [], options);
    } else {
      throw new src_ParseError("Unknown type of space \"" + group.text + "\"");
    }
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var node;

    if (regularSpace.hasOwnProperty(group.text)) {
      node = new mathMLTree.MathNode("mtext", [new mathMLTree.TextNode("\xA0")]);
    } else if (cssSpace.hasOwnProperty(group.text)) {
      // CSS-based MathML spaces (\nobreak, \allowbreak) are ignored
      return new mathMLTree.MathNode("mspace");
    } else {
      throw new src_ParseError("Unknown type of space \"" + group.text + "\"");
    }

    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/tag.js




var tag_pad = function pad() {
  var padNode = new mathMLTree.MathNode("mtd", []);
  padNode.setAttribute("width", "50%");
  return padNode;
};

defineFunctionBuilders({
  type: "tag",
  mathmlBuilder: function mathmlBuilder(group, options) {
    var table = new mathMLTree.MathNode("mtable", [new mathMLTree.MathNode("mtr", [tag_pad(), new mathMLTree.MathNode("mtd", [buildExpressionRow(group.body, options)]), tag_pad(), new mathMLTree.MathNode("mtd", [buildExpressionRow(group.tag, options)])])]);
    table.setAttribute("width", "100%");
    return table; // TODO: Left-aligned tags.
    // Currently, the group and options passed here do not contain
    // enough info to set tag alignment. `leqno` is in Settings but it is
    // not passed to Options. On the HTML side, leqno is
    // set by a CSS class applied in buildTree.js. That would have worked
    // in MathML if browsers supported <mlabeledtr>. Since they don't, we
    // need to rewrite the way this function is called.
  }
});
// CONCATENATED MODULE: ./src/functions/text.js



 // Non-mathy text, possibly in a font

var textFontFamilies = {
  "\\text": undefined,
  "\\textrm": "textrm",
  "\\textsf": "textsf",
  "\\texttt": "texttt",
  "\\textnormal": "textrm"
};
var textFontWeights = {
  "\\textbf": "textbf",
  "\\textmd": "textmd"
};
var textFontShapes = {
  "\\textit": "textit",
  "\\textup": "textup"
};

var optionsWithFont = function optionsWithFont(group, options) {
  var font = group.font; // Checks if the argument is a font family or a font style.

  if (!font) {
    return options;
  } else if (textFontFamilies[font]) {
    return options.withTextFontFamily(textFontFamilies[font]);
  } else if (textFontWeights[font]) {
    return options.withTextFontWeight(textFontWeights[font]);
  } else {
    return options.withTextFontShape(textFontShapes[font]);
  }
};

defineFunction({
  type: "text",
  names: [// Font families
  "\\text", "\\textrm", "\\textsf", "\\texttt", "\\textnormal", // Font weights
  "\\textbf", "\\textmd", // Font Shapes
  "\\textit", "\\textup"],
  props: {
    numArgs: 1,
    argTypes: ["text"],
    greediness: 2,
    allowedInText: true,
    consumeMode: "text"
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser,
        funcName = _ref.funcName;
    var body = args[0];
    return {
      type: "text",
      mode: parser.mode,
      body: defineFunction_ordargument(body),
      font: funcName
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var newOptions = optionsWithFont(group, options);
    var inner = buildHTML_buildExpression(group.body, newOptions, true);
    return buildCommon.makeSpan(["mord", "text"], buildCommon.tryCombineChars(inner), newOptions);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var newOptions = optionsWithFont(group, options);
    return buildExpressionRow(group.body, newOptions);
  }
});
// CONCATENATED MODULE: ./src/functions/underline.js





defineFunction({
  type: "underline",
  names: ["\\underline"],
  props: {
    numArgs: 1,
    allowedInText: true
  },
  handler: function handler(_ref, args) {
    var parser = _ref.parser;
    return {
      type: "underline",
      mode: parser.mode,
      body: args[0]
    };
  },
  htmlBuilder: function htmlBuilder(group, options) {
    // Underlines are handled in the TeXbook pg 443, Rule 10.
    // Build the inner group.
    var innerGroup = buildHTML_buildGroup(group.body, options); // Create the line to go below the body

    var line = buildCommon.makeLineSpan("underline-line", options); // Generate the vlist, with the appropriate kerns

    var vlist = buildCommon.makeVList({
      positionType: "top",
      positionData: innerGroup.height,
      children: [{
        type: "kern",
        size: line.height
      }, {
        type: "elem",
        elem: line
      }, {
        type: "kern",
        size: 3 * line.height
      }, {
        type: "elem",
        elem: innerGroup
      }]
    }, options);
    return buildCommon.makeSpan(["mord", "underline"], [vlist], options);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var operator = new mathMLTree.MathNode("mo", [new mathMLTree.TextNode("\u203E")]);
    operator.setAttribute("stretchy", "true");
    var node = new mathMLTree.MathNode("munder", [buildMathML_buildGroup(group.body, options), operator]);
    node.setAttribute("accentunder", "true");
    return node;
  }
});
// CONCATENATED MODULE: ./src/functions/verb.js




defineFunction({
  type: "verb",
  names: ["\\verb"],
  props: {
    numArgs: 0,
    allowedInText: true
  },
  handler: function handler(context, args, optArgs) {
    // \verb and \verb* are dealt with directly in Parser.js.
    // If we end up here, it's because of a failure to match the two delimiters
    // in the regex in Lexer.js.  LaTeX raises the following error when \verb is
    // terminated by end of line (or file).
    throw new src_ParseError("\\verb ended by end of line instead of matching delimiter");
  },
  htmlBuilder: function htmlBuilder(group, options) {
    var text = makeVerb(group);
    var body = []; // \verb enters text mode and therefore is sized like \textstyle

    var newOptions = options.havingStyle(options.style.text());

    for (var i = 0; i < text.length; i++) {
      var c = text[i];

      if (c === '~') {
        c = '\\textasciitilde';
      }

      body.push(buildCommon.makeSymbol(c, "Typewriter-Regular", group.mode, newOptions, ["mord", "texttt"]));
    }

    return buildCommon.makeSpan(["mord", "text"].concat(newOptions.sizingClasses(options)), buildCommon.tryCombineChars(body), newOptions);
  },
  mathmlBuilder: function mathmlBuilder(group, options) {
    var text = new mathMLTree.TextNode(makeVerb(group));
    var node = new mathMLTree.MathNode("mtext", [text]);
    node.setAttribute("mathvariant", "monospace");
    return node;
  }
});
/**
 * Converts verb group into body string.
 *
 * \verb* replaces each space with an open box \u2423
 * \verb replaces each space with a no-break space \xA0
 */

var makeVerb = function makeVerb(group) {
  return group.body.replace(/ /g, group.star ? "\u2423" : '\xA0');
};
// CONCATENATED MODULE: ./src/functions.js
/** Include this to ensure that all functions are defined. */

var functions = _functions;
/* harmony default export */ var src_functions = (functions); // TODO(kevinb): have functions return an object and call defineFunction with
// that object in this file instead of relying on side-effects.














 // Disabled until https://github.com/KaTeX/KaTeX/pull/1794 is merged.
// import "./functions/includegraphics";

























// CONCATENATED MODULE: ./src/Lexer.js
/**
 * The Lexer class handles tokenizing the input in various ways. Since our
 * parser expects us to be able to backtrack, the lexer allows lexing from any
 * given starting point.
 *
 * Its main exposed function is the `lex` function, which takes a position to
 * lex from and a type of token to lex. It defers to the appropriate `_innerLex`
 * function.
 *
 * The various `_innerLex` functions perform the actual lexing of different
 * kinds.
 */




/* The following tokenRegex
 * - matches typical whitespace (but not NBSP etc.) using its first group
 * - does not match any control character \x00-\x1f except whitespace
 * - does not match a bare backslash
 * - matches any ASCII character except those just mentioned
 * - does not match the BMP private use area \uE000-\uF8FF
 * - does not match bare surrogate code units
 * - matches any BMP character except for those just described
 * - matches any valid Unicode surrogate pair
 * - matches a backslash followed by one or more letters
 * - matches a backslash followed by any BMP character, including newline
 * Just because the Lexer matches something doesn't mean it's valid input:
 * If there is no matching function or symbol definition, the Parser will
 * still reject the input.
 */
var spaceRegexString = "[ \r\n\t]";
var controlWordRegexString = "\\\\[a-zA-Z@]+";
var controlSymbolRegexString = "\\\\[^\uD800-\uDFFF]";
var controlWordWhitespaceRegexString = "" + controlWordRegexString + spaceRegexString + "*";
var controlWordWhitespaceRegex = new RegExp("^(" + controlWordRegexString + ")" + spaceRegexString + "*$");
var combiningDiacriticalMarkString = "[\u0300-\u036F]";
var combiningDiacriticalMarksEndRegex = new RegExp(combiningDiacriticalMarkString + "+$");
var tokenRegexString = "(" + spaceRegexString + "+)|" + // whitespace
"([!-\\[\\]-\u2027\u202A-\uD7FF\uF900-\uFFFF]" + ( // single codepoint
combiningDiacriticalMarkString + "*") + // ...plus accents
"|[\uD800-\uDBFF][\uDC00-\uDFFF]" + ( // surrogate pair
combiningDiacriticalMarkString + "*") + // ...plus accents
"|\\\\verb\\*([^]).*?\\3" + // \verb*
"|\\\\verb([^*a-zA-Z]).*?\\4" + ( // \verb unstarred
"|" + controlWordWhitespaceRegexString) + ( // \macroName + spaces
"|" + controlSymbolRegexString + ")"); // \\, \', etc.

/** Main Lexer class */

var Lexer_Lexer =
/*#__PURE__*/
function () {
  // category codes, only supports comment characters (14) for now
  function Lexer(input, settings) {
    this.input = void 0;
    this.settings = void 0;
    this.tokenRegex = void 0;
    this.catcodes = void 0;
    // Separate accents from characters
    this.input = input;
    this.settings = settings;
    this.tokenRegex = new RegExp(tokenRegexString, 'g');
    this.catcodes = {
      "%": 14 // comment character

    };
  }

  var _proto = Lexer.prototype;

  _proto.setCatcode = function setCatcode(char, code) {
    this.catcodes[char] = code;
  }
  /**
   * This function lexes a single token.
   */
  ;

  _proto.lex = function lex() {
    var input = this.input;
    var pos = this.tokenRegex.lastIndex;

    if (pos === input.length) {
      return new Token_Token("EOF", new SourceLocation(this, pos, pos));
    }

    var match = this.tokenRegex.exec(input);

    if (match === null || match.index !== pos) {
      throw new src_ParseError("Unexpected character: '" + input[pos] + "'", new Token_Token(input[pos], new SourceLocation(this, pos, pos + 1)));
    }

    var text = match[2] || " ";

    if (this.catcodes[text] === 14) {
      // comment character
      var nlIndex = input.indexOf('\n', this.tokenRegex.lastIndex);

      if (nlIndex === -1) {
        this.tokenRegex.lastIndex = input.length; // EOF

        this.settings.reportNonstrict("commentAtEnd", "% comment has no terminating newline; LaTeX would " + "fail because of commenting the end of math mode (e.g. $)");
      } else {
        this.tokenRegex.lastIndex = nlIndex + 1;
      }

      return this.lex();
    } // Trim any trailing whitespace from control word match


    var controlMatch = text.match(controlWordWhitespaceRegex);

    if (controlMatch) {
      text = controlMatch[1];
    }

    return new Token_Token(text, new SourceLocation(this, pos, this.tokenRegex.lastIndex));
  };

  return Lexer;
}();


// CONCATENATED MODULE: ./src/Namespace.js
/**
 * A `Namespace` refers to a space of nameable things like macros or lengths,
 * which can be `set` either globally or local to a nested group, using an
 * undo stack similar to how TeX implements this functionality.
 * Performance-wise, `get` and local `set` take constant time, while global
 * `set` takes time proportional to the depth of group nesting.
 */


var Namespace_Namespace =
/*#__PURE__*/
function () {
  /**
   * Both arguments are optional.  The first argument is an object of
   * built-in mappings which never change.  The second argument is an object
   * of initial (global-level) mappings, which will constantly change
   * according to any global/top-level `set`s done.
   */
  function Namespace(builtins, globalMacros) {
    if (builtins === void 0) {
      builtins = {};
    }

    if (globalMacros === void 0) {
      globalMacros = {};
    }

    this.current = void 0;
    this.builtins = void 0;
    this.undefStack = void 0;
    this.current = globalMacros;
    this.builtins = builtins;
    this.undefStack = [];
  }
  /**
   * Start a new nested group, affecting future local `set`s.
   */


  var _proto = Namespace.prototype;

  _proto.beginGroup = function beginGroup() {
    this.undefStack.push({});
  }
  /**
   * End current nested group, restoring values before the group began.
   */
  ;

  _proto.endGroup = function endGroup() {
    if (this.undefStack.length === 0) {
      throw new src_ParseError("Unbalanced namespace destruction: attempt " + "to pop global namespace; please report this as a bug");
    }

    var undefs = this.undefStack.pop();

    for (var undef in undefs) {
      if (undefs.hasOwnProperty(undef)) {
        if (undefs[undef] === undefined) {
          delete this.current[undef];
        } else {
          this.current[undef] = undefs[undef];
        }
      }
    }
  }
  /**
   * Detect whether `name` has a definition.  Equivalent to
   * `get(name) != null`.
   */
  ;

  _proto.has = function has(name) {
    return this.current.hasOwnProperty(name) || this.builtins.hasOwnProperty(name);
  }
  /**
   * Get the current value of a name, or `undefined` if there is no value.
   *
   * Note: Do not use `if (namespace.get(...))` to detect whether a macro
   * is defined, as the definition may be the empty string which evaluates
   * to `false` in JavaScript.  Use `if (namespace.get(...) != null)` or
   * `if (namespace.has(...))`.
   */
  ;

  _proto.get = function get(name) {
    if (this.current.hasOwnProperty(name)) {
      return this.current[name];
    } else {
      return this.builtins[name];
    }
  }
  /**
   * Set the current value of a name, and optionally set it globally too.
   * Local set() sets the current value and (when appropriate) adds an undo
   * operation to the undo stack.  Global set() may change the undo
   * operation at every level, so takes time linear in their number.
   */
  ;

  _proto.set = function set(name, value, global) {
    if (global === void 0) {
      global = false;
    }

    if (global) {
      // Global set is equivalent to setting in all groups.  Simulate this
      // by destroying any undos currently scheduled for this name,
      // and adding an undo with the *new* value (in case it later gets
      // locally reset within this environment).
      for (var i = 0; i < this.undefStack.length; i++) {
        delete this.undefStack[i][name];
      }

      if (this.undefStack.length > 0) {
        this.undefStack[this.undefStack.length - 1][name] = value;
      }
    } else {
      // Undo this set at end of this group (possibly to `undefined`),
      // unless an undo is already in place, in which case that older
      // value is the correct one.
      var top = this.undefStack[this.undefStack.length - 1];

      if (top && !top.hasOwnProperty(name)) {
        top[name] = this.current[name];
      }
    }

    this.current[name] = value;
  };

  return Namespace;
}();


// CONCATENATED MODULE: ./src/macros.js
/**
 * Predefined macros for KaTeX.
 * This can be used to define some commands in terms of others.
 */





var builtinMacros = {};
/* harmony default export */ var macros = (builtinMacros); // This function might one day accept an additional argument and do more things.

function defineMacro(name, body) {
  builtinMacros[name] = body;
} //////////////////////////////////////////////////////////////////////
// macro tools
// LaTeX's \@firstoftwo{#1}{#2} expands to #1, skipping #2
// TeX source: \long\def\@firstoftwo#1#2{#1}

defineMacro("\\@firstoftwo", function (context) {
  var args = context.consumeArgs(2);
  return {
    tokens: args[0],
    numArgs: 0
  };
}); // LaTeX's \@secondoftwo{#1}{#2} expands to #2, skipping #1
// TeX source: \long\def\@secondoftwo#1#2{#2}

defineMacro("\\@secondoftwo", function (context) {
  var args = context.consumeArgs(2);
  return {
    tokens: args[1],
    numArgs: 0
  };
}); // LaTeX's \@ifnextchar{#1}{#2}{#3} looks ahead to the next (unexpanded)
// symbol.  If it matches #1, then the macro expands to #2; otherwise, #3.
// Note, however, that it does not consume the next symbol in either case.

defineMacro("\\@ifnextchar", function (context) {
  var args = context.consumeArgs(3); // symbol, if, else

  var nextToken = context.future();

  if (args[0].length === 1 && args[0][0].text === nextToken.text) {
    return {
      tokens: args[1],
      numArgs: 0
    };
  } else {
    return {
      tokens: args[2],
      numArgs: 0
    };
  }
}); // LaTeX's \@ifstar{#1}{#2} looks ahead to the next (unexpanded) symbol.
// If it is `*`, then it consumes the symbol, and the macro expands to #1;
// otherwise, the macro expands to #2 (without consuming the symbol).
// TeX source: \def\@ifstar#1{\@ifnextchar *{\@firstoftwo{#1}}}

defineMacro("\\@ifstar", "\\@ifnextchar *{\\@firstoftwo{#1}}"); // LaTeX's \TextOrMath{#1}{#2} expands to #1 in text mode, #2 in math mode

defineMacro("\\TextOrMath", function (context) {
  var args = context.consumeArgs(2);

  if (context.mode === 'text') {
    return {
      tokens: args[0],
      numArgs: 0
    };
  } else {
    return {
      tokens: args[1],
      numArgs: 0
    };
  }
}); // Lookup table for parsing numbers in base 8 through 16

var digitToNumber = {
  "0": 0,
  "1": 1,
  "2": 2,
  "3": 3,
  "4": 4,
  "5": 5,
  "6": 6,
  "7": 7,
  "8": 8,
  "9": 9,
  "a": 10,
  "A": 10,
  "b": 11,
  "B": 11,
  "c": 12,
  "C": 12,
  "d": 13,
  "D": 13,
  "e": 14,
  "E": 14,
  "f": 15,
  "F": 15
}; // TeX \char makes a literal character (catcode 12) using the following forms:
// (see The TeXBook, p. 43)
//   \char123  -- decimal
//   \char'123 -- octal
//   \char"123 -- hex
//   \char`x   -- character that can be written (i.e. isn't active)
//   \char`\x  -- character that cannot be written (e.g. %)
// These all refer to characters from the font, so we turn them into special
// calls to a function \@char dealt with in the Parser.

defineMacro("\\char", function (context) {
  var token = context.popToken();
  var base;
  var number = '';

  if (token.text === "'") {
    base = 8;
    token = context.popToken();
  } else if (token.text === '"') {
    base = 16;
    token = context.popToken();
  } else if (token.text === "`") {
    token = context.popToken();

    if (token.text[0] === "\\") {
      number = token.text.charCodeAt(1);
    } else if (token.text === "EOF") {
      throw new src_ParseError("\\char` missing argument");
    } else {
      number = token.text.charCodeAt(0);
    }
  } else {
    base = 10;
  }

  if (base) {
    // Parse a number in the given base, starting with first `token`.
    number = digitToNumber[token.text];

    if (number == null || number >= base) {
      throw new src_ParseError("Invalid base-" + base + " digit " + token.text);
    }

    var digit;

    while ((digit = digitToNumber[context.future().text]) != null && digit < base) {
      number *= base;
      number += digit;
      context.popToken();
    }
  }

  return "\\@char{" + number + "}";
}); // Basic support for macro definitions:
//     \def\macro{expansion}
//     \def\macro#1{expansion}
//     \def\macro#1#2{expansion}
//     \def\macro#1#2#3#4#5#6#7#8#9{expansion}
// Also the \gdef and \global\def equivalents

var macros_def = function def(context, global) {
  var arg = context.consumeArgs(1)[0];

  if (arg.length !== 1) {
    throw new src_ParseError("\\gdef's first argument must be a macro name");
  }

  var name = arg[0].text; // Count argument specifiers, and check they are in the order #1 #2 ...

  var numArgs = 0;
  arg = context.consumeArgs(1)[0];

  while (arg.length === 1 && arg[0].text === "#") {
    arg = context.consumeArgs(1)[0];

    if (arg.length !== 1) {
      throw new src_ParseError("Invalid argument number length \"" + arg.length + "\"");
    }

    if (!/^[1-9]$/.test(arg[0].text)) {
      throw new src_ParseError("Invalid argument number \"" + arg[0].text + "\"");
    }

    numArgs++;

    if (parseInt(arg[0].text) !== numArgs) {
      throw new src_ParseError("Argument number \"" + arg[0].text + "\" out of order");
    }

    arg = context.consumeArgs(1)[0];
  } // Final arg is the expansion of the macro


  context.macros.set(name, {
    tokens: arg,
    numArgs: numArgs
  }, global);
  return '';
};

defineMacro("\\gdef", function (context) {
  return macros_def(context, true);
});
defineMacro("\\def", function (context) {
  return macros_def(context, false);
});
defineMacro("\\global", function (context) {
  var next = context.consumeArgs(1)[0];

  if (next.length !== 1) {
    throw new src_ParseError("Invalid command after \\global");
  }

  var command = next[0].text; // TODO: Should expand command

  if (command === "\\def") {
    // \global\def is equivalent to \gdef
    return macros_def(context, true);
  } else {
    throw new src_ParseError("Invalid command '" + command + "' after \\global");
  }
}); // \newcommand{\macro}[args]{definition}
// \renewcommand{\macro}[args]{definition}
// TODO: Optional arguments: \newcommand{\macro}[args][default]{definition}

var macros_newcommand = function newcommand(context, existsOK, nonexistsOK) {
  var arg = context.consumeArgs(1)[0];

  if (arg.length !== 1) {
    throw new src_ParseError("\\newcommand's first argument must be a macro name");
  }

  var name = arg[0].text;
  var exists = context.isDefined(name);

  if (exists && !existsOK) {
    throw new src_ParseError("\\newcommand{" + name + "} attempting to redefine " + (name + "; use \\renewcommand"));
  }

  if (!exists && !nonexistsOK) {
    throw new src_ParseError("\\renewcommand{" + name + "} when command " + name + " " + "does not yet exist; use \\newcommand");
  }

  var numArgs = 0;
  arg = context.consumeArgs(1)[0];

  if (arg.length === 1 && arg[0].text === "[") {
    var argText = '';
    var token = context.expandNextToken();

    while (token.text !== "]" && token.text !== "EOF") {
      // TODO: Should properly expand arg, e.g., ignore {}s
      argText += token.text;
      token = context.expandNextToken();
    }

    if (!argText.match(/^\s*[0-9]+\s*$/)) {
      throw new src_ParseError("Invalid number of arguments: " + argText);
    }

    numArgs = parseInt(argText);
    arg = context.consumeArgs(1)[0];
  } // Final arg is the expansion of the macro


  context.macros.set(name, {
    tokens: arg,
    numArgs: numArgs
  });
  return '';
};

defineMacro("\\newcommand", function (context) {
  return macros_newcommand(context, false, true);
});
defineMacro("\\renewcommand", function (context) {
  return macros_newcommand(context, true, false);
});
defineMacro("\\providecommand", function (context) {
  return macros_newcommand(context, true, true);
}); //////////////////////////////////////////////////////////////////////
// Grouping
// \let\bgroup={ \let\egroup=}

defineMacro("\\bgroup", "{");
defineMacro("\\egroup", "}"); // Symbols from latex.ltx:
// \def\lq{`}
// \def\rq{'}
// \def \aa {\r a}
// \def \AA {\r A}

defineMacro("\\lq", "`");
defineMacro("\\rq", "'");
defineMacro("\\aa", "\\r a");
defineMacro("\\AA", "\\r A"); // Copyright (C) and registered (R) symbols. Use raw symbol in MathML.
// \DeclareTextCommandDefault{\textcopyright}{\textcircled{c}}
// \DeclareTextCommandDefault{\textregistered}{\textcircled{%
//      \check@mathfonts\fontsize\sf@size\z@\math@fontsfalse\selectfont R}}
// \DeclareRobustCommand{\copyright}{%
//    \ifmmode{\nfss@text{\textcopyright}}\else\textcopyright\fi}

defineMacro("\\textcopyright", "\\html@mathml{\\textcircled{c}}{\\char`©}");
defineMacro("\\copyright", "\\TextOrMath{\\textcopyright}{\\text{\\textcopyright}}");
defineMacro("\\textregistered", "\\html@mathml{\\textcircled{\\scriptsize R}}{\\char`®}"); // Characters omitted from Unicode range 1D400–1D7FF

defineMacro("\u212C", "\\mathscr{B}"); // script

defineMacro("\u2130", "\\mathscr{E}");
defineMacro("\u2131", "\\mathscr{F}");
defineMacro("\u210B", "\\mathscr{H}");
defineMacro("\u2110", "\\mathscr{I}");
defineMacro("\u2112", "\\mathscr{L}");
defineMacro("\u2133", "\\mathscr{M}");
defineMacro("\u211B", "\\mathscr{R}");
defineMacro("\u212D", "\\mathfrak{C}"); // Fraktur

defineMacro("\u210C", "\\mathfrak{H}");
defineMacro("\u2128", "\\mathfrak{Z}"); // Define \Bbbk with a macro that works in both HTML and MathML.

defineMacro("\\Bbbk", "\\Bbb{k}"); // Unicode middle dot
// The KaTeX fonts do not contain U+00B7. Instead, \cdotp displays
// the dot at U+22C5 and gives it punct spacing.

defineMacro("\xB7", "\\cdotp"); // \llap and \rlap render their contents in text mode

defineMacro("\\llap", "\\mathllap{\\textrm{#1}}");
defineMacro("\\rlap", "\\mathrlap{\\textrm{#1}}");
defineMacro("\\clap", "\\mathclap{\\textrm{#1}}"); // \not is defined by base/fontmath.ltx via
// \DeclareMathSymbol{\not}{\mathrel}{symbols}{"36}
// It's thus treated like a \mathrel, but defined by a symbol that has zero
// width but extends to the right.  We use \rlap to get that spacing.
// For MathML we write U+0338 here. buildMathML.js will then do the overlay.

defineMacro("\\not", '\\html@mathml{\\mathrel{\\mathrlap\\@not}}{\\char"338}'); // Negated symbols from base/fontmath.ltx:
// \def\neq{\not=} \let\ne=\neq
// \DeclareRobustCommand
//   \notin{\mathrel{\m@th\mathpalette\c@ncel\in}}
// \def\c@ncel#1#2{\m@th\ooalign{$\hfil#1\mkern1mu/\hfil$\crcr$#1#2$}}

defineMacro("\\neq", "\\html@mathml{\\mathrel{\\not=}}{\\mathrel{\\char`≠}}");
defineMacro("\\ne", "\\neq");
defineMacro("\u2260", "\\neq");
defineMacro("\\notin", "\\html@mathml{\\mathrel{{\\in}\\mathllap{/\\mskip1mu}}}" + "{\\mathrel{\\char`∉}}");
defineMacro("\u2209", "\\notin"); // Unicode stacked relations

defineMacro("\u2258", "\\html@mathml{" + "\\mathrel{=\\kern{-1em}\\raisebox{0.4em}{$\\scriptsize\\frown$}}" + "}{\\mathrel{\\char`\u2258}}");
defineMacro("\u2259", "\\html@mathml{\\stackrel{\\tiny\\wedge}{=}}{\\mathrel{\\char`\u2258}}");
defineMacro("\u225A", "\\html@mathml{\\stackrel{\\tiny\\vee}{=}}{\\mathrel{\\char`\u225A}}");
defineMacro("\u225B", "\\html@mathml{\\stackrel{\\scriptsize\\star}{=}}" + "{\\mathrel{\\char`\u225B}}");
defineMacro("\u225D", "\\html@mathml{\\stackrel{\\tiny\\mathrm{def}}{=}}" + "{\\mathrel{\\char`\u225D}}");
defineMacro("\u225E", "\\html@mathml{\\stackrel{\\tiny\\mathrm{m}}{=}}" + "{\\mathrel{\\char`\u225E}}");
defineMacro("\u225F", "\\html@mathml{\\stackrel{\\tiny?}{=}}{\\mathrel{\\char`\u225F}}"); // Misc Unicode

defineMacro("\u27C2", "\\perp");
defineMacro("\u203C", "\\mathclose{!\\mkern-0.8mu!}");
defineMacro("\u220C", "\\notni");
defineMacro("\u231C", "\\ulcorner");
defineMacro("\u231D", "\\urcorner");
defineMacro("\u231E", "\\llcorner");
defineMacro("\u231F", "\\lrcorner");
defineMacro("\xA9", "\\copyright");
defineMacro("\xAE", "\\textregistered");
defineMacro("\uFE0F", "\\textregistered"); //////////////////////////////////////////////////////////////////////
// LaTeX_2ε
// \vdots{\vbox{\baselineskip4\p@  \lineskiplimit\z@
// \kern6\p@\hbox{.}\hbox{.}\hbox{.}}}
// We'll call \varvdots, which gets a glyph from symbols.js.
// The zero-width rule gets us an equivalent to the vertical 6pt kern.

defineMacro("\\vdots", "\\mathord{\\varvdots\\rule{0pt}{15pt}}");
defineMacro("\u22EE", "\\vdots"); //////////////////////////////////////////////////////////////////////
// amsmath.sty
// http://mirrors.concertpass.com/tex-archive/macros/latex/required/amsmath/amsmath.pdf
// Italic Greek capital letters.  AMS defines these with \DeclareMathSymbol,
// but they are equivalent to \mathit{\Letter}.

defineMacro("\\varGamma", "\\mathit{\\Gamma}");
defineMacro("\\varDelta", "\\mathit{\\Delta}");
defineMacro("\\varTheta", "\\mathit{\\Theta}");
defineMacro("\\varLambda", "\\mathit{\\Lambda}");
defineMacro("\\varXi", "\\mathit{\\Xi}");
defineMacro("\\varPi", "\\mathit{\\Pi}");
defineMacro("\\varSigma", "\\mathit{\\Sigma}");
defineMacro("\\varUpsilon", "\\mathit{\\Upsilon}");
defineMacro("\\varPhi", "\\mathit{\\Phi}");
defineMacro("\\varPsi", "\\mathit{\\Psi}");
defineMacro("\\varOmega", "\\mathit{\\Omega}"); // \renewcommand{\colon}{\nobreak\mskip2mu\mathpunct{}\nonscript
// \mkern-\thinmuskip{:}\mskip6muplus1mu\relax}

defineMacro("\\colon", "\\nobreak\\mskip2mu\\mathpunct{}" + "\\mathchoice{\\mkern-3mu}{\\mkern-3mu}{}{}{:}\\mskip6mu"); // \newcommand{\boxed}[1]{\fbox{\m@th$\displaystyle#1$}}

defineMacro("\\boxed", "\\fbox{$\\displaystyle{#1}$}"); // \def\iff{\DOTSB\;\Longleftrightarrow\;}
// \def\implies{\DOTSB\;\Longrightarrow\;}
// \def\impliedby{\DOTSB\;\Longleftarrow\;}

defineMacro("\\iff", "\\DOTSB\\;\\Longleftrightarrow\\;");
defineMacro("\\implies", "\\DOTSB\\;\\Longrightarrow\\;");
defineMacro("\\impliedby", "\\DOTSB\\;\\Longleftarrow\\;"); // AMSMath's automatic \dots, based on \mdots@@ macro.

var dotsByToken = {
  ',': '\\dotsc',
  '\\not': '\\dotsb',
  // \keybin@ checks for the following:
  '+': '\\dotsb',
  '=': '\\dotsb',
  '<': '\\dotsb',
  '>': '\\dotsb',
  '-': '\\dotsb',
  '*': '\\dotsb',
  ':': '\\dotsb',
  // Symbols whose definition starts with \DOTSB:
  '\\DOTSB': '\\dotsb',
  '\\coprod': '\\dotsb',
  '\\bigvee': '\\dotsb',
  '\\bigwedge': '\\dotsb',
  '\\biguplus': '\\dotsb',
  '\\bigcap': '\\dotsb',
  '\\bigcup': '\\dotsb',
  '\\prod': '\\dotsb',
  '\\sum': '\\dotsb',
  '\\bigotimes': '\\dotsb',
  '\\bigoplus': '\\dotsb',
  '\\bigodot': '\\dotsb',
  '\\bigsqcup': '\\dotsb',
  '\\And': '\\dotsb',
  '\\longrightarrow': '\\dotsb',
  '\\Longrightarrow': '\\dotsb',
  '\\longleftarrow': '\\dotsb',
  '\\Longleftarrow': '\\dotsb',
  '\\longleftrightarrow': '\\dotsb',
  '\\Longleftrightarrow': '\\dotsb',
  '\\mapsto': '\\dotsb',
  '\\longmapsto': '\\dotsb',
  '\\hookrightarrow': '\\dotsb',
  '\\doteq': '\\dotsb',
  // Symbols whose definition starts with \mathbin:
  '\\mathbin': '\\dotsb',
  // Symbols whose definition starts with \mathrel:
  '\\mathrel': '\\dotsb',
  '\\relbar': '\\dotsb',
  '\\Relbar': '\\dotsb',
  '\\xrightarrow': '\\dotsb',
  '\\xleftarrow': '\\dotsb',
  // Symbols whose definition starts with \DOTSI:
  '\\DOTSI': '\\dotsi',
  '\\int': '\\dotsi',
  '\\oint': '\\dotsi',
  '\\iint': '\\dotsi',
  '\\iiint': '\\dotsi',
  '\\iiiint': '\\dotsi',
  '\\idotsint': '\\dotsi',
  // Symbols whose definition starts with \DOTSX:
  '\\DOTSX': '\\dotsx'
};
defineMacro("\\dots", function (context) {
  // TODO: If used in text mode, should expand to \textellipsis.
  // However, in KaTeX, \textellipsis and \ldots behave the same
  // (in text mode), and it's unlikely we'd see any of the math commands
  // that affect the behavior of \dots when in text mode.  So fine for now
  // (until we support \ifmmode ... \else ... \fi).
  var thedots = '\\dotso';
  var next = context.expandAfterFuture().text;

  if (next in dotsByToken) {
    thedots = dotsByToken[next];
  } else if (next.substr(0, 4) === '\\not') {
    thedots = '\\dotsb';
  } else if (next in src_symbols.math) {
    if (utils.contains(['bin', 'rel'], src_symbols.math[next].group)) {
      thedots = '\\dotsb';
    }
  }

  return thedots;
});
var spaceAfterDots = {
  // \rightdelim@ checks for the following:
  ')': true,
  ']': true,
  '\\rbrack': true,
  '\\}': true,
  '\\rbrace': true,
  '\\rangle': true,
  '\\rceil': true,
  '\\rfloor': true,
  '\\rgroup': true,
  '\\rmoustache': true,
  '\\right': true,
  '\\bigr': true,
  '\\biggr': true,
  '\\Bigr': true,
  '\\Biggr': true,
  // \extra@ also tests for the following:
  '$': true,
  // \extrap@ checks for the following:
  ';': true,
  '.': true,
  ',': true
};
defineMacro("\\dotso", function (context) {
  var next = context.future().text;

  if (next in spaceAfterDots) {
    return "\\ldots\\,";
  } else {
    return "\\ldots";
  }
});
defineMacro("\\dotsc", function (context) {
  var next = context.future().text; // \dotsc uses \extra@ but not \extrap@, instead specially checking for
  // ';' and '.', but doesn't check for ','.

  if (next in spaceAfterDots && next !== ',') {
    return "\\ldots\\,";
  } else {
    return "\\ldots";
  }
});
defineMacro("\\cdots", function (context) {
  var next = context.future().text;

  if (next in spaceAfterDots) {
    return "\\@cdots\\,";
  } else {
    return "\\@cdots";
  }
});
defineMacro("\\dotsb", "\\cdots");
defineMacro("\\dotsm", "\\cdots");
defineMacro("\\dotsi", "\\!\\cdots"); // amsmath doesn't actually define \dotsx, but \dots followed by a macro
// starting with \DOTSX implies \dotso, and then \extra@ detects this case
// and forces the added `\,`.

defineMacro("\\dotsx", "\\ldots\\,"); // \let\DOTSI\relax
// \let\DOTSB\relax
// \let\DOTSX\relax

defineMacro("\\DOTSI", "\\relax");
defineMacro("\\DOTSB", "\\relax");
defineMacro("\\DOTSX", "\\relax"); // Spacing, based on amsmath.sty's override of LaTeX defaults
// \DeclareRobustCommand{\tmspace}[3]{%
//   \ifmmode\mskip#1#2\else\kern#1#3\fi\relax}

defineMacro("\\tmspace", "\\TextOrMath{\\kern#1#3}{\\mskip#1#2}\\relax"); // \renewcommand{\,}{\tmspace+\thinmuskip{.1667em}}
// TODO: math mode should use \thinmuskip

defineMacro("\\,", "\\tmspace+{3mu}{.1667em}"); // \let\thinspace\,

defineMacro("\\thinspace", "\\,"); // \def\>{\mskip\medmuskip}
// \renewcommand{\:}{\tmspace+\medmuskip{.2222em}}
// TODO: \> and math mode of \: should use \medmuskip = 4mu plus 2mu minus 4mu

defineMacro("\\>", "\\mskip{4mu}");
defineMacro("\\:", "\\tmspace+{4mu}{.2222em}"); // \let\medspace\:

defineMacro("\\medspace", "\\:"); // \renewcommand{\;}{\tmspace+\thickmuskip{.2777em}}
// TODO: math mode should use \thickmuskip = 5mu plus 5mu

defineMacro("\\;", "\\tmspace+{5mu}{.2777em}"); // \let\thickspace\;

defineMacro("\\thickspace", "\\;"); // \renewcommand{\!}{\tmspace-\thinmuskip{.1667em}}
// TODO: math mode should use \thinmuskip

defineMacro("\\!", "\\tmspace-{3mu}{.1667em}"); // \let\negthinspace\!

defineMacro("\\negthinspace", "\\!"); // \newcommand{\negmedspace}{\tmspace-\medmuskip{.2222em}}
// TODO: math mode should use \medmuskip

defineMacro("\\negmedspace", "\\tmspace-{4mu}{.2222em}"); // \newcommand{\negthickspace}{\tmspace-\thickmuskip{.2777em}}
// TODO: math mode should use \thickmuskip

defineMacro("\\negthickspace", "\\tmspace-{5mu}{.277em}"); // \def\enspace{\kern.5em }

defineMacro("\\enspace", "\\kern.5em "); // \def\enskip{\hskip.5em\relax}

defineMacro("\\enskip", "\\hskip.5em\\relax"); // \def\quad{\hskip1em\relax}

defineMacro("\\quad", "\\hskip1em\\relax"); // \def\qquad{\hskip2em\relax}

defineMacro("\\qquad", "\\hskip2em\\relax"); // \tag@in@display form of \tag

defineMacro("\\tag", "\\@ifstar\\tag@literal\\tag@paren");
defineMacro("\\tag@paren", "\\tag@literal{({#1})}");
defineMacro("\\tag@literal", function (context) {
  if (context.macros.get("\\df@tag")) {
    throw new src_ParseError("Multiple \\tag");
  }

  return "\\gdef\\df@tag{\\text{#1}}";
}); // \renewcommand{\bmod}{\nonscript\mskip-\medmuskip\mkern5mu\mathbin
//   {\operator@font mod}\penalty900
//   \mkern5mu\nonscript\mskip-\medmuskip}
// \newcommand{\pod}[1]{\allowbreak
//   \if@display\mkern18mu\else\mkern8mu\fi(#1)}
// \renewcommand{\pmod}[1]{\pod{{\operator@font mod}\mkern6mu#1}}
// \newcommand{\mod}[1]{\allowbreak\if@display\mkern18mu
//   \else\mkern12mu\fi{\operator@font mod}\,\,#1}
// TODO: math mode should use \medmuskip = 4mu plus 2mu minus 4mu

defineMacro("\\bmod", "\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}" + "\\mathbin{\\rm mod}" + "\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}");
defineMacro("\\pod", "\\allowbreak" + "\\mathchoice{\\mkern18mu}{\\mkern8mu}{\\mkern8mu}{\\mkern8mu}(#1)");
defineMacro("\\pmod", "\\pod{{\\rm mod}\\mkern6mu#1}");
defineMacro("\\mod", "\\allowbreak" + "\\mathchoice{\\mkern18mu}{\\mkern12mu}{\\mkern12mu}{\\mkern12mu}" + "{\\rm mod}\\,\\,#1"); // \pmb    --   A simulation of bold.
// It works by typesetting three copies of the argument with small offsets.
// Ref: a rather lengthy macro in ambsy.sty

defineMacro("\\pmb", "\\html@mathml{\\@binrel{#1}{" + "\\mathrlap{#1}" + "\\mathrlap{\\mkern0.4mu\\raisebox{0.4mu}{$#1$}}" + "{\\mkern0.8mu#1}" + "}}{\\mathbf{#1}}"); //////////////////////////////////////////////////////////////////////
// LaTeX source2e
// \\ defaults to \newline, but changes to \cr within array environment

defineMacro("\\\\", "\\newline"); // \def\TeX{T\kern-.1667em\lower.5ex\hbox{E}\kern-.125emX\@}
// TODO: Doesn't normally work in math mode because \@ fails.  KaTeX doesn't
// support \@ yet, so that's omitted, and we add \text so that the result
// doesn't look funny in math mode.

defineMacro("\\TeX", "\\textrm{\\html@mathml{" + "T\\kern-.1667em\\raisebox{-.5ex}{E}\\kern-.125emX" + "}{TeX}}"); // \DeclareRobustCommand{\LaTeX}{L\kern-.36em%
//         {\sbox\z@ T%
//          \vbox to\ht\z@{\hbox{\check@mathfonts
//                               \fontsize\sf@size\z@
//                               \math@fontsfalse\selectfont
//                               A}%
//                         \vss}%
//         }%
//         \kern-.15em%
//         \TeX}
// This code aligns the top of the A with the T (from the perspective of TeX's
// boxes, though visually the A appears to extend above slightly).
// We compute the corresponding \raisebox when A is rendered at \scriptsize,
// which is size3, which has a scale factor of 0.7 (see Options.js).

var latexRaiseA = fontMetricsData['Main-Regular']["T".charCodeAt(0)][1] - 0.7 * fontMetricsData['Main-Regular']["A".charCodeAt(0)][1] + "em";
defineMacro("\\LaTeX", "\\textrm{\\html@mathml{" + ("L\\kern-.36em\\raisebox{" + latexRaiseA + "}{\\scriptsize A}") + "\\kern-.15em\\TeX}{LaTeX}}"); // New KaTeX logo based on tweaking LaTeX logo

defineMacro("\\KaTeX", "\\textrm{\\html@mathml{" + ("K\\kern-.17em\\raisebox{" + latexRaiseA + "}{\\scriptsize A}") + "\\kern-.15em\\TeX}{KaTeX}}"); // \DeclareRobustCommand\hspace{\@ifstar\@hspacer\@hspace}
// \def\@hspace#1{\hskip  #1\relax}
// \def\@hspacer#1{\vrule \@width\z@\nobreak
//                 \hskip #1\hskip \z@skip}

defineMacro("\\hspace", "\\@ifstar\\@hspacer\\@hspace");
defineMacro("\\@hspace", "\\hskip #1\\relax");
defineMacro("\\@hspacer", "\\rule{0pt}{0pt}\\hskip #1\\relax"); //////////////////////////////////////////////////////////////////////
// mathtools.sty
//\providecommand\ordinarycolon{:}

defineMacro("\\ordinarycolon", ":"); //\def\vcentcolon{\mathrel{\mathop\ordinarycolon}}
//TODO(edemaine): Not yet centered. Fix via \raisebox or #726

defineMacro("\\vcentcolon", "\\mathrel{\\mathop\\ordinarycolon}"); // \providecommand*\dblcolon{\vcentcolon\mathrel{\mkern-.9mu}\vcentcolon}

defineMacro("\\dblcolon", "\\html@mathml{" + "\\mathrel{\\vcentcolon\\mathrel{\\mkern-.9mu}\\vcentcolon}}" + "{\\mathop{\\char\"2237}}"); // \providecommand*\coloneqq{\vcentcolon\mathrel{\mkern-1.2mu}=}

defineMacro("\\coloneqq", "\\html@mathml{" + "\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}=}}" + "{\\mathop{\\char\"2254}}"); // ≔
// \providecommand*\Coloneqq{\dblcolon\mathrel{\mkern-1.2mu}=}

defineMacro("\\Coloneqq", "\\html@mathml{" + "\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}=}}" + "{\\mathop{\\char\"2237\\char\"3d}}"); // \providecommand*\coloneq{\vcentcolon\mathrel{\mkern-1.2mu}\mathrel{-}}

defineMacro("\\coloneq", "\\html@mathml{" + "\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}" + "{\\mathop{\\char\"3a\\char\"2212}}"); // \providecommand*\Coloneq{\dblcolon\mathrel{\mkern-1.2mu}\mathrel{-}}

defineMacro("\\Coloneq", "\\html@mathml{" + "\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}" + "{\\mathop{\\char\"2237\\char\"2212}}"); // \providecommand*\eqqcolon{=\mathrel{\mkern-1.2mu}\vcentcolon}

defineMacro("\\eqqcolon", "\\html@mathml{" + "\\mathrel{=\\mathrel{\\mkern-1.2mu}\\vcentcolon}}" + "{\\mathop{\\char\"2255}}"); // ≕
// \providecommand*\Eqqcolon{=\mathrel{\mkern-1.2mu}\dblcolon}

defineMacro("\\Eqqcolon", "\\html@mathml{" + "\\mathrel{=\\mathrel{\\mkern-1.2mu}\\dblcolon}}" + "{\\mathop{\\char\"3d\\char\"2237}}"); // \providecommand*\eqcolon{\mathrel{-}\mathrel{\mkern-1.2mu}\vcentcolon}

defineMacro("\\eqcolon", "\\html@mathml{" + "\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\vcentcolon}}" + "{\\mathop{\\char\"2239}}"); // \providecommand*\Eqcolon{\mathrel{-}\mathrel{\mkern-1.2mu}\dblcolon}

defineMacro("\\Eqcolon", "\\html@mathml{" + "\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\dblcolon}}" + "{\\mathop{\\char\"2212\\char\"2237}}"); // \providecommand*\colonapprox{\vcentcolon\mathrel{\mkern-1.2mu}\approx}

defineMacro("\\colonapprox", "\\html@mathml{" + "\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\approx}}" + "{\\mathop{\\char\"3a\\char\"2248}}"); // \providecommand*\Colonapprox{\dblcolon\mathrel{\mkern-1.2mu}\approx}

defineMacro("\\Colonapprox", "\\html@mathml{" + "\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\approx}}" + "{\\mathop{\\char\"2237\\char\"2248}}"); // \providecommand*\colonsim{\vcentcolon\mathrel{\mkern-1.2mu}\sim}

defineMacro("\\colonsim", "\\html@mathml{" + "\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\sim}}" + "{\\mathop{\\char\"3a\\char\"223c}}"); // \providecommand*\Colonsim{\dblcolon\mathrel{\mkern-1.2mu}\sim}

defineMacro("\\Colonsim", "\\html@mathml{" + "\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\sim}}" + "{\\mathop{\\char\"2237\\char\"223c}}"); // Some Unicode characters are implemented with macros to mathtools functions.

defineMacro("\u2237", "\\dblcolon"); // ::

defineMacro("\u2239", "\\eqcolon"); // -:

defineMacro("\u2254", "\\coloneqq"); // :=

defineMacro("\u2255", "\\eqqcolon"); // =:

defineMacro("\u2A74", "\\Coloneqq"); // ::=
//////////////////////////////////////////////////////////////////////
// colonequals.sty
// Alternate names for mathtools's macros:

defineMacro("\\ratio", "\\vcentcolon");
defineMacro("\\coloncolon", "\\dblcolon");
defineMacro("\\colonequals", "\\coloneqq");
defineMacro("\\coloncolonequals", "\\Coloneqq");
defineMacro("\\equalscolon", "\\eqqcolon");
defineMacro("\\equalscoloncolon", "\\Eqqcolon");
defineMacro("\\colonminus", "\\coloneq");
defineMacro("\\coloncolonminus", "\\Coloneq");
defineMacro("\\minuscolon", "\\eqcolon");
defineMacro("\\minuscoloncolon", "\\Eqcolon"); // \colonapprox name is same in mathtools and colonequals.

defineMacro("\\coloncolonapprox", "\\Colonapprox"); // \colonsim name is same in mathtools and colonequals.

defineMacro("\\coloncolonsim", "\\Colonsim"); // Additional macros, implemented by analogy with mathtools definitions:

defineMacro("\\simcolon", "\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\vcentcolon}");
defineMacro("\\simcoloncolon", "\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\dblcolon}");
defineMacro("\\approxcolon", "\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\vcentcolon}");
defineMacro("\\approxcoloncolon", "\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\dblcolon}"); // Present in newtxmath, pxfonts and txfonts

defineMacro("\\notni", "\\html@mathml{\\not\\ni}{\\mathrel{\\char`\u220C}}");
defineMacro("\\limsup", "\\DOTSB\\mathop{\\operatorname{lim\\,sup}}\\limits");
defineMacro("\\liminf", "\\DOTSB\\mathop{\\operatorname{lim\\,inf}}\\limits"); //////////////////////////////////////////////////////////////////////
// MathML alternates for KaTeX glyphs in the Unicode private area

defineMacro("\\gvertneqq", "\\html@mathml{\\@gvertneqq}{\u2269}");
defineMacro("\\lvertneqq", "\\html@mathml{\\@lvertneqq}{\u2268}");
defineMacro("\\ngeqq", "\\html@mathml{\\@ngeqq}{\u2271}");
defineMacro("\\ngeqslant", "\\html@mathml{\\@ngeqslant}{\u2271}");
defineMacro("\\nleqq", "\\html@mathml{\\@nleqq}{\u2270}");
defineMacro("\\nleqslant", "\\html@mathml{\\@nleqslant}{\u2270}");
defineMacro("\\nshortmid", "\\html@mathml{\\@nshortmid}{∤}");
defineMacro("\\nshortparallel", "\\html@mathml{\\@nshortparallel}{∦}");
defineMacro("\\nsubseteqq", "\\html@mathml{\\@nsubseteqq}{\u2288}");
defineMacro("\\nsupseteqq", "\\html@mathml{\\@nsupseteqq}{\u2289}");
defineMacro("\\varsubsetneq", "\\html@mathml{\\@varsubsetneq}{⊊}");
defineMacro("\\varsubsetneqq", "\\html@mathml{\\@varsubsetneqq}{⫋}");
defineMacro("\\varsupsetneq", "\\html@mathml{\\@varsupsetneq}{⊋}");
defineMacro("\\varsupsetneqq", "\\html@mathml{\\@varsupsetneqq}{⫌}"); //////////////////////////////////////////////////////////////////////
// stmaryrd and semantic
// The stmaryrd and semantic packages render the next four items by calling a
// glyph. Those glyphs do not exist in the KaTeX fonts. Hence the macros.

defineMacro("\\llbracket", "\\html@mathml{" + "\\mathopen{[\\mkern-3.2mu[}}" + "{\\mathopen{\\char`\u27E6}}");
defineMacro("\\rrbracket", "\\html@mathml{" + "\\mathclose{]\\mkern-3.2mu]}}" + "{\\mathclose{\\char`\u27E7}}");
defineMacro("\u27E6", "\\llbracket"); // blackboard bold [

defineMacro("\u27E7", "\\rrbracket"); // blackboard bold ]

defineMacro("\\lBrace", "\\html@mathml{" + "\\mathopen{\\{\\mkern-3.2mu[}}" + "{\\mathopen{\\char`\u2983}}");
defineMacro("\\rBrace", "\\html@mathml{" + "\\mathclose{]\\mkern-3.2mu\\}}}" + "{\\mathclose{\\char`\u2984}}");
defineMacro("\u2983", "\\lBrace"); // blackboard bold {

defineMacro("\u2984", "\\rBrace"); // blackboard bold }
// TODO: Create variable sized versions of the last two items. I believe that
// will require new font glyphs.
//////////////////////////////////////////////////////////////////////
// texvc.sty
// The texvc package contains macros available in mediawiki pages.
// We omit the functions deprecated at
// https://en.wikipedia.org/wiki/Help:Displaying_a_formula#Deprecated_syntax
// We also omit texvc's \O, which conflicts with \text{\O}

defineMacro("\\darr", "\\downarrow");
defineMacro("\\dArr", "\\Downarrow");
defineMacro("\\Darr", "\\Downarrow");
defineMacro("\\lang", "\\langle");
defineMacro("\\rang", "\\rangle");
defineMacro("\\uarr", "\\uparrow");
defineMacro("\\uArr", "\\Uparrow");
defineMacro("\\Uarr", "\\Uparrow");
defineMacro("\\N", "\\mathbb{N}");
defineMacro("\\R", "\\mathbb{R}");
defineMacro("\\Z", "\\mathbb{Z}");
defineMacro("\\alef", "\\aleph");
defineMacro("\\alefsym", "\\aleph");
defineMacro("\\Alpha", "\\mathrm{A}");
defineMacro("\\Beta", "\\mathrm{B}");
defineMacro("\\bull", "\\bullet");
defineMacro("\\Chi", "\\mathrm{X}");
defineMacro("\\clubs", "\\clubsuit");
defineMacro("\\cnums", "\\mathbb{C}");
defineMacro("\\Complex", "\\mathbb{C}");
defineMacro("\\Dagger", "\\ddagger");
defineMacro("\\diamonds", "\\diamondsuit");
defineMacro("\\empty", "\\emptyset");
defineMacro("\\Epsilon", "\\mathrm{E}");
defineMacro("\\Eta", "\\mathrm{H}");
defineMacro("\\exist", "\\exists");
defineMacro("\\harr", "\\leftrightarrow");
defineMacro("\\hArr", "\\Leftrightarrow");
defineMacro("\\Harr", "\\Leftrightarrow");
defineMacro("\\hearts", "\\heartsuit");
defineMacro("\\image", "\\Im");
defineMacro("\\infin", "\\infty");
defineMacro("\\Iota", "\\mathrm{I}");
defineMacro("\\isin", "\\in");
defineMacro("\\Kappa", "\\mathrm{K}");
defineMacro("\\larr", "\\leftarrow");
defineMacro("\\lArr", "\\Leftarrow");
defineMacro("\\Larr", "\\Leftarrow");
defineMacro("\\lrarr", "\\leftrightarrow");
defineMacro("\\lrArr", "\\Leftrightarrow");
defineMacro("\\Lrarr", "\\Leftrightarrow");
defineMacro("\\Mu", "\\mathrm{M}");
defineMacro("\\natnums", "\\mathbb{N}");
defineMacro("\\Nu", "\\mathrm{N}");
defineMacro("\\Omicron", "\\mathrm{O}");
defineMacro("\\plusmn", "\\pm");
defineMacro("\\rarr", "\\rightarrow");
defineMacro("\\rArr", "\\Rightarrow");
defineMacro("\\Rarr", "\\Rightarrow");
defineMacro("\\real", "\\Re");
defineMacro("\\reals", "\\mathbb{R}");
defineMacro("\\Reals", "\\mathbb{R}");
defineMacro("\\Rho", "\\mathrm{P}");
defineMacro("\\sdot", "\\cdot");
defineMacro("\\sect", "\\S");
defineMacro("\\spades", "\\spadesuit");
defineMacro("\\sub", "\\subset");
defineMacro("\\sube", "\\subseteq");
defineMacro("\\supe", "\\supseteq");
defineMacro("\\Tau", "\\mathrm{T}");
defineMacro("\\thetasym", "\\vartheta"); // TODO: defineMacro("\\varcoppa", "\\\mbox{\\coppa}");

defineMacro("\\weierp", "\\wp");
defineMacro("\\Zeta", "\\mathrm{Z}"); //////////////////////////////////////////////////////////////////////
// statmath.sty
// https://ctan.math.illinois.edu/macros/latex/contrib/statmath/statmath.pdf

defineMacro("\\argmin", "\\DOTSB\\mathop{\\operatorname{arg\\,min}}\\limits");
defineMacro("\\argmax", "\\DOTSB\\mathop{\\operatorname{arg\\,max}}\\limits"); // Custom Khan Academy colors, should be moved to an optional package

defineMacro("\\blue", "\\textcolor{##6495ed}{#1}");
defineMacro("\\orange", "\\textcolor{##ffa500}{#1}");
defineMacro("\\pink", "\\textcolor{##ff00af}{#1}");
defineMacro("\\red", "\\textcolor{##df0030}{#1}");
defineMacro("\\green", "\\textcolor{##28ae7b}{#1}");
defineMacro("\\gray", "\\textcolor{gray}{##1}");
defineMacro("\\purple", "\\textcolor{##9d38bd}{#1}");
defineMacro("\\blueA", "\\textcolor{##ccfaff}{#1}");
defineMacro("\\blueB", "\\textcolor{##80f6ff}{#1}");
defineMacro("\\blueC", "\\textcolor{##63d9ea}{#1}");
defineMacro("\\blueD", "\\textcolor{##11accd}{#1}");
defineMacro("\\blueE", "\\textcolor{##0c7f99}{#1}");
defineMacro("\\tealA", "\\textcolor{##94fff5}{#1}");
defineMacro("\\tealB", "\\textcolor{##26edd5}{#1}");
defineMacro("\\tealC", "\\textcolor{##01d1c1}{#1}");
defineMacro("\\tealD", "\\textcolor{##01a995}{#1}");
defineMacro("\\tealE", "\\textcolor{##208170}{#1}");
defineMacro("\\greenA", "\\textcolor{##b6ffb0}{#1}");
defineMacro("\\greenB", "\\textcolor{##8af281}{#1}");
defineMacro("\\greenC", "\\textcolor{##74cf70}{#1}");
defineMacro("\\greenD", "\\textcolor{##1fab54}{#1}");
defineMacro("\\greenE", "\\textcolor{##0d923f}{#1}");
defineMacro("\\goldA", "\\textcolor{##ffd0a9}{#1}");
defineMacro("\\goldB", "\\textcolor{##ffbb71}{#1}");
defineMacro("\\goldC", "\\textcolor{##ff9c39}{#1}");
defineMacro("\\goldD", "\\textcolor{##e07d10}{#1}");
defineMacro("\\goldE", "\\textcolor{##a75a05}{#1}");
defineMacro("\\redA", "\\textcolor{##fca9a9}{#1}");
defineMacro("\\redB", "\\textcolor{##ff8482}{#1}");
defineMacro("\\redC", "\\textcolor{##f9685d}{#1}");
defineMacro("\\redD", "\\textcolor{##e84d39}{#1}");
defineMacro("\\redE", "\\textcolor{##bc2612}{#1}");
defineMacro("\\maroonA", "\\textcolor{##ffbde0}{#1}");
defineMacro("\\maroonB", "\\textcolor{##ff92c6}{#1}");
defineMacro("\\maroonC", "\\textcolor{##ed5fa6}{#1}");
defineMacro("\\maroonD", "\\textcolor{##ca337c}{#1}");
defineMacro("\\maroonE", "\\textcolor{##9e034e}{#1}");
defineMacro("\\purpleA", "\\textcolor{##ddd7ff}{#1}");
defineMacro("\\purpleB", "\\textcolor{##c6b9fc}{#1}");
defineMacro("\\purpleC", "\\textcolor{##aa87ff}{#1}");
defineMacro("\\purpleD", "\\textcolor{##7854ab}{#1}");
defineMacro("\\purpleE", "\\textcolor{##543b78}{#1}");
defineMacro("\\mintA", "\\textcolor{##f5f9e8}{#1}");
defineMacro("\\mintB", "\\textcolor{##edf2df}{#1}");
defineMacro("\\mintC", "\\textcolor{##e0e5cc}{#1}");
defineMacro("\\grayA", "\\textcolor{##f6f7f7}{#1}");
defineMacro("\\grayB", "\\textcolor{##f0f1f2}{#1}");
defineMacro("\\grayC", "\\textcolor{##e3e5e6}{#1}");
defineMacro("\\grayD", "\\textcolor{##d6d8da}{#1}");
defineMacro("\\grayE", "\\textcolor{##babec2}{#1}");
defineMacro("\\grayF", "\\textcolor{##888d93}{#1}");
defineMacro("\\grayG", "\\textcolor{##626569}{#1}");
defineMacro("\\grayH", "\\textcolor{##3b3e40}{#1}");
defineMacro("\\grayI", "\\textcolor{##21242c}{#1}");
defineMacro("\\kaBlue", "\\textcolor{##314453}{#1}");
defineMacro("\\kaGreen", "\\textcolor{##71B307}{#1}");
// CONCATENATED MODULE: ./src/MacroExpander.js
/**
 * This file contains the “gullet” where macros are expanded
 * until only non-macro tokens remain.
 */







// List of commands that act like macros but aren't defined as a macro,
// function, or symbol.  Used in `isDefined`.
var implicitCommands = {
  "\\relax": true,
  // MacroExpander.js
  "^": true,
  // Parser.js
  "_": true,
  // Parser.js
  "\\limits": true,
  // Parser.js
  "\\nolimits": true // Parser.js

};

var MacroExpander_MacroExpander =
/*#__PURE__*/
function () {
  function MacroExpander(input, settings, mode) {
    this.settings = void 0;
    this.expansionCount = void 0;
    this.lexer = void 0;
    this.macros = void 0;
    this.stack = void 0;
    this.mode = void 0;
    this.settings = settings;
    this.expansionCount = 0;
    this.feed(input); // Make new global namespace

    this.macros = new Namespace_Namespace(macros, settings.macros);
    this.mode = mode;
    this.stack = []; // contains tokens in REVERSE order
  }
  /**
   * Feed a new input string to the same MacroExpander
   * (with existing macros etc.).
   */


  var _proto = MacroExpander.prototype;

  _proto.feed = function feed(input) {
    this.lexer = new Lexer_Lexer(input, this.settings);
  }
  /**
   * Switches between "text" and "math" modes.
   */
  ;

  _proto.switchMode = function switchMode(newMode) {
    this.mode = newMode;
  }
  /**
   * Start a new group nesting within all namespaces.
   */
  ;

  _proto.beginGroup = function beginGroup() {
    this.macros.beginGroup();
  }
  /**
   * End current group nesting within all namespaces.
   */
  ;

  _proto.endGroup = function endGroup() {
    this.macros.endGroup();
  }
  /**
   * Returns the topmost token on the stack, without expanding it.
   * Similar in behavior to TeX's `\futurelet`.
   */
  ;

  _proto.future = function future() {
    if (this.stack.length === 0) {
      this.pushToken(this.lexer.lex());
    }

    return this.stack[this.stack.length - 1];
  }
  /**
   * Remove and return the next unexpanded token.
   */
  ;

  _proto.popToken = function popToken() {
    this.future(); // ensure non-empty stack

    return this.stack.pop();
  }
  /**
   * Add a given token to the token stack.  In particular, this get be used
   * to put back a token returned from one of the other methods.
   */
  ;

  _proto.pushToken = function pushToken(token) {
    this.stack.push(token);
  }
  /**
   * Append an array of tokens to the token stack.
   */
  ;

  _proto.pushTokens = function pushTokens(tokens) {
    var _this$stack;

    (_this$stack = this.stack).push.apply(_this$stack, tokens);
  }
  /**
   * Consume all following space tokens, without expansion.
   */
  ;

  _proto.consumeSpaces = function consumeSpaces() {
    for (;;) {
      var token = this.future();

      if (token.text === " ") {
        this.stack.pop();
      } else {
        break;
      }
    }
  }
  /**
   * Consume the specified number of arguments from the token stream,
   * and return the resulting array of arguments.
   */
  ;

  _proto.consumeArgs = function consumeArgs(numArgs) {
    var args = []; // obtain arguments, either single token or balanced {…} group

    for (var i = 0; i < numArgs; ++i) {
      this.consumeSpaces(); // ignore spaces before each argument

      var startOfArg = this.popToken();

      if (startOfArg.text === "{") {
        var arg = [];
        var depth = 1;

        while (depth !== 0) {
          var tok = this.popToken();
          arg.push(tok);

          if (tok.text === "{") {
            ++depth;
          } else if (tok.text === "}") {
            --depth;
          } else if (tok.text === "EOF") {
            throw new src_ParseError("End of input in macro argument", startOfArg);
          }
        }

        arg.pop(); // remove last }

        arg.reverse(); // like above, to fit in with stack order

        args[i] = arg;
      } else if (startOfArg.text === "EOF") {
        throw new src_ParseError("End of input expecting macro argument");
      } else {
        args[i] = [startOfArg];
      }
    }

    return args;
  }
  /**
   * Expand the next token only once if possible.
   *
   * If the token is expanded, the resulting tokens will be pushed onto
   * the stack in reverse order and will be returned as an array,
   * also in reverse order.
   *
   * If not, the next token will be returned without removing it
   * from the stack.  This case can be detected by a `Token` return value
   * instead of an `Array` return value.
   *
   * In either case, the next token will be on the top of the stack,
   * or the stack will be empty.
   *
   * Used to implement `expandAfterFuture` and `expandNextToken`.
   *
   * At the moment, macro expansion doesn't handle delimited macros,
   * i.e. things like those defined by \def\foo#1\end{…}.
   * See the TeX book page 202ff. for details on how those should behave.
   */
  ;

  _proto.expandOnce = function expandOnce() {
    var topToken = this.popToken();
    var name = topToken.text;

    var expansion = this._getExpansion(name);

    if (expansion == null) {
      // mainly checking for undefined here
      // Fully expanded
      this.pushToken(topToken);
      return topToken;
    }

    this.expansionCount++;

    if (this.expansionCount > this.settings.maxExpand) {
      throw new src_ParseError("Too many expansions: infinite loop or " + "need to increase maxExpand setting");
    }

    var tokens = expansion.tokens;

    if (expansion.numArgs) {
      var args = this.consumeArgs(expansion.numArgs); // paste arguments in place of the placeholders

      tokens = tokens.slice(); // make a shallow copy

      for (var i = tokens.length - 1; i >= 0; --i) {
        var tok = tokens[i];

        if (tok.text === "#") {
          if (i === 0) {
            throw new src_ParseError("Incomplete placeholder at end of macro body", tok);
          }

          tok = tokens[--i]; // next token on stack

          if (tok.text === "#") {
            // ## → #
            tokens.splice(i + 1, 1); // drop first #
          } else if (/^[1-9]$/.test(tok.text)) {
            var _tokens;

            // replace the placeholder with the indicated argument
            (_tokens = tokens).splice.apply(_tokens, [i, 2].concat(args[+tok.text - 1]));
          } else {
            throw new src_ParseError("Not a valid argument number", tok);
          }
        }
      }
    } // Concatenate expansion onto top of stack.


    this.pushTokens(tokens);
    return tokens;
  }
  /**
   * Expand the next token only once (if possible), and return the resulting
   * top token on the stack (without removing anything from the stack).
   * Similar in behavior to TeX's `\expandafter\futurelet`.
   * Equivalent to expandOnce() followed by future().
   */
  ;

  _proto.expandAfterFuture = function expandAfterFuture() {
    this.expandOnce();
    return this.future();
  }
  /**
   * Recursively expand first token, then return first non-expandable token.
   */
  ;

  _proto.expandNextToken = function expandNextToken() {
    for (;;) {
      var expanded = this.expandOnce(); // expandOnce returns Token if and only if it's fully expanded.

      if (expanded instanceof Token_Token) {
        // \relax stops the expansion, but shouldn't get returned (a
        // null return value couldn't get implemented as a function).
        if (expanded.text === "\\relax") {
          this.stack.pop();
        } else {
          return this.stack.pop(); // === expanded
        }
      }
    } // Flow unable to figure out that this pathway is impossible.
    // https://github.com/facebook/flow/issues/4808


    throw new Error(); // eslint-disable-line no-unreachable
  }
  /**
   * Fully expand the given macro name and return the resulting list of
   * tokens, or return `undefined` if no such macro is defined.
   */
  ;

  _proto.expandMacro = function expandMacro(name) {
    if (!this.macros.get(name)) {
      return undefined;
    }

    var output = [];
    var oldStackLength = this.stack.length;
    this.pushToken(new Token_Token(name));

    while (this.stack.length > oldStackLength) {
      var expanded = this.expandOnce(); // expandOnce returns Token if and only if it's fully expanded.

      if (expanded instanceof Token_Token) {
        output.push(this.stack.pop());
      }
    }

    return output;
  }
  /**
   * Fully expand the given macro name and return the result as a string,
   * or return `undefined` if no such macro is defined.
   */
  ;

  _proto.expandMacroAsText = function expandMacroAsText(name) {
    var tokens = this.expandMacro(name);

    if (tokens) {
      return tokens.map(function (token) {
        return token.text;
      }).join("");
    } else {
      return tokens;
    }
  }
  /**
   * Returns the expanded macro as a reversed array of tokens and a macro
   * argument count.  Or returns `null` if no such macro.
   */
  ;

  _proto._getExpansion = function _getExpansion(name) {
    var definition = this.macros.get(name);

    if (definition == null) {
      // mainly checking for undefined here
      return definition;
    }

    var expansion = typeof definition === "function" ? definition(this) : definition;

    if (typeof expansion === "string") {
      var numArgs = 0;

      if (expansion.indexOf("#") !== -1) {
        var stripped = expansion.replace(/##/g, "");

        while (stripped.indexOf("#" + (numArgs + 1)) !== -1) {
          ++numArgs;
        }
      }

      var bodyLexer = new Lexer_Lexer(expansion, this.settings);
      var tokens = [];
      var tok = bodyLexer.lex();

      while (tok.text !== "EOF") {
        tokens.push(tok);
        tok = bodyLexer.lex();
      }

      tokens.reverse(); // to fit in with stack using push and pop

      var expanded = {
        tokens: tokens,
        numArgs: numArgs
      };
      return expanded;
    }

    return expansion;
  }
  /**
   * Determine whether a command is currently "defined" (has some
   * functionality), meaning that it's a macro (in the current group),
   * a function, a symbol, or one of the special commands listed in
   * `implicitCommands`.
   */
  ;

  _proto.isDefined = function isDefined(name) {
    return this.macros.has(name) || src_functions.hasOwnProperty(name) || src_symbols.math.hasOwnProperty(name) || src_symbols.text.hasOwnProperty(name) || implicitCommands.hasOwnProperty(name);
  };

  return MacroExpander;
}();


// CONCATENATED MODULE: ./src/unicodeAccents.js
// Mapping of Unicode accent characters to their LaTeX equivalent in text and
// math mode (when they exist).
/* harmony default export */ var unicodeAccents = ({
  "\u0301": {
    text: "\\'",
    math: '\\acute'
  },
  "\u0300": {
    text: '\\`',
    math: '\\grave'
  },
  "\u0308": {
    text: '\\"',
    math: '\\ddot'
  },
  "\u0303": {
    text: '\\~',
    math: '\\tilde'
  },
  "\u0304": {
    text: '\\=',
    math: '\\bar'
  },
  "\u0306": {
    text: "\\u",
    math: '\\breve'
  },
  "\u030C": {
    text: '\\v',
    math: '\\check'
  },
  "\u0302": {
    text: '\\^',
    math: '\\hat'
  },
  "\u0307": {
    text: '\\.',
    math: '\\dot'
  },
  "\u030A": {
    text: '\\r',
    math: '\\mathring'
  },
  "\u030B": {
    text: '\\H'
  }
});
// CONCATENATED MODULE: ./src/unicodeSymbols.js
// This file is GENERATED by unicodeMake.js. DO NOT MODIFY.
/* harmony default export */ var unicodeSymbols = ({
  "\xE1": "a\u0301",
  // á = \'{a}
  "\xE0": "a\u0300",
  // à = \`{a}
  "\xE4": "a\u0308",
  // ä = \"{a}
  "\u01DF": "a\u0308\u0304",
  // ǟ = \"\={a}
  "\xE3": "a\u0303",
  // ã = \~{a}
  "\u0101": "a\u0304",
  // ā = \={a}
  "\u0103": "a\u0306",
  // ă = \u{a}
  "\u1EAF": "a\u0306\u0301",
  // ắ = \u\'{a}
  "\u1EB1": "a\u0306\u0300",
  // ằ = \u\`{a}
  "\u1EB5": "a\u0306\u0303",
  // ẵ = \u\~{a}
  "\u01CE": "a\u030C",
  // ǎ = \v{a}
  "\xE2": "a\u0302",
  // â = \^{a}
  "\u1EA5": "a\u0302\u0301",
  // ấ = \^\'{a}
  "\u1EA7": "a\u0302\u0300",
  // ầ = \^\`{a}
  "\u1EAB": "a\u0302\u0303",
  // ẫ = \^\~{a}
  "\u0227": "a\u0307",
  // ȧ = \.{a}
  "\u01E1": "a\u0307\u0304",
  // ǡ = \.\={a}
  "\xE5": "a\u030A",
  // å = \r{a}
  "\u01FB": "a\u030A\u0301",
  // ǻ = \r\'{a}
  "\u1E03": "b\u0307",
  // ḃ = \.{b}
  "\u0107": "c\u0301",
  // ć = \'{c}
  "\u010D": "c\u030C",
  // č = \v{c}
  "\u0109": "c\u0302",
  // ĉ = \^{c}
  "\u010B": "c\u0307",
  // ċ = \.{c}
  "\u010F": "d\u030C",
  // ď = \v{d}
  "\u1E0B": "d\u0307",
  // ḋ = \.{d}
  "\xE9": "e\u0301",
  // é = \'{e}
  "\xE8": "e\u0300",
  // è = \`{e}
  "\xEB": "e\u0308",
  // ë = \"{e}
  "\u1EBD": "e\u0303",
  // ẽ = \~{e}
  "\u0113": "e\u0304",
  // ē = \={e}
  "\u1E17": "e\u0304\u0301",
  // ḗ = \=\'{e}
  "\u1E15": "e\u0304\u0300",
  // ḕ = \=\`{e}
  "\u0115": "e\u0306",
  // ĕ = \u{e}
  "\u011B": "e\u030C",
  // ě = \v{e}
  "\xEA": "e\u0302",
  // ê = \^{e}
  "\u1EBF": "e\u0302\u0301",
  // ế = \^\'{e}
  "\u1EC1": "e\u0302\u0300",
  // ề = \^\`{e}
  "\u1EC5": "e\u0302\u0303",
  // ễ = \^\~{e}
  "\u0117": "e\u0307",
  // ė = \.{e}
  "\u1E1F": "f\u0307",
  // ḟ = \.{f}
  "\u01F5": "g\u0301",
  // ǵ = \'{g}
  "\u1E21": "g\u0304",
  // ḡ = \={g}
  "\u011F": "g\u0306",
  // ğ = \u{g}
  "\u01E7": "g\u030C",
  // ǧ = \v{g}
  "\u011D": "g\u0302",
  // ĝ = \^{g}
  "\u0121": "g\u0307",
  // ġ = \.{g}
  "\u1E27": "h\u0308",
  // ḧ = \"{h}
  "\u021F": "h\u030C",
  // ȟ = \v{h}
  "\u0125": "h\u0302",
  // ĥ = \^{h}
  "\u1E23": "h\u0307",
  // ḣ = \.{h}
  "\xED": "i\u0301",
  // í = \'{i}
  "\xEC": "i\u0300",
  // ì = \`{i}
  "\xEF": "i\u0308",
  // ï = \"{i}
  "\u1E2F": "i\u0308\u0301",
  // ḯ = \"\'{i}
  "\u0129": "i\u0303",
  // ĩ = \~{i}
  "\u012B": "i\u0304",
  // ī = \={i}
  "\u012D": "i\u0306",
  // ĭ = \u{i}
  "\u01D0": "i\u030C",
  // ǐ = \v{i}
  "\xEE": "i\u0302",
  // î = \^{i}
  "\u01F0": "j\u030C",
  // ǰ = \v{j}
  "\u0135": "j\u0302",
  // ĵ = \^{j}
  "\u1E31": "k\u0301",
  // ḱ = \'{k}
  "\u01E9": "k\u030C",
  // ǩ = \v{k}
  "\u013A": "l\u0301",
  // ĺ = \'{l}
  "\u013E": "l\u030C",
  // ľ = \v{l}
  "\u1E3F": "m\u0301",
  // ḿ = \'{m}
  "\u1E41": "m\u0307",
  // ṁ = \.{m}
  "\u0144": "n\u0301",
  // ń = \'{n}
  "\u01F9": "n\u0300",
  // ǹ = \`{n}
  "\xF1": "n\u0303",
  // ñ = \~{n}
  "\u0148": "n\u030C",
  // ň = \v{n}
  "\u1E45": "n\u0307",
  // ṅ = \.{n}
  "\xF3": "o\u0301",
  // ó = \'{o}
  "\xF2": "o\u0300",
  // ò = \`{o}
  "\xF6": "o\u0308",
  // ö = \"{o}
  "\u022B": "o\u0308\u0304",
  // ȫ = \"\={o}
  "\xF5": "o\u0303",
  // õ = \~{o}
  "\u1E4D": "o\u0303\u0301",
  // ṍ = \~\'{o}
  "\u1E4F": "o\u0303\u0308",
  // ṏ = \~\"{o}
  "\u022D": "o\u0303\u0304",
  // ȭ = \~\={o}
  "\u014D": "o\u0304",
  // ō = \={o}
  "\u1E53": "o\u0304\u0301",
  // ṓ = \=\'{o}
  "\u1E51": "o\u0304\u0300",
  // ṑ = \=\`{o}
  "\u014F": "o\u0306",
  // ŏ = \u{o}
  "\u01D2": "o\u030C",
  // ǒ = \v{o}
  "\xF4": "o\u0302",
  // ô = \^{o}
  "\u1ED1": "o\u0302\u0301",
  // ố = \^\'{o}
  "\u1ED3": "o\u0302\u0300",
  // ồ = \^\`{o}
  "\u1ED7": "o\u0302\u0303",
  // ỗ = \^\~{o}
  "\u022F": "o\u0307",
  // ȯ = \.{o}
  "\u0231": "o\u0307\u0304",
  // ȱ = \.\={o}
  "\u0151": "o\u030B",
  // ő = \H{o}
  "\u1E55": "p\u0301",
  // ṕ = \'{p}
  "\u1E57": "p\u0307",
  // ṗ = \.{p}
  "\u0155": "r\u0301",
  // ŕ = \'{r}
  "\u0159": "r\u030C",
  // ř = \v{r}
  "\u1E59": "r\u0307",
  // ṙ = \.{r}
  "\u015B": "s\u0301",
  // ś = \'{s}
  "\u1E65": "s\u0301\u0307",
  // ṥ = \'\.{s}
  "\u0161": "s\u030C",
  // š = \v{s}
  "\u1E67": "s\u030C\u0307",
  // ṧ = \v\.{s}
  "\u015D": "s\u0302",
  // ŝ = \^{s}
  "\u1E61": "s\u0307",
  // ṡ = \.{s}
  "\u1E97": "t\u0308",
  // ẗ = \"{t}
  "\u0165": "t\u030C",
  // ť = \v{t}
  "\u1E6B": "t\u0307",
  // ṫ = \.{t}
  "\xFA": "u\u0301",
  // ú = \'{u}
  "\xF9": "u\u0300",
  // ù = \`{u}
  "\xFC": "u\u0308",
  // ü = \"{u}
  "\u01D8": "u\u0308\u0301",
  // ǘ = \"\'{u}
  "\u01DC": "u\u0308\u0300",
  // ǜ = \"\`{u}
  "\u01D6": "u\u0308\u0304",
  // ǖ = \"\={u}
  "\u01DA": "u\u0308\u030C",
  // ǚ = \"\v{u}
  "\u0169": "u\u0303",
  // ũ = \~{u}
  "\u1E79": "u\u0303\u0301",
  // ṹ = \~\'{u}
  "\u016B": "u\u0304",
  // ū = \={u}
  "\u1E7B": "u\u0304\u0308",
  // ṻ = \=\"{u}
  "\u016D": "u\u0306",
  // ŭ = \u{u}
  "\u01D4": "u\u030C",
  // ǔ = \v{u}
  "\xFB": "u\u0302",
  // û = \^{u}
  "\u016F": "u\u030A",
  // ů = \r{u}
  "\u0171": "u\u030B",
  // ű = \H{u}
  "\u1E7D": "v\u0303",
  // ṽ = \~{v}
  "\u1E83": "w\u0301",
  // ẃ = \'{w}
  "\u1E81": "w\u0300",
  // ẁ = \`{w}
  "\u1E85": "w\u0308",
  // ẅ = \"{w}
  "\u0175": "w\u0302",
  // ŵ = \^{w}
  "\u1E87": "w\u0307",
  // ẇ = \.{w}
  "\u1E98": "w\u030A",
  // ẘ = \r{w}
  "\u1E8D": "x\u0308",
  // ẍ = \"{x}
  "\u1E8B": "x\u0307",
  // ẋ = \.{x}
  "\xFD": "y\u0301",
  // ý = \'{y}
  "\u1EF3": "y\u0300",
  // ỳ = \`{y}
  "\xFF": "y\u0308",
  // ÿ = \"{y}
  "\u1EF9": "y\u0303",
  // ỹ = \~{y}
  "\u0233": "y\u0304",
  // ȳ = \={y}
  "\u0177": "y\u0302",
  // ŷ = \^{y}
  "\u1E8F": "y\u0307",
  // ẏ = \.{y}
  "\u1E99": "y\u030A",
  // ẙ = \r{y}
  "\u017A": "z\u0301",
  // ź = \'{z}
  "\u017E": "z\u030C",
  // ž = \v{z}
  "\u1E91": "z\u0302",
  // ẑ = \^{z}
  "\u017C": "z\u0307",
  // ż = \.{z}
  "\xC1": "A\u0301",
  // Á = \'{A}
  "\xC0": "A\u0300",
  // À = \`{A}
  "\xC4": "A\u0308",
  // Ä = \"{A}
  "\u01DE": "A\u0308\u0304",
  // Ǟ = \"\={A}
  "\xC3": "A\u0303",
  // Ã = \~{A}
  "\u0100": "A\u0304",
  // Ā = \={A}
  "\u0102": "A\u0306",
  // Ă = \u{A}
  "\u1EAE": "A\u0306\u0301",
  // Ắ = \u\'{A}
  "\u1EB0": "A\u0306\u0300",
  // Ằ = \u\`{A}
  "\u1EB4": "A\u0306\u0303",
  // Ẵ = \u\~{A}
  "\u01CD": "A\u030C",
  // Ǎ = \v{A}
  "\xC2": "A\u0302",
  // Â = \^{A}
  "\u1EA4": "A\u0302\u0301",
  // Ấ = \^\'{A}
  "\u1EA6": "A\u0302\u0300",
  // Ầ = \^\`{A}
  "\u1EAA": "A\u0302\u0303",
  // Ẫ = \^\~{A}
  "\u0226": "A\u0307",
  // Ȧ = \.{A}
  "\u01E0": "A\u0307\u0304",
  // Ǡ = \.\={A}
  "\xC5": "A\u030A",
  // Å = \r{A}
  "\u01FA": "A\u030A\u0301",
  // Ǻ = \r\'{A}
  "\u1E02": "B\u0307",
  // Ḃ = \.{B}
  "\u0106": "C\u0301",
  // Ć = \'{C}
  "\u010C": "C\u030C",
  // Č = \v{C}
  "\u0108": "C\u0302",
  // Ĉ = \^{C}
  "\u010A": "C\u0307",
  // Ċ = \.{C}
  "\u010E": "D\u030C",
  // Ď = \v{D}
  "\u1E0A": "D\u0307",
  // Ḋ = \.{D}
  "\xC9": "E\u0301",
  // É = \'{E}
  "\xC8": "E\u0300",
  // È = \`{E}
  "\xCB": "E\u0308",
  // Ë = \"{E}
  "\u1EBC": "E\u0303",
  // Ẽ = \~{E}
  "\u0112": "E\u0304",
  // Ē = \={E}
  "\u1E16": "E\u0304\u0301",
  // Ḗ = \=\'{E}
  "\u1E14": "E\u0304\u0300",
  // Ḕ = \=\`{E}
  "\u0114": "E\u0306",
  // Ĕ = \u{E}
  "\u011A": "E\u030C",
  // Ě = \v{E}
  "\xCA": "E\u0302",
  // Ê = \^{E}
  "\u1EBE": "E\u0302\u0301",
  // Ế = \^\'{E}
  "\u1EC0": "E\u0302\u0300",
  // Ề = \^\`{E}
  "\u1EC4": "E\u0302\u0303",
  // Ễ = \^\~{E}
  "\u0116": "E\u0307",
  // Ė = \.{E}
  "\u1E1E": "F\u0307",
  // Ḟ = \.{F}
  "\u01F4": "G\u0301",
  // Ǵ = \'{G}
  "\u1E20": "G\u0304",
  // Ḡ = \={G}
  "\u011E": "G\u0306",
  // Ğ = \u{G}
  "\u01E6": "G\u030C",
  // Ǧ = \v{G}
  "\u011C": "G\u0302",
  // Ĝ = \^{G}
  "\u0120": "G\u0307",
  // Ġ = \.{G}
  "\u1E26": "H\u0308",
  // Ḧ = \"{H}
  "\u021E": "H\u030C",
  // Ȟ = \v{H}
  "\u0124": "H\u0302",
  // Ĥ = \^{H}
  "\u1E22": "H\u0307",
  // Ḣ = \.{H}
  "\xCD": "I\u0301",
  // Í = \'{I}
  "\xCC": "I\u0300",
  // Ì = \`{I}
  "\xCF": "I\u0308",
  // Ï = \"{I}
  "\u1E2E": "I\u0308\u0301",
  // Ḯ = \"\'{I}
  "\u0128": "I\u0303",
  // Ĩ = \~{I}
  "\u012A": "I\u0304",
  // Ī = \={I}
  "\u012C": "I\u0306",
  // Ĭ = \u{I}
  "\u01CF": "I\u030C",
  // Ǐ = \v{I}
  "\xCE": "I\u0302",
  // Î = \^{I}
  "\u0130": "I\u0307",
  // İ = \.{I}
  "\u0134": "J\u0302",
  // Ĵ = \^{J}
  "\u1E30": "K\u0301",
  // Ḱ = \'{K}
  "\u01E8": "K\u030C",
  // Ǩ = \v{K}
  "\u0139": "L\u0301",
  // Ĺ = \'{L}
  "\u013D": "L\u030C",
  // Ľ = \v{L}
  "\u1E3E": "M\u0301",
  // Ḿ = \'{M}
  "\u1E40": "M\u0307",
  // Ṁ = \.{M}
  "\u0143": "N\u0301",
  // Ń = \'{N}
  "\u01F8": "N\u0300",
  // Ǹ = \`{N}
  "\xD1": "N\u0303",
  // Ñ = \~{N}
  "\u0147": "N\u030C",
  // Ň = \v{N}
  "\u1E44": "N\u0307",
  // Ṅ = \.{N}
  "\xD3": "O\u0301",
  // Ó = \'{O}
  "\xD2": "O\u0300",
  // Ò = \`{O}
  "\xD6": "O\u0308",
  // Ö = \"{O}
  "\u022A": "O\u0308\u0304",
  // Ȫ = \"\={O}
  "\xD5": "O\u0303",
  // Õ = \~{O}
  "\u1E4C": "O\u0303\u0301",
  // Ṍ = \~\'{O}
  "\u1E4E": "O\u0303\u0308",
  // Ṏ = \~\"{O}
  "\u022C": "O\u0303\u0304",
  // Ȭ = \~\={O}
  "\u014C": "O\u0304",
  // Ō = \={O}
  "\u1E52": "O\u0304\u0301",
  // Ṓ = \=\'{O}
  "\u1E50": "O\u0304\u0300",
  // Ṑ = \=\`{O}
  "\u014E": "O\u0306",
  // Ŏ = \u{O}
  "\u01D1": "O\u030C",
  // Ǒ = \v{O}
  "\xD4": "O\u0302",
  // Ô = \^{O}
  "\u1ED0": "O\u0302\u0301",
  // Ố = \^\'{O}
  "\u1ED2": "O\u0302\u0300",
  // Ồ = \^\`{O}
  "\u1ED6": "O\u0302\u0303",
  // Ỗ = \^\~{O}
  "\u022E": "O\u0307",
  // Ȯ = \.{O}
  "\u0230": "O\u0307\u0304",
  // Ȱ = \.\={O}
  "\u0150": "O\u030B",
  // Ő = \H{O}
  "\u1E54": "P\u0301",
  // Ṕ = \'{P}
  "\u1E56": "P\u0307",
  // Ṗ = \.{P}
  "\u0154": "R\u0301",
  // Ŕ = \'{R}
  "\u0158": "R\u030C",
  // Ř = \v{R}
  "\u1E58": "R\u0307",
  // Ṙ = \.{R}
  "\u015A": "S\u0301",
  // Ś = \'{S}
  "\u1E64": "S\u0301\u0307",
  // Ṥ = \'\.{S}
  "\u0160": "S\u030C",
  // Š = \v{S}
  "\u1E66": "S\u030C\u0307",
  // Ṧ = \v\.{S}
  "\u015C": "S\u0302",
  // Ŝ = \^{S}
  "\u1E60": "S\u0307",
  // Ṡ = \.{S}
  "\u0164": "T\u030C",
  // Ť = \v{T}
  "\u1E6A": "T\u0307",
  // Ṫ = \.{T}
  "\xDA": "U\u0301",
  // Ú = \'{U}
  "\xD9": "U\u0300",
  // Ù = \`{U}
  "\xDC": "U\u0308",
  // Ü = \"{U}
  "\u01D7": "U\u0308\u0301",
  // Ǘ = \"\'{U}
  "\u01DB": "U\u0308\u0300",
  // Ǜ = \"\`{U}
  "\u01D5": "U\u0308\u0304",
  // Ǖ = \"\={U}
  "\u01D9": "U\u0308\u030C",
  // Ǚ = \"\v{U}
  "\u0168": "U\u0303",
  // Ũ = \~{U}
  "\u1E78": "U\u0303\u0301",
  // Ṹ = \~\'{U}
  "\u016A": "U\u0304",
  // Ū = \={U}
  "\u1E7A": "U\u0304\u0308",
  // Ṻ = \=\"{U}
  "\u016C": "U\u0306",
  // Ŭ = \u{U}
  "\u01D3": "U\u030C",
  // Ǔ = \v{U}
  "\xDB": "U\u0302",
  // Û = \^{U}
  "\u016E": "U\u030A",
  // Ů = \r{U}
  "\u0170": "U\u030B",
  // Ű = \H{U}
  "\u1E7C": "V\u0303",
  // Ṽ = \~{V}
  "\u1E82": "W\u0301",
  // Ẃ = \'{W}
  "\u1E80": "W\u0300",
  // Ẁ = \`{W}
  "\u1E84": "W\u0308",
  // Ẅ = \"{W}
  "\u0174": "W\u0302",
  // Ŵ = \^{W}
  "\u1E86": "W\u0307",
  // Ẇ = \.{W}
  "\u1E8C": "X\u0308",
  // Ẍ = \"{X}
  "\u1E8A": "X\u0307",
  // Ẋ = \.{X}
  "\xDD": "Y\u0301",
  // Ý = \'{Y}
  "\u1EF2": "Y\u0300",
  // Ỳ = \`{Y}
  "\u0178": "Y\u0308",
  // Ÿ = \"{Y}
  "\u1EF8": "Y\u0303",
  // Ỹ = \~{Y}
  "\u0232": "Y\u0304",
  // Ȳ = \={Y}
  "\u0176": "Y\u0302",
  // Ŷ = \^{Y}
  "\u1E8E": "Y\u0307",
  // Ẏ = \.{Y}
  "\u0179": "Z\u0301",
  // Ź = \'{Z}
  "\u017D": "Z\u030C",
  // Ž = \v{Z}
  "\u1E90": "Z\u0302",
  // Ẑ = \^{Z}
  "\u017B": "Z\u0307",
  // Ż = \.{Z}
  "\u03AC": "\u03B1\u0301",
  // ά = \'{α}
  "\u1F70": "\u03B1\u0300",
  // ὰ = \`{α}
  "\u1FB1": "\u03B1\u0304",
  // ᾱ = \={α}
  "\u1FB0": "\u03B1\u0306",
  // ᾰ = \u{α}
  "\u03AD": "\u03B5\u0301",
  // έ = \'{ε}
  "\u1F72": "\u03B5\u0300",
  // ὲ = \`{ε}
  "\u03AE": "\u03B7\u0301",
  // ή = \'{η}
  "\u1F74": "\u03B7\u0300",
  // ὴ = \`{η}
  "\u03AF": "\u03B9\u0301",
  // ί = \'{ι}
  "\u1F76": "\u03B9\u0300",
  // ὶ = \`{ι}
  "\u03CA": "\u03B9\u0308",
  // ϊ = \"{ι}
  "\u0390": "\u03B9\u0308\u0301",
  // ΐ = \"\'{ι}
  "\u1FD2": "\u03B9\u0308\u0300",
  // ῒ = \"\`{ι}
  "\u1FD1": "\u03B9\u0304",
  // ῑ = \={ι}
  "\u1FD0": "\u03B9\u0306",
  // ῐ = \u{ι}
  "\u03CC": "\u03BF\u0301",
  // ό = \'{ο}
  "\u1F78": "\u03BF\u0300",
  // ὸ = \`{ο}
  "\u03CD": "\u03C5\u0301",
  // ύ = \'{υ}
  "\u1F7A": "\u03C5\u0300",
  // ὺ = \`{υ}
  "\u03CB": "\u03C5\u0308",
  // ϋ = \"{υ}
  "\u03B0": "\u03C5\u0308\u0301",
  // ΰ = \"\'{υ}
  "\u1FE2": "\u03C5\u0308\u0300",
  // ῢ = \"\`{υ}
  "\u1FE1": "\u03C5\u0304",
  // ῡ = \={υ}
  "\u1FE0": "\u03C5\u0306",
  // ῠ = \u{υ}
  "\u03CE": "\u03C9\u0301",
  // ώ = \'{ω}
  "\u1F7C": "\u03C9\u0300",
  // ὼ = \`{ω}
  "\u038E": "\u03A5\u0301",
  // Ύ = \'{Υ}
  "\u1FEA": "\u03A5\u0300",
  // Ὺ = \`{Υ}
  "\u03AB": "\u03A5\u0308",
  // Ϋ = \"{Υ}
  "\u1FE9": "\u03A5\u0304",
  // Ῡ = \={Υ}
  "\u1FE8": "\u03A5\u0306",
  // Ῠ = \u{Υ}
  "\u038F": "\u03A9\u0301",
  // Ώ = \'{Ω}
  "\u1FFA": "\u03A9\u0300" // Ὼ = \`{Ω}

});
// CONCATENATED MODULE: ./src/Parser.js
/* eslint no-constant-condition:0 */















/**
 * This file contains the parser used to parse out a TeX expression from the
 * input. Since TeX isn't context-free, standard parsers don't work particularly
 * well.
 *
 * The strategy of this parser is as such:
 *
 * The main functions (the `.parse...` ones) take a position in the current
 * parse string to parse tokens from. The lexer (found in Lexer.js, stored at
 * this.gullet.lexer) also supports pulling out tokens at arbitrary places. When
 * individual tokens are needed at a position, the lexer is called to pull out a
 * token, which is then used.
 *
 * The parser has a property called "mode" indicating the mode that
 * the parser is currently in. Currently it has to be one of "math" or
 * "text", which denotes whether the current environment is a math-y
 * one or a text-y one (e.g. inside \text). Currently, this serves to
 * limit the functions which can be used in text mode.
 *
 * The main functions then return an object which contains the useful data that
 * was parsed at its given point, and a new position at the end of the parsed
 * data. The main functions can call each other and continue the parsing by
 * using the returned position as a new starting point.
 *
 * There are also extra `.handle...` functions, which pull out some reused
 * functionality into self-contained functions.
 *
 * The functions return ParseNodes.
 */
var Parser_Parser =
/*#__PURE__*/
function () {
  function Parser(input, settings) {
    this.mode = void 0;
    this.gullet = void 0;
    this.settings = void 0;
    this.leftrightDepth = void 0;
    this.nextToken = void 0;
    // Start in math mode
    this.mode = "math"; // Create a new macro expander (gullet) and (indirectly via that) also a
    // new lexer (mouth) for this parser (stomach, in the language of TeX)

    this.gullet = new MacroExpander_MacroExpander(input, settings, this.mode); // Store the settings for use in parsing

    this.settings = settings; // Count leftright depth (for \middle errors)

    this.leftrightDepth = 0;
  }
  /**
   * Checks a result to make sure it has the right type, and throws an
   * appropriate error otherwise.
   */


  var _proto = Parser.prototype;

  _proto.expect = function expect(text, consume) {
    if (consume === void 0) {
      consume = true;
    }

    if (this.nextToken.text !== text) {
      throw new src_ParseError("Expected '" + text + "', got '" + this.nextToken.text + "'", this.nextToken);
    }

    if (consume) {
      this.consume();
    }
  }
  /**
   * Considers the current look ahead token as consumed,
   * and fetches the one after that as the new look ahead.
   */
  ;

  _proto.consume = function consume() {
    this.nextToken = this.gullet.expandNextToken();
  }
  /**
   * Switches between "text" and "math" modes.
   */
  ;

  _proto.switchMode = function switchMode(newMode) {
    this.mode = newMode;
    this.gullet.switchMode(newMode);
  }
  /**
   * Main parsing function, which parses an entire input.
   */
  ;

  _proto.parse = function parse() {
    // Create a group namespace for the math expression.
    // (LaTeX creates a new group for every $...$, $$...$$, \[...\].)
    this.gullet.beginGroup(); // Use old \color behavior (same as LaTeX's \textcolor) if requested.
    // We do this within the group for the math expression, so it doesn't
    // pollute settings.macros.

    if (this.settings.colorIsTextColor) {
      this.gullet.macros.set("\\color", "\\textcolor");
    } // Try to parse the input


    this.consume();
    var parse = this.parseExpression(false); // If we succeeded, make sure there's an EOF at the end

    this.expect("EOF", false); // End the group namespace for the expression

    this.gullet.endGroup();
    return parse;
  };

  _proto.parseExpression = function parseExpression(breakOnInfix, breakOnTokenText) {
    var body = []; // Keep adding atoms to the body until we can't parse any more atoms (either
    // we reached the end, a }, or a \right)

    while (true) {
      // Ignore spaces in math mode
      if (this.mode === "math") {
        this.consumeSpaces();
      }

      var lex = this.nextToken;

      if (Parser.endOfExpression.indexOf(lex.text) !== -1) {
        break;
      }

      if (breakOnTokenText && lex.text === breakOnTokenText) {
        break;
      }

      if (breakOnInfix && src_functions[lex.text] && src_functions[lex.text].infix) {
        break;
      }

      var atom = this.parseAtom(breakOnTokenText);

      if (!atom) {
        break;
      }

      body.push(atom);
    }

    if (this.mode === "text") {
      this.formLigatures(body);
    }

    return this.handleInfixNodes(body);
  }
  /**
   * Rewrites infix operators such as \over with corresponding commands such
   * as \frac.
   *
   * There can only be one infix operator per group.  If there's more than one
   * then the expression is ambiguous.  This can be resolved by adding {}.
   */
  ;

  _proto.handleInfixNodes = function handleInfixNodes(body) {
    var overIndex = -1;
    var funcName;

    for (var i = 0; i < body.length; i++) {
      var node = checkNodeType(body[i], "infix");

      if (node) {
        if (overIndex !== -1) {
          throw new src_ParseError("only one infix operator per group", node.token);
        }

        overIndex = i;
        funcName = node.replaceWith;
      }
    }

    if (overIndex !== -1 && funcName) {
      var numerNode;
      var denomNode;
      var numerBody = body.slice(0, overIndex);
      var denomBody = body.slice(overIndex + 1);

      if (numerBody.length === 1 && numerBody[0].type === "ordgroup") {
        numerNode = numerBody[0];
      } else {
        numerNode = {
          type: "ordgroup",
          mode: this.mode,
          body: numerBody
        };
      }

      if (denomBody.length === 1 && denomBody[0].type === "ordgroup") {
        denomNode = denomBody[0];
      } else {
        denomNode = {
          type: "ordgroup",
          mode: this.mode,
          body: denomBody
        };
      }

      var _node;

      if (funcName === "\\\\abovefrac") {
        _node = this.callFunction(funcName, [numerNode, body[overIndex], denomNode], []);
      } else {
        _node = this.callFunction(funcName, [numerNode, denomNode], []);
      }

      return [_node];
    } else {
      return body;
    }
  } // The greediness of a superscript or subscript
  ;

  /**
   * Handle a subscript or superscript with nice errors.
   */
  _proto.handleSupSubscript = function handleSupSubscript(name) {
    var symbolToken = this.nextToken;
    var symbol = symbolToken.text;
    this.consume();
    this.consumeSpaces(); // ignore spaces before sup/subscript argument

    var group = this.parseGroup(name, false, Parser.SUPSUB_GREEDINESS);

    if (!group) {
      throw new src_ParseError("Expected group after '" + symbol + "'", symbolToken);
    }

    return group;
  }
  /**
   * Converts the textual input of an unsupported command into a text node
   * contained within a color node whose color is determined by errorColor
   */
  ;

  _proto.handleUnsupportedCmd = function handleUnsupportedCmd() {
    var text = this.nextToken.text;
    var textordArray = [];

    for (var i = 0; i < text.length; i++) {
      textordArray.push({
        type: "textord",
        mode: "text",
        text: text[i]
      });
    }

    var textNode = {
      type: "text",
      mode: this.mode,
      body: textordArray
    };
    var colorNode = {
      type: "color",
      mode: this.mode,
      color: this.settings.errorColor,
      body: [textNode]
    };
    this.consume();
    return colorNode;
  }
  /**
   * Parses a group with optional super/subscripts.
   */
  ;

  _proto.parseAtom = function parseAtom(breakOnTokenText) {
    // The body of an atom is an implicit group, so that things like
    // \left(x\right)^2 work correctly.
    var base = this.parseGroup("atom", false, null, breakOnTokenText); // In text mode, we don't have superscripts or subscripts

    if (this.mode === "text") {
      return base;
    } // Note that base may be empty (i.e. null) at this point.


    var superscript;
    var subscript;

    while (true) {
      // Guaranteed in math mode, so eat any spaces first.
      this.consumeSpaces(); // Lex the first token

      var lex = this.nextToken;

      if (lex.text === "\\limits" || lex.text === "\\nolimits") {
        // We got a limit control
        var opNode = checkNodeType(base, "op");

        if (opNode) {
          var limits = lex.text === "\\limits";
          opNode.limits = limits;
          opNode.alwaysHandleSupSub = true;
        } else {
          throw new src_ParseError("Limit controls must follow a math operator", lex);
        }

        this.consume();
      } else if (lex.text === "^") {
        // We got a superscript start
        if (superscript) {
          throw new src_ParseError("Double superscript", lex);
        }

        superscript = this.handleSupSubscript("superscript");
      } else if (lex.text === "_") {
        // We got a subscript start
        if (subscript) {
          throw new src_ParseError("Double subscript", lex);
        }

        subscript = this.handleSupSubscript("subscript");
      } else if (lex.text === "'") {
        // We got a prime
        if (superscript) {
          throw new src_ParseError("Double superscript", lex);
        }

        var prime = {
          type: "textord",
          mode: this.mode,
          text: "\\prime"
        }; // Many primes can be grouped together, so we handle this here

        var primes = [prime];
        this.consume(); // Keep lexing tokens until we get something that's not a prime

        while (this.nextToken.text === "'") {
          // For each one, add another prime to the list
          primes.push(prime);
          this.consume();
        } // If there's a superscript following the primes, combine that
        // superscript in with the primes.


        if (this.nextToken.text === "^") {
          primes.push(this.handleSupSubscript("superscript"));
        } // Put everything into an ordgroup as the superscript


        superscript = {
          type: "ordgroup",
          mode: this.mode,
          body: primes
        };
      } else {
        // If it wasn't ^, _, or ', stop parsing super/subscripts
        break;
      }
    } // Base must be set if superscript or subscript are set per logic above,
    // but need to check here for type check to pass.


    if (superscript || subscript) {
      // If we got either a superscript or subscript, create a supsub
      return {
        type: "supsub",
        mode: this.mode,
        base: base,
        sup: superscript,
        sub: subscript
      };
    } else {
      // Otherwise return the original body
      return base;
    }
  }
  /**
   * Parses an entire function, including its base and all of its arguments.
   */
  ;

  _proto.parseFunction = function parseFunction(breakOnTokenText, name, // For error reporting.
  greediness) {
    var token = this.nextToken;
    var func = token.text;
    var funcData = src_functions[func];

    if (!funcData) {
      return null;
    }

    if (greediness != null && funcData.greediness <= greediness) {
      throw new src_ParseError("Got function '" + func + "' with no arguments" + (name ? " as " + name : ""), token);
    } else if (this.mode === "text" && !funcData.allowedInText) {
      throw new src_ParseError("Can't use function '" + func + "' in text mode", token);
    } else if (this.mode === "math" && funcData.allowedInMath === false) {
      throw new src_ParseError("Can't use function '" + func + "' in math mode", token);
    } // hyperref package sets the catcode of % as an active character


    if (funcData.argTypes && funcData.argTypes[0] === "url") {
      this.gullet.lexer.setCatcode("%", 13);
    } // Consume the command token after possibly switching to the
    // mode specified by the function (for instant mode switching),
    // and then immediately switch back.


    if (funcData.consumeMode) {
      var oldMode = this.mode;
      this.switchMode(funcData.consumeMode);
      this.consume();
      this.switchMode(oldMode);
    } else {
      this.consume();
    }

    var _this$parseArguments = this.parseArguments(func, funcData),
        args = _this$parseArguments.args,
        optArgs = _this$parseArguments.optArgs;

    return this.callFunction(func, args, optArgs, token, breakOnTokenText);
  }
  /**
   * Call a function handler with a suitable context and arguments.
   */
  ;

  _proto.callFunction = function callFunction(name, args, optArgs, token, breakOnTokenText) {
    var context = {
      funcName: name,
      parser: this,
      token: token,
      breakOnTokenText: breakOnTokenText
    };
    var func = src_functions[name];

    if (func && func.handler) {
      return func.handler(context, args, optArgs);
    } else {
      throw new src_ParseError("No function handler for " + name);
    }
  }
  /**
   * Parses the arguments of a function or environment
   */
  ;

  _proto.parseArguments = function parseArguments(func, // Should look like "\name" or "\begin{name}".
  funcData) {
    var totalArgs = funcData.numArgs + funcData.numOptionalArgs;

    if (totalArgs === 0) {
      return {
        args: [],
        optArgs: []
      };
    }

    var baseGreediness = funcData.greediness;
    var args = [];
    var optArgs = [];

    for (var i = 0; i < totalArgs; i++) {
      var argType = funcData.argTypes && funcData.argTypes[i];
      var isOptional = i < funcData.numOptionalArgs; // Ignore spaces between arguments.  As the TeXbook says:
      // "After you have said ‘\def\row#1#2{...}’, you are allowed to
      //  put spaces between the arguments (e.g., ‘\row x n’), because
      //  TeX doesn’t use single spaces as undelimited arguments."

      if (i > 0 && !isOptional) {
        this.consumeSpaces();
      } // Also consume leading spaces in math mode, as parseSymbol
      // won't know what to do with them.  This can only happen with
      // macros, e.g. \frac\foo\foo where \foo expands to a space symbol.
      // In LaTeX, the \foo's get treated as (blank) arguments).
      // In KaTeX, for now, both spaces will get consumed.
      // TODO(edemaine)


      if (i === 0 && !isOptional && this.mode === "math") {
        this.consumeSpaces();
      }

      var nextToken = this.nextToken;
      var arg = this.parseGroupOfType("argument to '" + func + "'", argType, isOptional, baseGreediness);

      if (!arg) {
        if (isOptional) {
          optArgs.push(null);
          continue;
        }

        throw new src_ParseError("Expected group after '" + func + "'", nextToken);
      }

      (isOptional ? optArgs : args).push(arg);
    }

    return {
      args: args,
      optArgs: optArgs
    };
  }
  /**
   * Parses a group when the mode is changing.
   */
  ;

  _proto.parseGroupOfType = function parseGroupOfType(name, type, optional, greediness) {
    switch (type) {
      case "color":
        return this.parseColorGroup(optional);

      case "size":
        return this.parseSizeGroup(optional);

      case "url":
        return this.parseUrlGroup(optional);

      case "math":
      case "text":
        return this.parseGroup(name, optional, greediness, undefined, type);

      case "raw":
        {
          if (optional && this.nextToken.text === "{") {
            return null;
          }

          var token = this.parseStringGroup("raw", optional, true);

          if (token) {
            return {
              type: "raw",
              mode: "text",
              string: token.text
            };
          } else {
            throw new src_ParseError("Expected raw group", this.nextToken);
          }
        }

      case "original":
      case null:
      case undefined:
        return this.parseGroup(name, optional, greediness);

      default:
        throw new src_ParseError("Unknown group type as " + name, this.nextToken);
    }
  };

  _proto.consumeSpaces = function consumeSpaces() {
    while (this.nextToken.text === " ") {
      this.consume();
    }
  }
  /**
   * Parses a group, essentially returning the string formed by the
   * brace-enclosed tokens plus some position information.
   */
  ;

  _proto.parseStringGroup = function parseStringGroup(modeName, // Used to describe the mode in error messages.
  optional, raw) {
    var groupBegin = optional ? "[" : "{";
    var groupEnd = optional ? "]" : "}";
    var nextToken = this.nextToken;

    if (nextToken.text !== groupBegin) {
      if (optional) {
        return null;
      } else if (raw && nextToken.text !== "EOF" && /[^{}[\]]/.test(nextToken.text)) {
        // allow a single character in raw string group
        this.gullet.lexer.setCatcode("%", 14); // reset the catcode of %

        this.consume();
        return nextToken;
      }
    }

    var outerMode = this.mode;
    this.mode = "text";
    this.expect(groupBegin);
    var str = "";
    var firstToken = this.nextToken;
    var nested = 0; // allow nested braces in raw string group

    var lastToken = firstToken;

    while (raw && nested > 0 || this.nextToken.text !== groupEnd) {
      switch (this.nextToken.text) {
        case "EOF":
          throw new src_ParseError("Unexpected end of input in " + modeName, firstToken.range(lastToken, str));

        case groupBegin:
          nested++;
          break;

        case groupEnd:
          nested--;
          break;
      }

      lastToken = this.nextToken;
      str += lastToken.text;
      this.consume();
    }

    this.mode = outerMode;
    this.gullet.lexer.setCatcode("%", 14); // reset the catcode of %

    this.expect(groupEnd);
    return firstToken.range(lastToken, str);
  }
  /**
   * Parses a regex-delimited group: the largest sequence of tokens
   * whose concatenated strings match `regex`. Returns the string
   * formed by the tokens plus some position information.
   */
  ;

  _proto.parseRegexGroup = function parseRegexGroup(regex, modeName) {
    var outerMode = this.mode;
    this.mode = "text";
    var firstToken = this.nextToken;
    var lastToken = firstToken;
    var str = "";

    while (this.nextToken.text !== "EOF" && regex.test(str + this.nextToken.text)) {
      lastToken = this.nextToken;
      str += lastToken.text;
      this.consume();
    }

    if (str === "") {
      throw new src_ParseError("Invalid " + modeName + ": '" + firstToken.text + "'", firstToken);
    }

    this.mode = outerMode;
    return firstToken.range(lastToken, str);
  }
  /**
   * Parses a color description.
   */
  ;

  _proto.parseColorGroup = function parseColorGroup(optional) {
    var res = this.parseStringGroup("color", optional);

    if (!res) {
      return null;
    }

    var match = /^(#[a-f0-9]{3}|#?[a-f0-9]{6}|[a-z]+)$/i.exec(res.text);

    if (!match) {
      throw new src_ParseError("Invalid color: '" + res.text + "'", res);
    }

    var color = match[0];

    if (/^[0-9a-f]{6}$/i.test(color)) {
      // We allow a 6-digit HTML color spec without a leading "#".
      // This follows the xcolor package's HTML color model.
      // Predefined color names are all missed by this RegEx pattern.
      color = "#" + color;
    }

    return {
      type: "color-token",
      mode: this.mode,
      color: color
    };
  }
  /**
   * Parses a size specification, consisting of magnitude and unit.
   */
  ;

  _proto.parseSizeGroup = function parseSizeGroup(optional) {
    var res;
    var isBlank = false;

    if (!optional && this.nextToken.text !== "{") {
      res = this.parseRegexGroup(/^[-+]? *(?:$|\d+|\d+\.\d*|\.\d*) *[a-z]{0,2} *$/, "size");
    } else {
      res = this.parseStringGroup("size", optional);
    }

    if (!res) {
      return null;
    }

    if (!optional && res.text.length === 0) {
      // Because we've tested for what is !optional, this block won't
      // affect \kern, \hspace, etc. It will capture the mandatory arguments
      // to \genfrac and \above.
      res.text = "0pt"; // Enable \above{}

      isBlank = true; // This is here specifically for \genfrac
    }

    var match = /([-+]?) *(\d+(?:\.\d*)?|\.\d+) *([a-z]{2})/.exec(res.text);

    if (!match) {
      throw new src_ParseError("Invalid size: '" + res.text + "'", res);
    }

    var data = {
      number: +(match[1] + match[2]),
      // sign + magnitude, cast to number
      unit: match[3]
    };

    if (!validUnit(data)) {
      throw new src_ParseError("Invalid unit: '" + data.unit + "'", res);
    }

    return {
      type: "size",
      mode: this.mode,
      value: data,
      isBlank: isBlank
    };
  }
  /**
   * Parses an URL, checking escaped letters and allowed protocols.
   */
  ;

  _proto.parseUrlGroup = function parseUrlGroup(optional) {
    var res = this.parseStringGroup("url", optional, true); // get raw string

    if (!res) {
      return null;
    } // hyperref package allows backslashes alone in href, but doesn't
    // generate valid links in such cases; we interpret this as
    // "undefined" behaviour, and keep them as-is. Some browser will
    // replace backslashes with forward slashes.


    var url = res.text.replace(/\\([#$%&~_^{}])/g, '$1');
    var protocol = /^\s*([^\\/#]*?)(?::|&#0*58|&#x0*3a)/i.exec(url);
    protocol = protocol != null ? protocol[1] : "_relative";
    var allowed = this.settings.allowedProtocols;

    if (!utils.contains(allowed, "*") && !utils.contains(allowed, protocol)) {
      throw new src_ParseError("Forbidden protocol '" + protocol + "'", res);
    }

    return {
      type: "url",
      mode: this.mode,
      url: url
    };
  }
  /**
   * If `optional` is false or absent, this parses an ordinary group,
   * which is either a single nucleus (like "x") or an expression
   * in braces (like "{x+y}") or an implicit group, a group that starts
   * at the current position, and ends right before a higher explicit
   * group ends, or at EOF.
   * If `optional` is true, it parses either a bracket-delimited expression
   * (like "[x+y]") or returns null to indicate the absence of a
   * bracket-enclosed group.
   * If `mode` is present, switches to that mode while parsing the group,
   * and switches back after.
   */
  ;

  _proto.parseGroup = function parseGroup(name, // For error reporting.
  optional, greediness, breakOnTokenText, mode) {
    var outerMode = this.mode;
    var firstToken = this.nextToken;
    var text = firstToken.text; // Switch to specified mode

    if (mode) {
      this.switchMode(mode);
    }

    var groupEnd;
    var result; // Try to parse an open brace or \begingroup

    if (optional ? text === "[" : text === "{" || text === "\\begingroup") {
      groupEnd = Parser.endOfGroup[text]; // Start a new group namespace

      this.gullet.beginGroup(); // If we get a brace, parse an expression

      this.consume();
      var expression = this.parseExpression(false, groupEnd);
      var lastToken = this.nextToken; // End group namespace before consuming symbol after close brace

      this.gullet.endGroup();
      result = {
        type: "ordgroup",
        mode: this.mode,
        loc: SourceLocation.range(firstToken, lastToken),
        body: expression,
        // A group formed by \begingroup...\endgroup is a semi-simple group
        // which doesn't affect spacing in math mode, i.e., is transparent.
        // https://tex.stackexchange.com/questions/1930/when-should-one-
        // use-begingroup-instead-of-bgroup
        semisimple: text === "\\begingroup" || undefined
      };
    } else if (optional) {
      // Return nothing for an optional group
      result = null;
    } else {
      // If there exists a function with this name, parse the function.
      // Otherwise, just return a nucleus
      result = this.parseFunction(breakOnTokenText, name, greediness) || this.parseSymbol();

      if (result == null && text[0] === "\\" && !implicitCommands.hasOwnProperty(text)) {
        if (this.settings.throwOnError) {
          throw new src_ParseError("Undefined control sequence: " + text, firstToken);
        }

        result = this.handleUnsupportedCmd();
      }
    } // Switch mode back


    if (mode) {
      this.switchMode(outerMode);
    } // Make sure we got a close brace


    if (groupEnd) {
      this.expect(groupEnd);
    }

    return result;
  }
  /**
   * Form ligature-like combinations of characters for text mode.
   * This includes inputs like "--", "---", "``" and "''".
   * The result will simply replace multiple textord nodes with a single
   * character in each value by a single textord node having multiple
   * characters in its value.  The representation is still ASCII source.
   * The group will be modified in place.
   */
  ;

  _proto.formLigatures = function formLigatures(group) {
    var n = group.length - 1;

    for (var i = 0; i < n; ++i) {
      var a = group[i]; // $FlowFixMe: Not every node type has a `text` property.

      var v = a.text;

      if (v === "-" && group[i + 1].text === "-") {
        if (i + 1 < n && group[i + 2].text === "-") {
          group.splice(i, 3, {
            type: "textord",
            mode: "text",
            loc: SourceLocation.range(a, group[i + 2]),
            text: "---"
          });
          n -= 2;
        } else {
          group.splice(i, 2, {
            type: "textord",
            mode: "text",
            loc: SourceLocation.range(a, group[i + 1]),
            text: "--"
          });
          n -= 1;
        }
      }

      if ((v === "'" || v === "`") && group[i + 1].text === v) {
        group.splice(i, 2, {
          type: "textord",
          mode: "text",
          loc: SourceLocation.range(a, group[i + 1]),
          text: v + v
        });
        n -= 1;
      }
    }
  }
  /**
   * Parse a single symbol out of the string. Here, we handle single character
   * symbols and special functions like verbatim
   */
  ;

  _proto.parseSymbol = function parseSymbol() {
    var nucleus = this.nextToken;
    var text = nucleus.text;

    if (/^\\verb[^a-zA-Z]/.test(text)) {
      this.consume();
      var arg = text.slice(5);
      var star = arg.charAt(0) === "*";

      if (star) {
        arg = arg.slice(1);
      } // Lexer's tokenRegex is constructed to always have matching
      // first/last characters.


      if (arg.length < 2 || arg.charAt(0) !== arg.slice(-1)) {
        throw new src_ParseError("\\verb assertion failed --\n                    please report what input caused this bug");
      }

      arg = arg.slice(1, -1); // remove first and last char

      return {
        type: "verb",
        mode: "text",
        body: arg,
        star: star
      };
    } // At this point, we should have a symbol, possibly with accents.
    // First expand any accented base symbol according to unicodeSymbols.


    if (unicodeSymbols.hasOwnProperty(text[0]) && !src_symbols[this.mode][text[0]]) {
      // This behavior is not strict (XeTeX-compatible) in math mode.
      if (this.settings.strict && this.mode === "math") {
        this.settings.reportNonstrict("unicodeTextInMathMode", "Accented Unicode text character \"" + text[0] + "\" used in " + "math mode", nucleus);
      }

      text = unicodeSymbols[text[0]] + text.substr(1);
    } // Strip off any combining characters


    var match = combiningDiacriticalMarksEndRegex.exec(text);

    if (match) {
      text = text.substring(0, match.index);

      if (text === 'i') {
        text = "\u0131"; // dotless i, in math and text mode
      } else if (text === 'j') {
        text = "\u0237"; // dotless j, in math and text mode
      }
    } // Recognize base symbol


    var symbol;

    if (src_symbols[this.mode][text]) {
      if (this.settings.strict && this.mode === 'math' && extraLatin.indexOf(text) >= 0) {
        this.settings.reportNonstrict("unicodeTextInMathMode", "Latin-1/Unicode text character \"" + text[0] + "\" used in " + "math mode", nucleus);
      }

      var group = src_symbols[this.mode][text].group;
      var loc = SourceLocation.range(nucleus);
      var s;

      if (ATOMS.hasOwnProperty(group)) {
        // $FlowFixMe
        var family = group;
        s = {
          type: "atom",
          mode: this.mode,
          family: family,
          loc: loc,
          text: text
        };
      } else {
        // $FlowFixMe
        s = {
          type: group,
          mode: this.mode,
          loc: loc,
          text: text
        };
      }

      symbol = s;
    } else if (text.charCodeAt(0) >= 0x80) {
      // no symbol for e.g. ^
      if (this.settings.strict) {
        if (!supportedCodepoint(text.charCodeAt(0))) {
          this.settings.reportNonstrict("unknownSymbol", "Unrecognized Unicode character \"" + text[0] + "\"" + (" (" + text.charCodeAt(0) + ")"), nucleus);
        } else if (this.mode === "math") {
          this.settings.reportNonstrict("unicodeTextInMathMode", "Unicode text character \"" + text[0] + "\" used in math mode", nucleus);
        }
      }

      symbol = {
        type: "textord",
        mode: this.mode,
        loc: SourceLocation.range(nucleus),
        text: text
      };
    } else {
      return null; // EOF, ^, _, {, }, etc.
    }

    this.consume(); // Transform combining characters into accents

    if (match) {
      for (var i = 0; i < match[0].length; i++) {
        var accent = match[0][i];

        if (!unicodeAccents[accent]) {
          throw new src_ParseError("Unknown accent ' " + accent + "'", nucleus);
        }

        var command = unicodeAccents[accent][this.mode];

        if (!command) {
          throw new src_ParseError("Accent " + accent + " unsupported in " + this.mode + " mode", nucleus);
        }

        symbol = {
          type: "accent",
          mode: this.mode,
          loc: SourceLocation.range(nucleus),
          label: command,
          isStretchy: false,
          isShifty: true,
          base: symbol
        };
      }
    }

    return symbol;
  };

  return Parser;
}();

Parser_Parser.endOfExpression = ["}", "\\endgroup", "\\end", "\\right", "&"];
Parser_Parser.endOfGroup = {
  "[": "]",
  "{": "}",
  "\\begingroup": "\\endgroup"
  /**
   * Parses an "expression", which is a list of atoms.
   *
   * `breakOnInfix`: Should the parsing stop when we hit infix nodes? This
   *                 happens when functions have higher precendence han infix
   *                 nodes in implicit parses.
   *
   * `breakOnTokenText`: The text of the token that the expression should end
   *                     with, or `null` if something else should end the
   *                     expression.
   */

};
Parser_Parser.SUPSUB_GREEDINESS = 1;

// CONCATENATED MODULE: ./src/parseTree.js
/**
 * Provides a single function for parsing an expression using a Parser
 * TODO(emily): Remove this
 */



/**
 * Parses an expression using a Parser, then returns the parsed result.
 */
var parseTree_parseTree = function parseTree(toParse, settings) {
  if (!(typeof toParse === 'string' || toParse instanceof String)) {
    throw new TypeError('KaTeX can only parse string typed expression');
  }

  var parser = new Parser_Parser(toParse, settings); // Blank out any \df@tag to avoid spurious "Duplicate \tag" errors

  delete parser.gullet.macros.current["\\df@tag"];
  var tree = parser.parse(); // If the input used \tag, it will set the \df@tag macro to the tag.
  // In this case, we separately parse the tag and wrap the tree.

  if (parser.gullet.macros.get("\\df@tag")) {
    if (!settings.displayMode) {
      throw new src_ParseError("\\tag works only in display equations");
    }

    parser.gullet.feed("\\df@tag");
    tree = [{
      type: "tag",
      mode: "text",
      body: tree,
      tag: parser.parse()
    }];
  }

  return tree;
};

/* harmony default export */ var src_parseTree = (parseTree_parseTree);
// CONCATENATED MODULE: ./katex.js
/* eslint no-console:0 */

/**
 * This is the main entry point for KaTeX. Here, we expose functions for
 * rendering expressions either to DOM nodes or to markup strings.
 *
 * We also expose the ParseError class to check if errors thrown from KaTeX are
 * errors in the expression, or errors in javascript handling.
 */










/**
 * Parse and build an expression, and place that expression in the DOM node
 * given.
 */
var katex_render = function render(expression, baseNode, options) {
  baseNode.textContent = "";
  var node = katex_renderToDomTree(expression, options).toNode();
  baseNode.appendChild(node);
}; // KaTeX's styles don't work properly in quirks mode. Print out an error, and
// disable rendering.


if (typeof document !== "undefined") {
  if (document.compatMode !== "CSS1Compat") {
    typeof console !== "undefined" && console.warn("Warning: KaTeX doesn't work in quirks mode. Make sure your " + "website has a suitable doctype.");

    katex_render = function render() {
      throw new src_ParseError("KaTeX doesn't work in quirks mode.");
    };
  }
}
/**
 * Parse and build an expression, and return the markup for that.
 */


var renderToString = function renderToString(expression, options) {
  var markup = katex_renderToDomTree(expression, options).toMarkup();
  return markup;
};
/**
 * Parse an expression and return the parse tree.
 */


var katex_generateParseTree = function generateParseTree(expression, options) {
  var settings = new src_Settings(options);
  return src_parseTree(expression, settings);
};
/**
 * If the given error is a KaTeX ParseError and options.throwOnError is false,
 * renders the invalid LaTeX as a span with hover title giving the KaTeX
 * error message.  Otherwise, simply throws the error.
 */


var katex_renderError = function renderError(error, expression, options) {
  if (options.throwOnError || !(error instanceof src_ParseError)) {
    throw error;
  }

  var node = buildCommon.makeSpan(["katex-error"], [new domTree_SymbolNode(expression)]);
  node.setAttribute("title", error.toString());
  node.setAttribute("style", "color:" + options.errorColor);
  return node;
};
/**
 * Generates and returns the katex build tree. This is used for advanced
 * use cases (like rendering to custom output).
 */


var katex_renderToDomTree = function renderToDomTree(expression, options) {
  var settings = new src_Settings(options);

  try {
    var tree = src_parseTree(expression, settings);
    return buildTree_buildTree(tree, expression, settings);
  } catch (error) {
    return katex_renderError(error, expression, settings);
  }
};
/**
 * Generates and returns the katex build tree, with just HTML (no MathML).
 * This is used for advanced use cases (like rendering to custom output).
 */


var katex_renderToHTMLTree = function renderToHTMLTree(expression, options) {
  var settings = new src_Settings(options);

  try {
    var tree = src_parseTree(expression, settings);
    return buildTree_buildHTMLTree(tree, expression, settings);
  } catch (error) {
    return katex_renderError(error, expression, settings);
  }
};

/* harmony default export */ var katex_0 = ({
  /**
   * Current KaTeX version
   */
  version: "0.10.2",

  /**
   * Renders the given LaTeX into an HTML+MathML combination, and adds
   * it as a child to the specified DOM node.
   */
  render: katex_render,

  /**
   * Renders the given LaTeX into an HTML+MathML combination string,
   * for sending to the client.
   */
  renderToString: renderToString,

  /**
   * KaTeX error, usually during parsing.
   */
  ParseError: src_ParseError,

  /**
   * Parses the given LaTeX into KaTeX's internal parse tree structure,
   * without rendering to HTML or MathML.
   *
   * NOTE: This method is not currently recommended for public use.
   * The internal tree representation is unstable and is very likely
   * to change. Use at your own risk.
   */
  __parse: katex_generateParseTree,

  /**
   * Renders the given LaTeX into an HTML+MathML internal DOM tree
   * representation, without flattening that representation to a string.
   *
   * NOTE: This method is not currently recommended for public use.
   * The internal tree representation is unstable and is very likely
   * to change. Use at your own risk.
   */
  __renderToDomTree: katex_renderToDomTree,

  /**
   * Renders the given LaTeX into an HTML internal DOM tree representation,
   * without MathML and without flattening that representation to a string.
   *
   * NOTE: This method is not currently recommended for public use.
   * The internal tree representation is unstable and is very likely
   * to change. Use at your own risk.
   */
  __renderToHTMLTree: katex_renderToHTMLTree,

  /**
   * extends internal font metrics object with a new object
   * each key in the new object represents a font name
  */
  __setFontMetrics: setFontMetrics,

  /**
   * adds a new symbol to builtin symbols table
   */
  __defineSymbol: defineSymbol,

  /**
   * adds a new macro to builtin macro list
   */
  __defineMacro: defineMacro,

  /**
   * Expose the dom tree node types, which can be useful for type checking nodes.
   *
   * NOTE: This method is not currently recommended for public use.
   * The internal tree representation is unstable and is very likely
   * to change. Use at your own risk.
   */
  __domTree: {
    Span: domTree_Span,
    Anchor: domTree_Anchor,
    SymbolNode: domTree_SymbolNode,
    SvgNode: SvgNode,
    PathNode: domTree_PathNode,
    LineNode: LineNode
  }
});
// CONCATENATED MODULE: ./katex.webpack.js
/**
 * This is the webpack entry point for KaTeX. As ECMAScript, flow[1] and jest[2]
 * doesn't support CSS modules natively, a separate entry point is used and
 * it is not flowtyped.
 *
 * [1] https://gist.github.com/lambdahands/d19e0da96285b749f0ef
 * [2] https://facebook.github.io/jest/docs/en/webpack.html
 */


/* harmony default export */ var katex_webpack = __webpack_exports__["default"] = (katex_0);

/***/ })
/******/ ])["default"];
});

/***/ }),
/* 8 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return _SidebarDisplay; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Display_js__ = __webpack_require__(4);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__ = __webpack_require__(3);





class Sidebar {
    constructor(parentContainer) {
        this.adjuster = document.createElement("div");
        this.adjuster.style.backgroundColor = "rgba(0,0,0,0.125)";
        this.adjuster.style.height = "100%";
        this.adjuster.style.cursor = "ew-resize";
        this.adjuster.style.width = "2px";

        parentContainer.appendChild(this.adjuster);

        this.resize = this.resize.bind(this);
        this.stopResize = this.stopResize.bind(this);

        this.adjuster.addEventListener("mousedown", (function(e) {
            e.preventDefault();
            window.addEventListener('mousemove', this.resize);
            window.addEventListener('mouseup', this.stopResize);
        }).bind(this));

        this.sidebar = document.createElement("div");
        this.sidebar.style.height = "100%";
        this.sidebar.style.width = "240px";
        this.sidebar.style.border = "none";
        this.sidebar.style.display = "flex";
        this.sidebar.style.flexDirection = "column";
        this.sidebar.className = "card";

        parentContainer.appendChild(this.sidebar);

        this.main_div = document.createElement("div");
        this.main_div.style.overflow = "auto";
        this.sidebar.appendChild(this.main_div);

        let filler = document.createElement("div");
        filler.style.flexGrow = "1";
        this.sidebar.appendChild(filler);

        this.footer_div = document.createElement("div");
        this.sidebar.appendChild(this.footer_div);

        this.panels = [];
        this.currentPanel = null;
    }

    addPanel(panel) {
        this.panels.push(panel);
        return this.panels.length;
    }

    init(display) {
        this.display = display;
        this.footer = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["b" /* Panel */](this.footer_div, display);
    }

    resize(e) {
        let width = this.sidebar.getBoundingClientRect().right - e.pageX;
        this.sidebar.style.width = `${width}px`;
    }

    stopResize() {
        window.removeEventListener('mousemove', this.resize);
        window.removeEventListener('mouseup', this.stopResize);
        this.display.resize();
    }

    showPanel(panel) {
        if (!panel) panel = this.currentPanel;
        this.currentPanel = panel;

        for (let x of this.panels) {
            if (x == panel)
                x.show();
            else
                x.hide();
        }
    }
}

class SidebarDisplay extends __WEBPACK_IMPORTED_MODULE_0__Display_js__["a" /* Display */] {
    constructor(container, sseq) {
        if (typeof container == "string")
            container = document.querySelector(container);

        container.style.display = "flex";
        container.style.displayDirection = "row";

        let child = document.createElement("div");
        child.style.height = "100%";
        child.style.minHeight = "100%";
        child.style.overflow = "hidden";
        child.style.position = "relative";
        child.style.flexGrow = "1";

        container.appendChild(child);

        let sidebar = new Sidebar(container)

        super(child, sseq);

        this.sidebar = sidebar;
        this.sidebar.init(this);
    }
}

const _SidebarDisplay = SidebarDisplay;



/***/ }),
/* 9 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
Object.defineProperty(__webpack_exports__, "__esModule", { value: true });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_chart_SocketDisplay_js__ = __webpack_require__(10);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1_chart_interface_panel_mod_js__ = __webpack_require__(3);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2_chart_interface_Tooltip_js__ = __webpack_require__(2);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2_chart_interface_Tooltip_js___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_2_chart_interface_Tooltip_js__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3_mousetrap__ = __webpack_require__(24);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3_mousetrap___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_3_mousetrap__);

"use strict"





class TableDisplay extends __WEBPACK_IMPORTED_MODULE_0_chart_SocketDisplay_js__["a" /* SocketDisplay */] {
    constructor(container, sseq) {
        super(container, sseq);
        this.tooltip = new __WEBPACK_IMPORTED_MODULE_2_chart_interface_Tooltip_js__["Tooltip"](this);
        this.on("mouseover-class", this._onMouseoverClass.bind(this));
        this.on("mouseout-class", this._onMouseoutClass.bind(this));
        this.on("mouseover-bidegree", this._onMouseoverBidegree.bind(this));
        this.on("mouseout-bidegree", this._onMouseoutBidegree.bind(this));

        this.on("click", this._onClick.bind(this));
        this.tablePanel = new __WEBPACK_IMPORTED_MODULE_1_chart_interface_panel_mod_js__["e" /* TablePanel */](this.sidebar.main_div, this);
        this.tablePanel.show();
    }

    _onClick() {
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
/* harmony export (immutable) */ __webpack_exports__["TableDisplay"] = TableDisplay;



/***/ }),
/* 10 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__interface_mod_js__ = __webpack_require__(11);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__SocketListener_js__ = __webpack_require__(23);



class SocketDisplay extends __WEBPACK_IMPORTED_MODULE_0__interface_mod_js__["a" /* SidebarDisplay */] {
    constructor(container, socket) {
        super(container);
        this.socket = new __WEBPACK_IMPORTED_MODULE_1__SocketListener_js__["a" /* SocketListener */](socket);
        this._onclick = this._onclick.bind(this);
        this.on("click", this._onclick);
        this.add_message_handlers_from_object(default_message_handlers);
    }

    add_message_handlers_from_object(handlers) {
        for(let [cmd_filter, handler] of Object.entries(handlers)) {
            this.socket.add_message_handler(cmd_filter, handler.bind(this));
        }
    }

    send(...args){
        this.socket.send(...args);
    }

    console_log_if_debug(msg) {
        if(this.debug_mode) {
            console.log(msg);
        }
    }

    _onclick(o){        
        this.send("click", { "chart_class" : o.mouseover_class, "x" : o.real_x, "y" : o.real_y });
    }
}
/* harmony export (immutable) */ __webpack_exports__["a"] = SocketDisplay;


let default_message_handlers = {
    "start" : function(){
        this.send("new_user", {});
    },

    "initialize.chart.state" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.setSseq(SpectralSequenceChart.from_JSON(kwargs.state));
        this.y_clip_offset = this.sseq.y_clip_offset;
        this.send("initialize.complete", {});
    },

    "chart.batched" : function(cmd, args, kwargs) {
        console.log("chart.batched", kwargs.messages);
        for(let msg of kwargs.messages) {
            try {
                this.socket.handle_message(msg);
            } catch(err) {
                console.error(err);
            }
        }
        this.update()
    },
    
    "chart.state.reset" : function(cmd, args, kwargs) {
        this.console_log_if_debug("accepted user:", kwargs.state);
        this.setSseq(SpectralSequenceChart.from_JSON(kwargs.state));
        if(kwargs.display_state !== undefined){
            this.set_display_state(kwargs.display_state);
        }
        this.y_clip_offset = this.sseq.y_clip_offset;
    },

    "chart.set_x_range" : function(cmd, args, kwargs){
        this.sseq.x_range = [kwargs.x_min, kwargs.x_max];
    },
    "chart.set_y_range" : function(cmd, args, kwargs){
        this.sseq.y_range = [kwargs.y_min, kwargs.y_max];
    },
    "chart.set_initial_x_range" : function(cmd, args, kwargs){
        this.sseq.initial_x_range = [kwargs.x_min, kwargs.x_max];
    },
    "chart.set_initial_y_range" : function(cmd, args, kwargs){
        this.sseq.initial_y_range = [kwargs.y_min, kwargs.y_max];
    },    
    "chart.insert_page_range" : function(cmd, args, kwargs) {
        this.sseq.page_list.splice(kwargs.idx, 0, kwargs.page_range);
    },

    "chart.node.add" : function(cmd, args, kwargs) {
        this.console_log_if_debug("add node", cmd, kwargs)
        // this.info(msg);
    },

    "chart.class.add" : function(cmd, args, kwargs) {
        let c = this.sseq.add_class(kwargs.new_class);
        this.update();
    },

    "chart.class.update" : function(cmd, args, kwargs) {
        let c = kwargs.class_to_update;
        Object.assign(this.sseq.classes[c.uuid], c);
    },

    "chart.class.set_name" : function(cmd, args, kwargs) {
        let [x,y,idx] = load_args({
            "x" : Number.isInteger, 
            "y" : Number.isInteger, 
            "idx" : Number.isInteger
        });
        this.sseq.classes_by_degree.get([kwargs.x, msg.arguments.y])[msg.arguments.idx].name = msg.arguments.name;
    },

    "chart.edge.add" : function(cmd, args, kwargs) {
        console.log("chart.edge.add");
        this.console_log_if_debug(kwargs);
        this.sseq.add_edge(kwargs);
    },

    "chart.edge.update" : function(cmd, args, kwargs) {
        this.console_log_if_debug(kwargs);
        let e = kwargs.edge_to_update;
        Object.assign(this.sseq.edges[e.uuid], e);
    },

    "display.set_background_color" : function(cmd, args, kwargs) {
        this.display.setBackgroundColor(kwargs.color);
    },

    "interact.alert" : function(cmd, args, kwargs) {
        alert(kwargs.msg);
    },
    "interact.prompt" : function(cmd, args, kwargs) {
        let result = prompt(kwargs.msg, kwargs.default);
        this.send("interact.result", {"result" : result});
    }
};

/***/ }),
/* 11 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__display_mod_js__ = __webpack_require__(12);
/* unused harmony reexport BasicDisplay */
/* unused harmony reexport Display */
/* unused harmony reexport EditorDisplay */
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return __WEBPACK_IMPORTED_MODULE_0__display_mod_js__["a"]; });
/* unused harmony reexport TableDisplay */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Undo__ = __webpack_require__(21);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Undo___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_1__Undo__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__SaveLoad__ = __webpack_require__(22);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__SaveLoad___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_2__SaveLoad__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3__panel_mod_js__ = __webpack_require__(3);
/* unused harmony reexport Undo */
/* unused harmony reexport IO */
/* unused harmony reexport Panel */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_4__Tooltip_js__ = __webpack_require__(2);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_4__Tooltip_js___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_4__Tooltip_js__);
/* unused harmony reexport Tooltip */








/***/ }),
/* 12 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__BasicDisplay_js__ = __webpack_require__(13);
/* unused harmony reexport BasicDisplay */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Display_js__ = __webpack_require__(4);
/* unused harmony reexport Display */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__EditorDisplay_js__ = __webpack_require__(16);
/* unused harmony reexport EditorDisplay */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3__SidebarDisplay_js__ = __webpack_require__(8);
/* harmony reexport (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return __WEBPACK_IMPORTED_MODULE_3__SidebarDisplay_js__["a"]; });





/***/ }),
/* 13 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* unused harmony export BasicDisplay */
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Display_js__ = __webpack_require__(4);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Tooltip_js__ = __webpack_require__(2);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Tooltip_js___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_1__Tooltip_js__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__Latex_js__ = __webpack_require__(0);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3_mousetrap__ = __webpack_require__(5);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3_mousetrap___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_3_mousetrap__);








class BasicDisplay extends __WEBPACK_IMPORTED_MODULE_0__Display_js__["a" /* Display */] {
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

        this.tooltip = new __WEBPACK_IMPORTED_MODULE_1__Tooltip_js__["Tooltip"](this);
        this.on("mouseover", (node) => {
            this.tooltip.setHTML(this.getClassTooltipHTML(node, this.page));
            this.tooltip.show(node._canvas_x, node._canvas_y);
        });
        this.on("mouseout", () => this.tooltip.hide());

        Object(__WEBPACK_IMPORTED_MODULE_3_mousetrap__["bind"])('left',  this.previousPage);
        Object(__WEBPACK_IMPORTED_MODULE_3_mousetrap__["bind"])('right', this.nextPage);
        Object(__WEBPACK_IMPORTED_MODULE_3_mousetrap__["bind"])('x',
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

        if (extra_info) {
            tooltip += extra_info;
        }

        return tooltip;
    }

    getClassTooltipHTML(c, page) {
        return Object(__WEBPACK_IMPORTED_MODULE_2__Latex_js__["a" /* renderLatex */])(this.getClassTooltip(c,page));
    }

    static toTooltipString(obj, page) {
        if (!obj) {
            return false;
        }

        if(obj.constructor === String){
            return obj;
        }

        if(obj.constructor === Array) {
            return obj.map((x) => __WEBPACK_IMPORTED_MODULE_1__Tooltip_js__["Tooltip"].toTooltipString(x, page)).filter((x) => x).join("\n");
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

const _BasicDisplay = BasicDisplay;



/***/ }),
/* 14 */
/***/ (function(module, exports, __webpack_require__) {

var require;var require;(function(f){if(true){module.exports=f()}else if(typeof define==="function"&&define.amd){define([],f)}else{var g;if(typeof window!=="undefined"){g=window}else if(typeof global!=="undefined"){g=global}else if(typeof self!=="undefined"){g=self}else{g=this}g.d3 = f()}})(function(){var define,module,exports;return (function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c="function"==typeof require&&require;if(!f&&c)return require(i,!0);if(u)return u(i,!0);var a=new Error("Cannot find module '"+i+"'");throw a.code="MODULE_NOT_FOUND",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u="function"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({1:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.map=exports.slice=void 0;var array=Array.prototype,slice=array.slice;exports.slice=slice;var map=array.map;exports.map=map;

},{}],2:[function(require,module,exports){
"use strict";function _default(e,t){return e<t?-1:e>t?1:e>=t?0:NaN}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],3:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _array=require("./array.js"),_bisect=_interopRequireDefault(require("./bisect.js")),_constant=_interopRequireDefault(require("./constant.js")),_extent=_interopRequireDefault(require("./extent.js")),_identity=_interopRequireDefault(require("./identity.js")),_range=_interopRequireDefault(require("./range.js")),_ticks=require("./ticks.js"),_sturges=_interopRequireDefault(require("./threshold/sturges.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(){var e=_identity.default,t=_extent.default,r=_sturges.default;function n(n){Array.isArray(n)||(n=Array.from(n));var u,a,i=n.length,s=new Array(i);for(u=0;u<i;++u)s[u]=e(n[u],u,n);var f=t(s),o=f[0],l=f[1],_=r(s,o,l);Array.isArray(_)||(_=(0,_ticks.tickStep)(o,l,_),_=(0,_range.default)(Math.ceil(o/_)*_,l,_));for(var c=_.length;_[0]<=o;)_.shift(),--c;for(;_[c-1]>l;)_.pop(),--c;var d,y=new Array(c+1);for(u=0;u<=c;++u)(d=y[u]=[]).x0=u>0?_[u-1]:o,d.x1=u<c?_[u]:l;for(u=0;u<i;++u)o<=(a=s[u])&&a<=l&&y[(0,_bisect.default)(_,a,0,c)].push(n[u]);return y}return n.value=function(t){return arguments.length?(e="function"==typeof t?t:(0,_constant.default)(t),n):e},n.domain=function(e){return arguments.length?(t="function"==typeof e?e:(0,_constant.default)([e[0],e[1]]),n):t},n.thresholds=function(e){return arguments.length?(r="function"==typeof e?e:Array.isArray(e)?(0,_constant.default)(_array.slice.call(e)):(0,_constant.default)(e),n):r},n}

},{"./array.js":1,"./bisect.js":4,"./constant.js":6,"./extent.js":12,"./identity.js":16,"./range.js":32,"./threshold/sturges.js":38,"./ticks.js":39}],4:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=exports.bisectLeft=exports.bisectRight=void 0;var _ascending=_interopRequireDefault(require("./ascending.js")),_bisector=_interopRequireDefault(require("./bisector.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var ascendingBisect=(0,_bisector.default)(_ascending.default),bisectRight=ascendingBisect.right;exports.bisectRight=bisectRight;var bisectLeft=ascendingBisect.left;exports.bisectLeft=bisectLeft;var _default=bisectRight;exports.default=_default;

},{"./ascending.js":2,"./bisector.js":5}],5:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _ascending=_interopRequireDefault(require("./ascending.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return 1===e.length&&(e=ascendingComparator(e)),{left:function(n,t,r,u){for(null==r&&(r=0),null==u&&(u=n.length);r<u;){var l=r+u>>>1;e(n[l],t)<0?r=l+1:u=l}return r},right:function(n,t,r,u){for(null==r&&(r=0),null==u&&(u=n.length);r<u;){var l=r+u>>>1;e(n[l],t)>0?u=l:r=l+1}return r}}}function ascendingComparator(e){return function(n,t){return(0,_ascending.default)(e(n),t)}}

},{"./ascending.js":2}],6:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],7:[function(require,module,exports){
"use strict";function count(e,t){let l=0;if(void 0===t)for(let t of e)null!=t&&(t=+t)>=t&&++l;else{let o=-1;for(let u of e)null!=(u=t(u,++o,e))&&(u=+u)>=u&&++l}return l}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=count;

},{}],8:[function(require,module,exports){
"use strict";function length(e){return 0|e.length}function empty(e){return!(e>0)}function arrayify(e){return"object"!=typeof e||"length"in e?e:Array.from(e)}function reducer(e){return r=>e(...r)}function cross(...e){const r="function"==typeof e[e.length-1]&&reducer(e.pop()),t=(e=e.map(arrayify)).map(length),n=e.length-1,o=new Array(n+1).fill(0),u=[];if(n<0||t.some(empty))return u;for(;;){u.push(o.map((r,t)=>e[t][r]));let f=n;for(;++o[f]===t[f];){if(0===f)return r?u.map(r):u;o[f--]=0}}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=cross;

},{}],9:[function(require,module,exports){
"use strict";function cumsum(e,r){var t=0,u=0;return Float64Array.from(e,void 0===r?e=>t+=+e||0:o=>t+=+r(o,u++,e)||0)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=cumsum;

},{}],10:[function(require,module,exports){
"use strict";function _default(e,t){return t<e?-1:t>e?1:t>=e?0:NaN}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],11:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=deviation;var _variance=_interopRequireDefault(require("./variance.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function deviation(e,t){const r=(0,_variance.default)(e,t);return r?Math.sqrt(r):r}

},{"./variance.js":41}],12:[function(require,module,exports){
"use strict";function _default(e,t){let l,o;if(void 0===t)for(const t of e)null!=t&&(void 0===l?t>=t&&(l=o=t):(l>t&&(l=t),o<t&&(o=t)));else{let f=-1;for(let u of e)null!=(u=t(u,++f,e))&&(void 0===l?u>=u&&(l=o=u):(l>u&&(l=u),o<u&&(o=u)))}return[l,o]}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],13:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=greatest;var _ascending=_interopRequireDefault(require("./ascending.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function greatest(e,t=_ascending.default){let n,r=!1;if(1===t.length){let s;for(const u of e){const e=t(u);(r?(0,_ascending.default)(e,s)>0:0===(0,_ascending.default)(e,e))&&(n=u,s=e,r=!0)}}else for(const s of e)(r?t(s,n)>0:0===t(s,s))&&(n=s,r=!0);return n}

},{"./ascending.js":2}],14:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=greatestIndex;var _ascending=_interopRequireDefault(require("./ascending.js")),_maxIndex=_interopRequireDefault(require("./maxIndex.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function greatestIndex(e,t=_ascending.default){if(1===t.length)return(0,_maxIndex.default)(e,t);let r,n=-1,u=-1;for(const a of e)++u,(n<0?0===t(a,a):t(a,r)>0)&&(r=a,n=u);return n}

},{"./ascending.js":2,"./maxIndex.js":21}],15:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=group,exports.groups=groups,exports.rollup=rollup,exports.rollups=rollups;var _identity=_interopRequireDefault(require("./identity.js"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function group(t,...e){return nest(t,_identity.default,_identity.default,e)}function groups(t,...e){return nest(t,Array.from,_identity.default,e)}function rollup(t,e,...r){return nest(t,_identity.default,e,r)}function rollups(t,e,...r){return nest(t,Array.from,e,r)}function nest(t,e,r,n){return function t(u,o){if(o>=n.length)return r(u);const s=new Map,i=n[o++];let l=-1;for(const t of u){const e=i(t,++l,u),r=s.get(e);r?r.push(t):s.set(e,[t])}for(const[e,r]of s)s.set(e,t(r,o));return e(s)}(t,0)}

},{"./identity.js":16}],16:[function(require,module,exports){
"use strict";function _default(e){return e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],17:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"bisect",{enumerable:!0,get:function(){return _bisect.default}}),Object.defineProperty(exports,"bisectRight",{enumerable:!0,get:function(){return _bisect.bisectRight}}),Object.defineProperty(exports,"bisectLeft",{enumerable:!0,get:function(){return _bisect.bisectLeft}}),Object.defineProperty(exports,"ascending",{enumerable:!0,get:function(){return _ascending.default}}),Object.defineProperty(exports,"bisector",{enumerable:!0,get:function(){return _bisector.default}}),Object.defineProperty(exports,"count",{enumerable:!0,get:function(){return _count.default}}),Object.defineProperty(exports,"cross",{enumerable:!0,get:function(){return _cross.default}}),Object.defineProperty(exports,"cumsum",{enumerable:!0,get:function(){return _cumsum.default}}),Object.defineProperty(exports,"descending",{enumerable:!0,get:function(){return _descending.default}}),Object.defineProperty(exports,"deviation",{enumerable:!0,get:function(){return _deviation.default}}),Object.defineProperty(exports,"extent",{enumerable:!0,get:function(){return _extent.default}}),Object.defineProperty(exports,"group",{enumerable:!0,get:function(){return _group.default}}),Object.defineProperty(exports,"groups",{enumerable:!0,get:function(){return _group.groups}}),Object.defineProperty(exports,"rollup",{enumerable:!0,get:function(){return _group.rollup}}),Object.defineProperty(exports,"rollups",{enumerable:!0,get:function(){return _group.rollups}}),Object.defineProperty(exports,"bin",{enumerable:!0,get:function(){return _bin.default}}),Object.defineProperty(exports,"histogram",{enumerable:!0,get:function(){return _bin.default}}),Object.defineProperty(exports,"thresholdFreedmanDiaconis",{enumerable:!0,get:function(){return _freedmanDiaconis.default}}),Object.defineProperty(exports,"thresholdScott",{enumerable:!0,get:function(){return _scott.default}}),Object.defineProperty(exports,"thresholdSturges",{enumerable:!0,get:function(){return _sturges.default}}),Object.defineProperty(exports,"max",{enumerable:!0,get:function(){return _max.default}}),Object.defineProperty(exports,"maxIndex",{enumerable:!0,get:function(){return _maxIndex.default}}),Object.defineProperty(exports,"mean",{enumerable:!0,get:function(){return _mean.default}}),Object.defineProperty(exports,"median",{enumerable:!0,get:function(){return _median.default}}),Object.defineProperty(exports,"merge",{enumerable:!0,get:function(){return _merge.default}}),Object.defineProperty(exports,"min",{enumerable:!0,get:function(){return _min.default}}),Object.defineProperty(exports,"minIndex",{enumerable:!0,get:function(){return _minIndex.default}}),Object.defineProperty(exports,"pairs",{enumerable:!0,get:function(){return _pairs.default}}),Object.defineProperty(exports,"permute",{enumerable:!0,get:function(){return _permute.default}}),Object.defineProperty(exports,"quantile",{enumerable:!0,get:function(){return _quantile.default}}),Object.defineProperty(exports,"quantileSorted",{enumerable:!0,get:function(){return _quantile.quantileSorted}}),Object.defineProperty(exports,"quickselect",{enumerable:!0,get:function(){return _quickselect.default}}),Object.defineProperty(exports,"range",{enumerable:!0,get:function(){return _range.default}}),Object.defineProperty(exports,"least",{enumerable:!0,get:function(){return _least.default}}),Object.defineProperty(exports,"leastIndex",{enumerable:!0,get:function(){return _leastIndex.default}}),Object.defineProperty(exports,"greatest",{enumerable:!0,get:function(){return _greatest.default}}),Object.defineProperty(exports,"greatestIndex",{enumerable:!0,get:function(){return _greatestIndex.default}}),Object.defineProperty(exports,"scan",{enumerable:!0,get:function(){return _scan.default}}),Object.defineProperty(exports,"shuffle",{enumerable:!0,get:function(){return _shuffle.default}}),Object.defineProperty(exports,"sum",{enumerable:!0,get:function(){return _sum.default}}),Object.defineProperty(exports,"ticks",{enumerable:!0,get:function(){return _ticks.default}}),Object.defineProperty(exports,"tickIncrement",{enumerable:!0,get:function(){return _ticks.tickIncrement}}),Object.defineProperty(exports,"tickStep",{enumerable:!0,get:function(){return _ticks.tickStep}}),Object.defineProperty(exports,"transpose",{enumerable:!0,get:function(){return _transpose.default}}),Object.defineProperty(exports,"variance",{enumerable:!0,get:function(){return _variance.default}}),Object.defineProperty(exports,"zip",{enumerable:!0,get:function(){return _zip.default}});var _bisect=_interopRequireWildcard(require("./bisect.js")),_ascending=_interopRequireDefault(require("./ascending.js")),_bisector=_interopRequireDefault(require("./bisector.js")),_count=_interopRequireDefault(require("./count.js")),_cross=_interopRequireDefault(require("./cross.js")),_cumsum=_interopRequireDefault(require("./cumsum.js")),_descending=_interopRequireDefault(require("./descending.js")),_deviation=_interopRequireDefault(require("./deviation.js")),_extent=_interopRequireDefault(require("./extent.js")),_group=_interopRequireWildcard(require("./group.js")),_bin=_interopRequireDefault(require("./bin.js")),_freedmanDiaconis=_interopRequireDefault(require("./threshold/freedmanDiaconis.js")),_scott=_interopRequireDefault(require("./threshold/scott.js")),_sturges=_interopRequireDefault(require("./threshold/sturges.js")),_max=_interopRequireDefault(require("./max.js")),_maxIndex=_interopRequireDefault(require("./maxIndex.js")),_mean=_interopRequireDefault(require("./mean.js")),_median=_interopRequireDefault(require("./median.js")),_merge=_interopRequireDefault(require("./merge.js")),_min=_interopRequireDefault(require("./min.js")),_minIndex=_interopRequireDefault(require("./minIndex.js")),_pairs=_interopRequireDefault(require("./pairs.js")),_permute=_interopRequireDefault(require("./permute.js")),_quantile=_interopRequireWildcard(require("./quantile.js")),_quickselect=_interopRequireDefault(require("./quickselect.js")),_range=_interopRequireDefault(require("./range.js")),_least=_interopRequireDefault(require("./least.js")),_leastIndex=_interopRequireDefault(require("./leastIndex.js")),_greatest=_interopRequireDefault(require("./greatest.js")),_greatestIndex=_interopRequireDefault(require("./greatestIndex.js")),_scan=_interopRequireDefault(require("./scan.js")),_shuffle=_interopRequireDefault(require("./shuffle.js")),_sum=_interopRequireDefault(require("./sum.js")),_ticks=_interopRequireWildcard(require("./ticks.js")),_transpose=_interopRequireDefault(require("./transpose.js")),_variance=_interopRequireDefault(require("./variance.js")),_zip=_interopRequireDefault(require("./zip.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var i=n?Object.getOwnPropertyDescriptor(e,u):null;i&&(i.get||i.set)?Object.defineProperty(t,u,i):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}

},{"./ascending.js":2,"./bin.js":3,"./bisect.js":4,"./bisector.js":5,"./count.js":7,"./cross.js":8,"./cumsum.js":9,"./descending.js":10,"./deviation.js":11,"./extent.js":12,"./greatest.js":13,"./greatestIndex.js":14,"./group.js":15,"./least.js":18,"./leastIndex.js":19,"./max.js":20,"./maxIndex.js":21,"./mean.js":22,"./median.js":23,"./merge.js":24,"./min.js":25,"./minIndex.js":26,"./pairs.js":28,"./permute.js":29,"./quantile.js":30,"./quickselect.js":31,"./range.js":32,"./scan.js":33,"./shuffle.js":34,"./sum.js":35,"./threshold/freedmanDiaconis.js":36,"./threshold/scott.js":37,"./threshold/sturges.js":38,"./ticks.js":39,"./transpose.js":40,"./variance.js":41,"./zip.js":42}],18:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=least;var _ascending=_interopRequireDefault(require("./ascending.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function least(e,t=_ascending.default){let n,r=!1;if(1===t.length){let s;for(const u of e){const e=t(u);(r?(0,_ascending.default)(e,s)<0:0===(0,_ascending.default)(e,e))&&(n=u,s=e,r=!0)}}else for(const s of e)(r?t(s,n)<0:0===t(s,s))&&(n=s,r=!0);return n}

},{"./ascending.js":2}],19:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=leastIndex;var _ascending=_interopRequireDefault(require("./ascending.js")),_minIndex=_interopRequireDefault(require("./minIndex.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function leastIndex(e,n=_ascending.default){if(1===n.length)return(0,_minIndex.default)(e,n);let t,r=-1,u=-1;for(const i of e)++u,(r<0?0===n(i,i):n(i,t)<0)&&(t=i,r=u);return r}

},{"./ascending.js":2,"./minIndex.js":26}],20:[function(require,module,exports){
"use strict";function max(e,o){let t;if(void 0===o)for(const o of e)null!=o&&(t<o||void 0===t&&o>=o)&&(t=o);else{let l=-1;for(let r of e)null!=(r=o(r,++l,e))&&(t<r||void 0===t&&r>=r)&&(t=r)}return t}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=max;

},{}],21:[function(require,module,exports){
"use strict";function maxIndex(e,o){let t,l=-1,n=-1;if(void 0===o)for(const o of e)++n,null!=o&&(t<o||void 0===t&&o>=o)&&(t=o,l=n);else for(let r of e)null!=(r=o(r,++n,e))&&(t<r||void 0===t&&r>=r)&&(t=r,l=n);return l}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=maxIndex;

},{}],22:[function(require,module,exports){
"use strict";function mean(e,t){let l=0,o=0;if(void 0===t)for(let t of e)null!=t&&(t=+t)>=t&&(++l,o+=t);else{let f=-1;for(let r of e)null!=(r=t(r,++f,e))&&(r=+r)>=r&&(++l,o+=r)}if(l)return o/l}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=mean;

},{}],23:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _quantile=_interopRequireDefault(require("./quantile.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){return(0,_quantile.default)(e,.5,t)}

},{"./quantile.js":30}],24:[function(require,module,exports){
"use strict";function*flatten(e){for(const t of e)yield*t}function merge(e){return Array.from(flatten(e))}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=merge;

},{}],25:[function(require,module,exports){
"use strict";function min(e,o){let t;if(void 0===o)for(const o of e)null!=o&&(t>o||void 0===t&&o>=o)&&(t=o);else{let l=-1;for(let i of e)null!=(i=o(i,++l,e))&&(t>i||void 0===t&&i>=i)&&(t=i)}return t}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=min;

},{}],26:[function(require,module,exports){
"use strict";function minIndex(e,o){let t,n=-1,l=-1;if(void 0===o)for(const o of e)++l,null!=o&&(t>o||void 0===t&&o>=o)&&(t=o,n=l);else for(let i of e)null!=(i=o(i,++l,e))&&(t>i||void 0===t&&i>=i)&&(t=i,n=l);return n}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=minIndex;

},{}],27:[function(require,module,exports){
"use strict";function _default(e){return null===e?NaN:+e}function*numbers(e,l){if(void 0===l)for(let l of e)null!=l&&(l=+l)>=l&&(yield l);else{let t=-1;for(let u of e)null!=(u=l(u,++t,e))&&(u=+u)>=u&&(yield u)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.numbers=numbers;

},{}],28:[function(require,module,exports){
"use strict";function pairs(r,e=pair){const t=[];let o,p=!1;for(const s of r)p&&t.push(e(o,s)),o=s,p=!0;return t}function pair(r,e){return[r,e]}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=pairs,exports.pair=pair;

},{}],29:[function(require,module,exports){
"use strict";function _default(e,t){return Array.from(t,t=>e[t])}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],30:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=quantile,exports.quantileSorted=quantileSorted;var _max=_interopRequireDefault(require("./max.js")),_min=_interopRequireDefault(require("./min.js")),_quickselect=_interopRequireDefault(require("./quickselect.js")),_number=_interopRequireWildcard(require("./number.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},u=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var n in e)if(Object.prototype.hasOwnProperty.call(e,n)){var i=u?Object.getOwnPropertyDescriptor(e,n):null;i&&(i.get||i.set)?Object.defineProperty(t,n,i):t[n]=e[n]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function quantile(e,r,t){if(u=(e=Float64Array.from((0,_number.numbers)(e,t))).length){if((r=+r)<=0||u<2)return(0,_min.default)(e);if(r>=1)return(0,_max.default)(e);var u,n=(u-1)*r,i=Math.floor(n),a=(0,_max.default)((0,_quickselect.default)(e,i).subarray(0,i+1));return a+((0,_min.default)(e.subarray(i+1))-a)*(n-i)}}function quantileSorted(e,r,t=_number.default){if(u=e.length){if((r=+r)<=0||u<2)return+t(e[0],0,e);if(r>=1)return+t(e[u-1],u-1,e);var u,n=(u-1)*r,i=Math.floor(n),a=+t(e[i],i,e);return a+(+t(e[i+1],i+1,e)-a)*(n-i)}}

},{"./max.js":20,"./min.js":25,"./number.js":27,"./quickselect.js":31}],31:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=quickselect;var _ascending=_interopRequireDefault(require("./ascending.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function quickselect(e,t,a=0,r=e.length-1,s=_ascending.default){for(;r>a;){if(r-a>600){const n=r-a+1,o=t-a+1,u=Math.log(n),i=.5*Math.exp(2*u/3),c=.5*Math.sqrt(u*i*(n-i)/n)*(o-n/2<0?-1:1);quickselect(e,t,Math.max(a,Math.floor(t-o*i/n+c)),Math.min(r,Math.floor(t+(n-o)*i/n+c)),s)}const n=e[t];let o=a,u=r;for(swap(e,a,t),s(e[r],n)>0&&swap(e,a,r);o<u;){for(swap(e,o,u),++o,--u;s(e[o],n)<0;)++o;for(;s(e[u],n)>0;)--u}0===s(e[a],n)?swap(e,a,u):swap(e,++u,r),u<=t&&(a=u+1),t<=u&&(r=u-1)}return e}function swap(e,t,a){const r=e[t];e[t]=e[a],e[a]=r}

},{"./ascending.js":2}],32:[function(require,module,exports){
"use strict";function _default(e,t,r){e=+e,t=+t,r=(u=arguments.length)<2?(t=e,e=0,1):u<3?1:+r;for(var a=-1,u=0|Math.max(0,Math.ceil((t-e)/r)),l=new Array(u);++a<u;)l[a]=e+a*r;return l}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],33:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=scan;var _leastIndex=_interopRequireDefault(require("./leastIndex.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function scan(e,t){const r=(0,_leastIndex.default)(e,t);return r<0?void 0:r}

},{"./leastIndex.js":19}],34:[function(require,module,exports){
"use strict";function shuffle(e,t=0,r=e.length){for(var f,u,o=r-(t=+t);o;)u=Math.random()*o--|0,f=e[o+t],e[o+t]=e[u+t],e[u+t]=f;return e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=shuffle;

},{}],35:[function(require,module,exports){
"use strict";function sum(e,t){let o=0;if(void 0===t)for(let t of e)(t=+t)&&(o+=t);else{let r=-1;for(let f of e)(f=+t(f,++r,e))&&(o+=f)}return o}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=sum;

},{}],36:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _count=_interopRequireDefault(require("../count.js")),_quantile=_interopRequireDefault(require("../quantile.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t,u){return Math.ceil((u-t)/(2*((0,_quantile.default)(e,.75)-(0,_quantile.default)(e,.25))*Math.pow((0,_count.default)(e),-1/3)))}

},{"../count.js":7,"../quantile.js":30}],37:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _count=_interopRequireDefault(require("../count.js")),_deviation=_interopRequireDefault(require("../deviation.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t,u){return Math.ceil((u-t)/(3.5*(0,_deviation.default)(e)*Math.pow((0,_count.default)(e),-1/3)))}

},{"../count.js":7,"../deviation.js":11}],38:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _count=_interopRequireDefault(require("../count.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return Math.ceil(Math.log((0,_count.default)(e))/Math.LN2)+1}

},{"../count.js":7}],39:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.tickIncrement=tickIncrement,exports.tickStep=tickStep;var e10=Math.sqrt(50),e5=Math.sqrt(10),e2=Math.sqrt(2);function _default(t,e,r){var a,o,n,M,h=-1;if(r=+r,(t=+t)===(e=+e)&&r>0)return[t];if((a=e<t)&&(o=t,t=e,e=o),0===(M=tickIncrement(t,e,r))||!isFinite(M))return[];if(M>0)for(t=Math.ceil(t/M),e=Math.floor(e/M),n=new Array(o=Math.ceil(e-t+1));++h<o;)n[h]=(t+h)*M;else for(t=Math.floor(t*M),e=Math.ceil(e*M),n=new Array(o=Math.ceil(t-e+1));++h<o;)n[h]=(t-h)/M;return a&&n.reverse(),n}function tickIncrement(t,e,r){var a=(e-t)/Math.max(0,r),o=Math.floor(Math.log(a)/Math.LN10),n=a/Math.pow(10,o);return o>=0?(n>=e10?10:n>=e5?5:n>=e2?2:1)*Math.pow(10,o):-Math.pow(10,-o)/(n>=e10?10:n>=e5?5:n>=e2?2:1)}function tickStep(t,e,r){var a=Math.abs(e-t)/Math.max(0,r),o=Math.pow(10,Math.floor(Math.log(a)/Math.LN10)),n=a/o;return n>=e10?o*=10:n>=e5?o*=5:n>=e2&&(o*=2),e<t?-o:o}

},{}],40:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _min=_interopRequireDefault(require("./min.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){if(!(u=e.length))return[];for(var r=-1,t=(0,_min.default)(e,length),n=new Array(t);++r<t;)for(var u,f=-1,i=n[r]=new Array(u);++f<u;)i[f]=e[f][r];return n}function length(e){return e.length}

},{"./min.js":25}],41:[function(require,module,exports){
"use strict";function variance(e,t){let l,r=0,o=0,f=0;if(void 0===t)for(let t of e)null!=t&&(t=+t)>=t&&(f+=(l=t-o)*(t-(o+=l/++r)));else{let i=-1;for(let n of e)null!=(n=t(n,++i,e))&&(n=+n)>=n&&(f+=(l=n-o)*(n-(o+=l/++r)))}if(r>1)return f/(r-1)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=variance;

},{}],42:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _transpose=_interopRequireDefault(require("./transpose.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(){return(0,_transpose.default)(arguments)}

},{"./transpose.js":40}],43:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.Color=Color,exports.default=color,exports.rgbConvert=rgbConvert,exports.rgb=rgb,exports.Rgb=Rgb,exports.hslConvert=hslConvert,exports.hsl=hsl,exports.brighter=exports.darker=void 0;var _define=_interopRequireWildcard(require("./define.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var a in e)if(Object.prototype.hasOwnProperty.call(e,a)){var i=n?Object.getOwnPropertyDescriptor(e,a):null;i&&(i.get||i.set)?Object.defineProperty(t,a,i):t[a]=e[a]}return t.default=e,r&&r.set(e,t),t}function Color(){}var darker=.7;exports.darker=darker;var brighter=1/darker;exports.brighter=brighter;var reI="\\s*([+-]?\\d+)\\s*",reN="\\s*([+-]?\\d*\\.?\\d+(?:[eE][+-]?\\d+)?)\\s*",reP="\\s*([+-]?\\d*\\.?\\d+(?:[eE][+-]?\\d+)?)%\\s*",reHex=/^#([0-9a-f]{3,8})$/,reRgbInteger=new RegExp("^rgb\\("+[reI,reI,reI]+"\\)$"),reRgbPercent=new RegExp("^rgb\\("+[reP,reP,reP]+"\\)$"),reRgbaInteger=new RegExp("^rgba\\("+[reI,reI,reI,reN]+"\\)$"),reRgbaPercent=new RegExp("^rgba\\("+[reP,reP,reP,reN]+"\\)$"),reHslPercent=new RegExp("^hsl\\("+[reN,reP,reP]+"\\)$"),reHslaPercent=new RegExp("^hsla\\("+[reN,reP,reP,reN]+"\\)$"),named={aliceblue:15792383,antiquewhite:16444375,aqua:65535,aquamarine:8388564,azure:15794175,beige:16119260,bisque:16770244,black:0,blanchedalmond:16772045,blue:255,blueviolet:9055202,brown:10824234,burlywood:14596231,cadetblue:6266528,chartreuse:8388352,chocolate:13789470,coral:16744272,cornflowerblue:6591981,cornsilk:16775388,crimson:14423100,cyan:65535,darkblue:139,darkcyan:35723,darkgoldenrod:12092939,darkgray:11119017,darkgreen:25600,darkgrey:11119017,darkkhaki:12433259,darkmagenta:9109643,darkolivegreen:5597999,darkorange:16747520,darkorchid:10040012,darkred:9109504,darksalmon:15308410,darkseagreen:9419919,darkslateblue:4734347,darkslategray:3100495,darkslategrey:3100495,darkturquoise:52945,darkviolet:9699539,deeppink:16716947,deepskyblue:49151,dimgray:6908265,dimgrey:6908265,dodgerblue:2003199,firebrick:11674146,floralwhite:16775920,forestgreen:2263842,fuchsia:16711935,gainsboro:14474460,ghostwhite:16316671,gold:16766720,goldenrod:14329120,gray:8421504,green:32768,greenyellow:11403055,grey:8421504,honeydew:15794160,hotpink:16738740,indianred:13458524,indigo:4915330,ivory:16777200,khaki:15787660,lavender:15132410,lavenderblush:16773365,lawngreen:8190976,lemonchiffon:16775885,lightblue:11393254,lightcoral:15761536,lightcyan:14745599,lightgoldenrodyellow:16448210,lightgray:13882323,lightgreen:9498256,lightgrey:13882323,lightpink:16758465,lightsalmon:16752762,lightseagreen:2142890,lightskyblue:8900346,lightslategray:7833753,lightslategrey:7833753,lightsteelblue:11584734,lightyellow:16777184,lime:65280,limegreen:3329330,linen:16445670,magenta:16711935,maroon:8388608,mediumaquamarine:6737322,mediumblue:205,mediumorchid:12211667,mediumpurple:9662683,mediumseagreen:3978097,mediumslateblue:8087790,mediumspringgreen:64154,mediumturquoise:4772300,mediumvioletred:13047173,midnightblue:1644912,mintcream:16121850,mistyrose:16770273,moccasin:16770229,navajowhite:16768685,navy:128,oldlace:16643558,olive:8421376,olivedrab:7048739,orange:16753920,orangered:16729344,orchid:14315734,palegoldenrod:15657130,palegreen:10025880,paleturquoise:11529966,palevioletred:14381203,papayawhip:16773077,peachpuff:16767673,peru:13468991,pink:16761035,plum:14524637,powderblue:11591910,purple:8388736,rebeccapurple:6697881,red:16711680,rosybrown:12357519,royalblue:4286945,saddlebrown:9127187,salmon:16416882,sandybrown:16032864,seagreen:3050327,seashell:16774638,sienna:10506797,silver:12632256,skyblue:8900331,slateblue:6970061,slategray:7372944,slategrey:7372944,snow:16775930,springgreen:65407,steelblue:4620980,tan:13808780,teal:32896,thistle:14204888,tomato:16737095,turquoise:4251856,violet:15631086,wheat:16113331,white:16777215,whitesmoke:16119285,yellow:16776960,yellowgreen:10145074};function color_formatHex(){return this.rgb().formatHex()}function color_formatHsl(){return hslConvert(this).formatHsl()}function color_formatRgb(){return this.rgb().formatRgb()}function color(e){var r,t;return e=(e+"").trim().toLowerCase(),(r=reHex.exec(e))?(t=r[1].length,r=parseInt(r[1],16),6===t?rgbn(r):3===t?new Rgb(r>>8&15|r>>4&240,r>>4&15|240&r,(15&r)<<4|15&r,1):8===t?rgba(r>>24&255,r>>16&255,r>>8&255,(255&r)/255):4===t?rgba(r>>12&15|r>>8&240,r>>8&15|r>>4&240,r>>4&15|240&r,((15&r)<<4|15&r)/255):null):(r=reRgbInteger.exec(e))?new Rgb(r[1],r[2],r[3],1):(r=reRgbPercent.exec(e))?new Rgb(255*r[1]/100,255*r[2]/100,255*r[3]/100,1):(r=reRgbaInteger.exec(e))?rgba(r[1],r[2],r[3],r[4]):(r=reRgbaPercent.exec(e))?rgba(255*r[1]/100,255*r[2]/100,255*r[3]/100,r[4]):(r=reHslPercent.exec(e))?hsla(r[1],r[2]/100,r[3]/100,1):(r=reHslaPercent.exec(e))?hsla(r[1],r[2]/100,r[3]/100,r[4]):named.hasOwnProperty(e)?rgbn(named[e]):"transparent"===e?new Rgb(NaN,NaN,NaN,0):null}function rgbn(e){return new Rgb(e>>16&255,e>>8&255,255&e,1)}function rgba(e,r,t,n){return n<=0&&(e=r=t=NaN),new Rgb(e,r,t,n)}function rgbConvert(e){return e instanceof Color||(e=color(e)),e?new Rgb((e=e.rgb()).r,e.g,e.b,e.opacity):new Rgb}function rgb(e,r,t,n){return 1===arguments.length?rgbConvert(e):new Rgb(e,r,t,null==n?1:n)}function Rgb(e,r,t,n){this.r=+e,this.g=+r,this.b=+t,this.opacity=+n}function rgb_formatHex(){return"#"+hex(this.r)+hex(this.g)+hex(this.b)}function rgb_formatRgb(){var e=this.opacity;return(1===(e=isNaN(e)?1:Math.max(0,Math.min(1,e)))?"rgb(":"rgba(")+Math.max(0,Math.min(255,Math.round(this.r)||0))+", "+Math.max(0,Math.min(255,Math.round(this.g)||0))+", "+Math.max(0,Math.min(255,Math.round(this.b)||0))+(1===e?")":", "+e+")")}function hex(e){return((e=Math.max(0,Math.min(255,Math.round(e)||0)))<16?"0":"")+e.toString(16)}function hsla(e,r,t,n){return n<=0?e=r=t=NaN:t<=0||t>=1?e=r=NaN:r<=0&&(e=NaN),new Hsl(e,r,t,n)}function hslConvert(e){if(e instanceof Hsl)return new Hsl(e.h,e.s,e.l,e.opacity);if(e instanceof Color||(e=color(e)),!e)return new Hsl;if(e instanceof Hsl)return e;var r=(e=e.rgb()).r/255,t=e.g/255,n=e.b/255,a=Math.min(r,t,n),i=Math.max(r,t,n),o=NaN,l=i-a,s=(i+a)/2;return l?(o=r===i?(t-n)/l+6*(t<n):t===i?(n-r)/l+2:(r-t)/l+4,l/=s<.5?i+a:2-i-a,o*=60):l=s>0&&s<1?0:o,new Hsl(o,l,s,e.opacity)}function hsl(e,r,t,n){return 1===arguments.length?hslConvert(e):new Hsl(e,r,t,null==n?1:n)}function Hsl(e,r,t,n){this.h=+e,this.s=+r,this.l=+t,this.opacity=+n}function hsl2rgb(e,r,t){return 255*(e<60?r+(t-r)*e/60:e<180?t:e<240?r+(t-r)*(240-e)/60:r)}(0,_define.default)(Color,color,{copy:function(e){return Object.assign(new this.constructor,this,e)},displayable:function(){return this.rgb().displayable()},hex:color_formatHex,formatHex:color_formatHex,formatHsl:color_formatHsl,formatRgb:color_formatRgb,toString:color_formatRgb}),(0,_define.default)(Rgb,rgb,(0,_define.extend)(Color,{brighter:function(e){return e=null==e?brighter:Math.pow(brighter,e),new Rgb(this.r*e,this.g*e,this.b*e,this.opacity)},darker:function(e){return e=null==e?darker:Math.pow(darker,e),new Rgb(this.r*e,this.g*e,this.b*e,this.opacity)},rgb:function(){return this},displayable:function(){return-.5<=this.r&&this.r<255.5&&-.5<=this.g&&this.g<255.5&&-.5<=this.b&&this.b<255.5&&0<=this.opacity&&this.opacity<=1},hex:rgb_formatHex,formatHex:rgb_formatHex,formatRgb:rgb_formatRgb,toString:rgb_formatRgb})),(0,_define.default)(Hsl,hsl,(0,_define.extend)(Color,{brighter:function(e){return e=null==e?brighter:Math.pow(brighter,e),new Hsl(this.h,this.s,this.l*e,this.opacity)},darker:function(e){return e=null==e?darker:Math.pow(darker,e),new Hsl(this.h,this.s,this.l*e,this.opacity)},rgb:function(){var e=this.h%360+360*(this.h<0),r=isNaN(e)||isNaN(this.s)?0:this.s,t=this.l,n=t+(t<.5?t:1-t)*r,a=2*t-n;return new Rgb(hsl2rgb(e>=240?e-240:e+120,a,n),hsl2rgb(e,a,n),hsl2rgb(e<120?e+240:e-120,a,n),this.opacity)},displayable:function(){return(0<=this.s&&this.s<=1||isNaN(this.s))&&0<=this.l&&this.l<=1&&0<=this.opacity&&this.opacity<=1},formatHsl:function(){var e=this.opacity;return(1===(e=isNaN(e)?1:Math.max(0,Math.min(1,e)))?"hsl(":"hsla(")+(this.h||0)+", "+100*(this.s||0)+"%, "+100*(this.l||0)+"%"+(1===e?")":", "+e+")")}}));

},{"./define.js":45}],44:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=cubehelix,exports.Cubehelix=Cubehelix;var _define=_interopRequireWildcard(require("./define.js")),_color=require("./color.js"),_math=require("./math.js");function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var t=_getRequireWildcardCache();if(t&&t.has(e))return t.get(e);var r={},i=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var n in e)if(Object.prototype.hasOwnProperty.call(e,n)){var o=i?Object.getOwnPropertyDescriptor(e,n):null;o&&(o.get||o.set)?Object.defineProperty(r,n,o):r[n]=e[n]}return r.default=e,t&&t.set(e,r),r}var A=-.14861,B=1.78277,C=-.29227,D=-.90649,E=1.97294,ED=E*D,EB=E*B,BC_DA=B*C-D*A;function cubehelixConvert(e){if(e instanceof Cubehelix)return new Cubehelix(e.h,e.s,e.l,e.opacity);e instanceof _color.Rgb||(e=(0,_color.rgbConvert)(e));var t=e.r/255,r=e.g/255,i=e.b/255,n=(BC_DA*i+ED*t-EB*r)/(BC_DA+ED-EB),o=i-n,u=(E*(r-n)-C*o)/D,h=Math.sqrt(u*u+o*o)/(E*n*(1-n)),l=h?Math.atan2(u,o)*_math.rad2deg-120:NaN;return new Cubehelix(l<0?l+360:l,h,n,e.opacity)}function cubehelix(e,t,r,i){return 1===arguments.length?cubehelixConvert(e):new Cubehelix(e,t,r,null==i?1:i)}function Cubehelix(e,t,r,i){this.h=+e,this.s=+t,this.l=+r,this.opacity=+i}(0,_define.default)(Cubehelix,cubehelix,(0,_define.extend)(_color.Color,{brighter:function(e){return e=null==e?_color.brighter:Math.pow(_color.brighter,e),new Cubehelix(this.h,this.s,this.l*e,this.opacity)},darker:function(e){return e=null==e?_color.darker:Math.pow(_color.darker,e),new Cubehelix(this.h,this.s,this.l*e,this.opacity)},rgb:function(){var e=isNaN(this.h)?0:(this.h+120)*_math.deg2rad,t=+this.l,r=isNaN(this.s)?0:this.s*t*(1-t),i=Math.cos(e),n=Math.sin(e);return new _color.Rgb(255*(t+r*(A*i+B*n)),255*(t+r*(C*i+D*n)),255*(t+r*(E*i)),this.opacity)}}));

},{"./color.js":43,"./define.js":45,"./math.js":48}],45:[function(require,module,exports){
"use strict";function _default(e,t,r){e.prototype=t.prototype=r,r.constructor=e}function extend(e,t){var r=Object.create(e.prototype);for(var o in t)r[o]=t[o];return r}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.extend=extend;

},{}],46:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"color",{enumerable:!0,get:function(){return _color.default}}),Object.defineProperty(exports,"rgb",{enumerable:!0,get:function(){return _color.rgb}}),Object.defineProperty(exports,"hsl",{enumerable:!0,get:function(){return _color.hsl}}),Object.defineProperty(exports,"lab",{enumerable:!0,get:function(){return _lab.default}}),Object.defineProperty(exports,"hcl",{enumerable:!0,get:function(){return _lab.hcl}}),Object.defineProperty(exports,"lch",{enumerable:!0,get:function(){return _lab.lch}}),Object.defineProperty(exports,"gray",{enumerable:!0,get:function(){return _lab.gray}}),Object.defineProperty(exports,"cubehelix",{enumerable:!0,get:function(){return _cubehelix.default}});var _color=_interopRequireWildcard(require("./color.js")),_lab=_interopRequireWildcard(require("./lab.js")),_cubehelix=_interopRequireDefault(require("./cubehelix.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var o=n?Object.getOwnPropertyDescriptor(e,u):null;o&&(o.get||o.set)?Object.defineProperty(t,u,o):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}

},{"./color.js":43,"./cubehelix.js":44,"./lab.js":47}],47:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.gray=gray,exports.default=lab,exports.Lab=Lab,exports.lch=lch,exports.hcl=hcl,exports.Hcl=Hcl;var _define=_interopRequireWildcard(require("./define.js")),_color=require("./color.js"),_math=require("./math.js");function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var t=new WeakMap;return _getRequireWildcardCache=function(){return t},t}function _interopRequireWildcard(t){if(t&&t.__esModule)return t;if(null===t||"object"!=typeof t&&"function"!=typeof t)return{default:t};var r=_getRequireWildcardCache();if(r&&r.has(t))return r.get(t);var e={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in t)if(Object.prototype.hasOwnProperty.call(t,i)){var a=n?Object.getOwnPropertyDescriptor(t,i):null;a&&(a.get||a.set)?Object.defineProperty(e,i,a):e[i]=t[i]}return e.default=t,r&&r.set(t,e),e}var K=18,Xn=.96422,Yn=1,Zn=.82521,t0=4/29,t1=6/29,t2=3*t1*t1,t3=t1*t1*t1;function labConvert(t){if(t instanceof Lab)return new Lab(t.l,t.a,t.b,t.opacity);if(t instanceof Hcl)return hcl2lab(t);t instanceof _color.Rgb||(t=(0,_color.rgbConvert)(t));var r,e,n=rgb2lrgb(t.r),i=rgb2lrgb(t.g),a=rgb2lrgb(t.b),l=xyz2lab((.2225045*n+.7168786*i+.0606169*a)/Yn);return n===i&&i===a?r=e=l:(r=xyz2lab((.4360747*n+.3850649*i+.1430804*a)/Xn),e=xyz2lab((.0139322*n+.0971045*i+.7141733*a)/Zn)),new Lab(116*l-16,500*(r-l),200*(l-e),t.opacity)}function gray(t,r){return new Lab(t,0,0,null==r?1:r)}function lab(t,r,e,n){return 1===arguments.length?labConvert(t):new Lab(t,r,e,null==n?1:n)}function Lab(t,r,e,n){this.l=+t,this.a=+r,this.b=+e,this.opacity=+n}function xyz2lab(t){return t>t3?Math.pow(t,1/3):t/t2+t0}function lab2xyz(t){return t>t1?t*t*t:t2*(t-t0)}function lrgb2rgb(t){return 255*(t<=.0031308?12.92*t:1.055*Math.pow(t,1/2.4)-.055)}function rgb2lrgb(t){return(t/=255)<=.04045?t/12.92:Math.pow((t+.055)/1.055,2.4)}function hclConvert(t){if(t instanceof Hcl)return new Hcl(t.h,t.c,t.l,t.opacity);if(t instanceof Lab||(t=labConvert(t)),0===t.a&&0===t.b)return new Hcl(NaN,0<t.l&&t.l<100?0:NaN,t.l,t.opacity);var r=Math.atan2(t.b,t.a)*_math.rad2deg;return new Hcl(r<0?r+360:r,Math.sqrt(t.a*t.a+t.b*t.b),t.l,t.opacity)}function lch(t,r,e,n){return 1===arguments.length?hclConvert(t):new Hcl(e,r,t,null==n?1:n)}function hcl(t,r,e,n){return 1===arguments.length?hclConvert(t):new Hcl(t,r,e,null==n?1:n)}function Hcl(t,r,e,n){this.h=+t,this.c=+r,this.l=+e,this.opacity=+n}function hcl2lab(t){if(isNaN(t.h))return new Lab(t.l,0,0,t.opacity);var r=t.h*_math.deg2rad;return new Lab(t.l,Math.cos(r)*t.c,Math.sin(r)*t.c,t.opacity)}(0,_define.default)(Lab,lab,(0,_define.extend)(_color.Color,{brighter:function(t){return new Lab(this.l+K*(null==t?1:t),this.a,this.b,this.opacity)},darker:function(t){return new Lab(this.l-K*(null==t?1:t),this.a,this.b,this.opacity)},rgb:function(){var t=(this.l+16)/116,r=isNaN(this.a)?t:t+this.a/500,e=isNaN(this.b)?t:t-this.b/200;return r=Xn*lab2xyz(r),t=Yn*lab2xyz(t),e=Zn*lab2xyz(e),new _color.Rgb(lrgb2rgb(3.1338561*r-1.6168667*t-.4906146*e),lrgb2rgb(-.9787684*r+1.9161415*t+.033454*e),lrgb2rgb(.0719453*r-.2289914*t+1.4052427*e),this.opacity)}})),(0,_define.default)(Hcl,hcl,(0,_define.extend)(_color.Color,{brighter:function(t){return new Hcl(this.h,this.c,this.l+K*(null==t?1:t),this.opacity)},darker:function(t){return new Hcl(this.h,this.c,this.l-K*(null==t?1:t),this.opacity)},rgb:function(){return hcl2lab(this).rgb()}}));

},{"./color.js":43,"./define.js":45,"./math.js":48}],48:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.rad2deg=exports.deg2rad=void 0;var deg2rad=Math.PI/180;exports.deg2rad=deg2rad;var rad2deg=180/Math.PI;exports.rad2deg=rad2deg;

},{}],49:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=void 0;var noop={value:function(){}};function dispatch(){for(var t,e=0,n=arguments.length,r={};e<n;++e){if(!(t=arguments[e]+"")||t in r||/[\s.]/.test(t))throw new Error("illegal type: "+t);r[t]=[]}return new Dispatch(r)}function Dispatch(t){this._=t}function parseTypenames(t,e){return t.trim().split(/^|\s+/).map(function(t){var n="",r=t.indexOf(".");if(r>=0&&(n=t.slice(r+1),t=t.slice(0,r)),t&&!e.hasOwnProperty(t))throw new Error("unknown type: "+t);return{type:t,name:n}})}function get(t,e){for(var n,r=0,o=t.length;r<o;++r)if((n=t[r]).name===e)return n.value}function set(t,e,n){for(var r=0,o=t.length;r<o;++r)if(t[r].name===e){t[r]=noop,t=t.slice(0,r).concat(t.slice(r+1));break}return null!=n&&t.push({name:e,value:n}),t}Dispatch.prototype=dispatch.prototype={constructor:Dispatch,on:function(t,e){var n,r=this._,o=parseTypenames(t+"",r),i=-1,a=o.length;if(!(arguments.length<2)){if(null!=e&&"function"!=typeof e)throw new Error("invalid callback: "+e);for(;++i<a;)if(n=(t=o[i]).type)r[n]=set(r[n],t.name,e);else if(null==e)for(n in r)r[n]=set(r[n],t.name,null);return this}for(;++i<a;)if((n=(t=o[i]).type)&&(n=get(r[n],t.name)))return n},copy:function(){var t={},e=this._;for(var n in e)t[n]=e[n].slice();return new Dispatch(t)},call:function(t,e){if((n=arguments.length-2)>0)for(var n,r,o=new Array(n),i=0;i<n;++i)o[i]=arguments[i+2];if(!this._.hasOwnProperty(t))throw new Error("unknown type: "+t);for(i=0,n=(r=this._[t]).length;i<n;++i)r[i].value.apply(e,o)},apply:function(t,e,n){if(!this._.hasOwnProperty(t))throw new Error("unknown type: "+t);for(var r=this._[t],o=0,i=r.length;o<i;++o)r[o].value.apply(e,n)}};var _default=dispatch;exports.default=_default;

},{}],50:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"dispatch",{enumerable:!0,get:function(){return _dispatch.default}});var _dispatch=_interopRequireDefault(require("./dispatch.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./dispatch.js":49}],51:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],52:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Dispatch=require("d3-dispatch"),_d3Selection=require("d3-selection"),_nodrag=_interopRequireWildcard(require("./nodrag.js")),_noevent=_interopRequireWildcard(require("./noevent.js")),_constant=_interopRequireDefault(require("./constant.js")),_event=_interopRequireDefault(require("./event.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var t=_getRequireWildcardCache();if(t&&t.has(e))return t.get(e);var n={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var r in e)if(Object.prototype.hasOwnProperty.call(e,r)){var i=o?Object.getOwnPropertyDescriptor(e,r):null;i&&(i.get||i.set)?Object.defineProperty(n,r,i):n[r]=e[r]}return n.default=e,t&&t.set(e,n),n}function defaultFilter(){return!_d3Selection.event.ctrlKey&&!_d3Selection.event.button}function defaultContainer(){return this.parentNode}function defaultSubject(e){return null==e?{x:_d3Selection.event.x,y:_d3Selection.event.y}:e}function defaultTouchable(){return navigator.maxTouchPoints||"ontouchstart"in this}function _default(){var e,t,n,o,r=defaultFilter,i=defaultContainer,u=defaultSubject,a=defaultTouchable,c={},l=(0,_d3Dispatch.dispatch)("start","drag","end"),d=0,f=0;function s(e){e.on("mousedown.drag",_).filter(a).on("touchstart.drag",h).on("touchmove.drag",g).on("touchend.drag touchcancel.drag",y).style("touch-action","none").style("-webkit-tap-highlight-color","rgba(0,0,0,0)")}function _(){if(!o&&r.apply(this,arguments)){var u=S("mouse",i.apply(this,arguments),_d3Selection.mouse,this,arguments);u&&((0,_d3Selection.select)(_d3Selection.event.view).on("mousemove.drag",p,!0).on("mouseup.drag",v,!0),(0,_nodrag.default)(_d3Selection.event.view),(0,_noevent.nopropagation)(),n=!1,e=_d3Selection.event.clientX,t=_d3Selection.event.clientY,u("start"))}}function p(){if((0,_noevent.default)(),!n){var o=_d3Selection.event.clientX-e,r=_d3Selection.event.clientY-t;n=o*o+r*r>f}c.mouse("drag")}function v(){(0,_d3Selection.select)(_d3Selection.event.view).on("mousemove.drag mouseup.drag",null),(0,_nodrag.yesdrag)(_d3Selection.event.view,n),(0,_noevent.default)(),c.mouse("end")}function h(){if(r.apply(this,arguments)){var e,t,n=_d3Selection.event.changedTouches,o=i.apply(this,arguments),u=n.length;for(e=0;e<u;++e)(t=S(n[e].identifier,o,_d3Selection.touch,this,arguments))&&((0,_noevent.nopropagation)(),t("start"))}}function g(){var e,t,n=_d3Selection.event.changedTouches,o=n.length;for(e=0;e<o;++e)(t=c[n[e].identifier])&&((0,_noevent.default)(),t("drag"))}function y(){var e,t,n=_d3Selection.event.changedTouches,r=n.length;for(o&&clearTimeout(o),o=setTimeout(function(){o=null},500),e=0;e<r;++e)(t=c[n[e].identifier])&&((0,_noevent.nopropagation)(),t("end"))}function S(e,t,n,o,r){var i,a,f,_=n(t,e),p=l.copy();if((0,_d3Selection.customEvent)(new _event.default(s,"beforestart",i,e,d,_[0],_[1],0,0,p),function(){return null!=(_d3Selection.event.subject=i=u.apply(o,r))&&(a=i.x-_[0]||0,f=i.y-_[1]||0,!0)}))return function u(l){var v,h=_;switch(l){case"start":c[e]=u,v=d++;break;case"end":delete c[e],--d;case"drag":_=n(t,e),v=d}(0,_d3Selection.customEvent)(new _event.default(s,l,i,e,v,_[0]+a,_[1]+f,_[0]-h[0],_[1]-h[1],p),p.apply,p,[l,o,r])}}return s.filter=function(e){return arguments.length?(r="function"==typeof e?e:(0,_constant.default)(!!e),s):r},s.container=function(e){return arguments.length?(i="function"==typeof e?e:(0,_constant.default)(e),s):i},s.subject=function(e){return arguments.length?(u="function"==typeof e?e:(0,_constant.default)(e),s):u},s.touchable=function(e){return arguments.length?(a="function"==typeof e?e:(0,_constant.default)(!!e),s):a},s.on=function(){var e=l.on.apply(l,arguments);return e===l?s:e},s.clickDistance=function(e){return arguments.length?(f=(e=+e)*e,s):Math.sqrt(f)},s}

},{"./constant.js":51,"./event.js":53,"./nodrag.js":55,"./noevent.js":56,"d3-dispatch":50,"d3-selection":136}],53:[function(require,module,exports){
"use strict";function DragEvent(t,e,i,s,h,r,n,o,a,p){this.target=t,this.type=e,this.subject=i,this.identifier=s,this.active=h,this.x=r,this.y=n,this.dx=o,this.dy=a,this._=p}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=DragEvent,DragEvent.prototype.on=function(){var t=this._.on.apply(this._,arguments);return t===this._?this:t};

},{}],54:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"drag",{enumerable:!0,get:function(){return _drag.default}}),Object.defineProperty(exports,"dragDisable",{enumerable:!0,get:function(){return _nodrag.default}}),Object.defineProperty(exports,"dragEnable",{enumerable:!0,get:function(){return _nodrag.yesdrag}});var _drag=_interopRequireDefault(require("./drag.js")),_nodrag=_interopRequireWildcard(require("./nodrag.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var a=n?Object.getOwnPropertyDescriptor(e,u):null;a&&(a.get||a.set)?Object.defineProperty(t,u,a):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./drag.js":52,"./nodrag.js":55}],55:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.yesdrag=yesdrag;var _d3Selection=require("d3-selection"),_noevent=_interopRequireDefault(require("./noevent.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){var t=e.document.documentElement,n=(0,_d3Selection.select)(e).on("dragstart.drag",_noevent.default,!0);"onselectstart"in t?n.on("selectstart.drag",_noevent.default,!0):(t.__noselect=t.style.MozUserSelect,t.style.MozUserSelect="none")}function yesdrag(e,t){var n=e.document.documentElement,l=(0,_d3Selection.select)(e).on("dragstart.drag",null);t&&(l.on("click.drag",_noevent.default,!0),setTimeout(function(){l.on("click.drag",null)},0)),"onselectstart"in n?l.on("selectstart.drag",null):(n.style.MozUserSelect=n.__noselect,delete n.__noselect)}

},{"./noevent.js":56,"d3-selection":136}],56:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.nopropagation=nopropagation,exports.default=_default;var _d3Selection=require("d3-selection");function nopropagation(){_d3Selection.event.stopImmediatePropagation()}function _default(){_d3Selection.event.preventDefault(),_d3Selection.event.stopImmediatePropagation()}

},{"d3-selection":136}],57:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.backInOut=exports.backOut=exports.backIn=void 0;var overshoot=1.70158,backIn=function t(o){function r(t){return t*t*((o+1)*t-o)}return o=+o,r.overshoot=t,r}(overshoot);exports.backIn=backIn;var backOut=function t(o){function r(t){return--t*t*((o+1)*t+o)+1}return o=+o,r.overshoot=t,r}(overshoot);exports.backOut=backOut;var backInOut=function t(o){function r(t){return((t*=2)<1?t*t*((o+1)*t-o):(t-=2)*t*((o+1)*t+o)+2)/2}return o=+o,r.overshoot=t,r}(overshoot);exports.backInOut=backInOut;

},{}],58:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.bounceIn=bounceIn,exports.bounceOut=bounceOut,exports.bounceInOut=bounceInOut;var b1=4/11,b2=6/11,b3=8/11,b4=.75,b5=9/11,b6=10/11,b7=.9375,b8=21/22,b9=63/64,b0=1/b1/b1;function bounceIn(b){return 1-bounceOut(1-b)}function bounceOut(b){return(b=+b)<b1?b0*b*b:b<b3?b0*(b-=b2)*b+b4:b<b6?b0*(b-=b5)*b+b7:b0*(b-=b8)*b+b9}function bounceInOut(b){return((b*=2)<=1?1-bounceOut(1-b):bounceOut(b-1)+1)/2}

},{}],59:[function(require,module,exports){
"use strict";function circleIn(t){return 1-Math.sqrt(1-t*t)}function circleOut(t){return Math.sqrt(1- --t*t)}function circleInOut(t){return((t*=2)<=1?1-Math.sqrt(1-t*t):Math.sqrt(1-(t-=2)*t)+1)/2}Object.defineProperty(exports,"__esModule",{value:!0}),exports.circleIn=circleIn,exports.circleOut=circleOut,exports.circleInOut=circleInOut;

},{}],60:[function(require,module,exports){
"use strict";function cubicIn(u){return u*u*u}function cubicOut(u){return--u*u*u+1}function cubicInOut(u){return((u*=2)<=1?u*u*u:(u-=2)*u*u+2)/2}Object.defineProperty(exports,"__esModule",{value:!0}),exports.cubicIn=cubicIn,exports.cubicOut=cubicOut,exports.cubicInOut=cubicInOut;

},{}],61:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.elasticInOut=exports.elasticOut=exports.elasticIn=void 0;var tau=2*Math.PI,amplitude=1,period=.3,elasticIn=function t(a,e){var n=Math.asin(1/(a=Math.max(1,a)))*(e/=tau);function u(t){return a*Math.pow(2,10*--t)*Math.sin((n-t)/e)}return u.amplitude=function(a){return t(a,e*tau)},u.period=function(e){return t(a,e)},u}(amplitude,period);exports.elasticIn=elasticIn;var elasticOut=function t(a,e){var n=Math.asin(1/(a=Math.max(1,a)))*(e/=tau);function u(t){return 1-a*Math.pow(2,-10*(t=+t))*Math.sin((t+n)/e)}return u.amplitude=function(a){return t(a,e*tau)},u.period=function(e){return t(a,e)},u}(amplitude,period);exports.elasticOut=elasticOut;var elasticInOut=function t(a,e){var n=Math.asin(1/(a=Math.max(1,a)))*(e/=tau);function u(t){return((t=2*t-1)<0?a*Math.pow(2,10*t)*Math.sin((n-t)/e):2-a*Math.pow(2,-10*t)*Math.sin((n+t)/e))/2}return u.amplitude=function(a){return t(a,e*tau)},u.period=function(e){return t(a,e)},u}(amplitude,period);exports.elasticInOut=elasticInOut;

},{}],62:[function(require,module,exports){
"use strict";function expIn(e){return Math.pow(2,10*e-10)}function expOut(e){return 1-Math.pow(2,-10*e)}function expInOut(e){return((e*=2)<=1?Math.pow(2,10*e-10):2-Math.pow(2,10-10*e))/2}Object.defineProperty(exports,"__esModule",{value:!0}),exports.expIn=expIn,exports.expOut=expOut,exports.expInOut=expInOut;

},{}],63:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"easeLinear",{enumerable:!0,get:function(){return _linear.linear}}),Object.defineProperty(exports,"easeQuad",{enumerable:!0,get:function(){return _quad.quadInOut}}),Object.defineProperty(exports,"easeQuadIn",{enumerable:!0,get:function(){return _quad.quadIn}}),Object.defineProperty(exports,"easeQuadOut",{enumerable:!0,get:function(){return _quad.quadOut}}),Object.defineProperty(exports,"easeQuadInOut",{enumerable:!0,get:function(){return _quad.quadInOut}}),Object.defineProperty(exports,"easeCubic",{enumerable:!0,get:function(){return _cubic.cubicInOut}}),Object.defineProperty(exports,"easeCubicIn",{enumerable:!0,get:function(){return _cubic.cubicIn}}),Object.defineProperty(exports,"easeCubicOut",{enumerable:!0,get:function(){return _cubic.cubicOut}}),Object.defineProperty(exports,"easeCubicInOut",{enumerable:!0,get:function(){return _cubic.cubicInOut}}),Object.defineProperty(exports,"easePoly",{enumerable:!0,get:function(){return _poly.polyInOut}}),Object.defineProperty(exports,"easePolyIn",{enumerable:!0,get:function(){return _poly.polyIn}}),Object.defineProperty(exports,"easePolyOut",{enumerable:!0,get:function(){return _poly.polyOut}}),Object.defineProperty(exports,"easePolyInOut",{enumerable:!0,get:function(){return _poly.polyInOut}}),Object.defineProperty(exports,"easeSin",{enumerable:!0,get:function(){return _sin.sinInOut}}),Object.defineProperty(exports,"easeSinIn",{enumerable:!0,get:function(){return _sin.sinIn}}),Object.defineProperty(exports,"easeSinOut",{enumerable:!0,get:function(){return _sin.sinOut}}),Object.defineProperty(exports,"easeSinInOut",{enumerable:!0,get:function(){return _sin.sinInOut}}),Object.defineProperty(exports,"easeExp",{enumerable:!0,get:function(){return _exp.expInOut}}),Object.defineProperty(exports,"easeExpIn",{enumerable:!0,get:function(){return _exp.expIn}}),Object.defineProperty(exports,"easeExpOut",{enumerable:!0,get:function(){return _exp.expOut}}),Object.defineProperty(exports,"easeExpInOut",{enumerable:!0,get:function(){return _exp.expInOut}}),Object.defineProperty(exports,"easeCircle",{enumerable:!0,get:function(){return _circle.circleInOut}}),Object.defineProperty(exports,"easeCircleIn",{enumerable:!0,get:function(){return _circle.circleIn}}),Object.defineProperty(exports,"easeCircleOut",{enumerable:!0,get:function(){return _circle.circleOut}}),Object.defineProperty(exports,"easeCircleInOut",{enumerable:!0,get:function(){return _circle.circleInOut}}),Object.defineProperty(exports,"easeBounce",{enumerable:!0,get:function(){return _bounce.bounceOut}}),Object.defineProperty(exports,"easeBounceIn",{enumerable:!0,get:function(){return _bounce.bounceIn}}),Object.defineProperty(exports,"easeBounceOut",{enumerable:!0,get:function(){return _bounce.bounceOut}}),Object.defineProperty(exports,"easeBounceInOut",{enumerable:!0,get:function(){return _bounce.bounceInOut}}),Object.defineProperty(exports,"easeBack",{enumerable:!0,get:function(){return _back.backInOut}}),Object.defineProperty(exports,"easeBackIn",{enumerable:!0,get:function(){return _back.backIn}}),Object.defineProperty(exports,"easeBackOut",{enumerable:!0,get:function(){return _back.backOut}}),Object.defineProperty(exports,"easeBackInOut",{enumerable:!0,get:function(){return _back.backInOut}}),Object.defineProperty(exports,"easeElastic",{enumerable:!0,get:function(){return _elastic.elasticOut}}),Object.defineProperty(exports,"easeElasticIn",{enumerable:!0,get:function(){return _elastic.elasticIn}}),Object.defineProperty(exports,"easeElasticOut",{enumerable:!0,get:function(){return _elastic.elasticOut}}),Object.defineProperty(exports,"easeElasticInOut",{enumerable:!0,get:function(){return _elastic.elasticInOut}});var _linear=require("./linear.js"),_quad=require("./quad.js"),_cubic=require("./cubic.js"),_poly=require("./poly.js"),_sin=require("./sin.js"),_exp=require("./exp.js"),_circle=require("./circle.js"),_bounce=require("./bounce.js"),_back=require("./back.js"),_elastic=require("./elastic.js");

},{"./back.js":57,"./bounce.js":58,"./circle.js":59,"./cubic.js":60,"./elastic.js":61,"./exp.js":62,"./linear.js":64,"./poly.js":65,"./quad.js":66,"./sin.js":67}],64:[function(require,module,exports){
"use strict";function linear(e){return+e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.linear=linear;

},{}],65:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.polyInOut=exports.polyOut=exports.polyIn=void 0;var exponent=3,polyIn=function t(n){function o(t){return Math.pow(t,n)}return n=+n,o.exponent=t,o}(exponent);exports.polyIn=polyIn;var polyOut=function t(n){function o(t){return 1-Math.pow(1-t,n)}return n=+n,o.exponent=t,o}(exponent);exports.polyOut=polyOut;var polyInOut=function t(n){function o(t){return((t*=2)<=1?Math.pow(t,n):2-Math.pow(2-t,n))/2}return n=+n,o.exponent=t,o}(exponent);exports.polyInOut=polyInOut;

},{}],66:[function(require,module,exports){
"use strict";function quadIn(u){return u*u}function quadOut(u){return u*(2-u)}function quadInOut(u){return((u*=2)<=1?u*u:--u*(2-u)+1)/2}Object.defineProperty(exports,"__esModule",{value:!0}),exports.quadIn=quadIn,exports.quadOut=quadOut,exports.quadInOut=quadInOut;

},{}],67:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.sinIn=sinIn,exports.sinOut=sinOut,exports.sinInOut=sinInOut;var pi=Math.PI,halfPi=pi/2;function sinIn(n){return 1-Math.cos(n*halfPi)}function sinOut(n){return Math.sin(n*halfPi)}function sinInOut(n){return(1-Math.cos(pi*n))/2}

},{}],68:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=defaultLocale,exports.formatPrefix=exports.format=void 0;var locale,format,formatPrefix,_locale=_interopRequireDefault(require("./locale.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function defaultLocale(e){return locale=(0,_locale.default)(e),exports.format=format=locale.format,exports.formatPrefix=formatPrefix=locale.formatPrefix,locale}exports.format=format,exports.formatPrefix=formatPrefix,defaultLocale({decimal:".",thousands:",",grouping:[3],currency:["$",""],minus:"-"});

},{"./locale.js":80}],69:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _formatDecimal=_interopRequireDefault(require("./formatDecimal.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return(e=(0,_formatDecimal.default)(Math.abs(e)))?e[1]:NaN}

},{"./formatDecimal.js":70}],70:[function(require,module,exports){
"use strict";function _default(e,t){if((l=(e=t?e.toExponential(t-1):e.toExponential()).indexOf("e"))<0)return null;var l,n=e.slice(0,l);return[n.length>1?n[0]+n.slice(2):n,+e.slice(l+1)]}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],71:[function(require,module,exports){
"use strict";function _default(e,t){return function(r,u){for(var n=r.length,s=[],o=0,a=e[0],f=0;n>0&&a>0&&(f+a+1>u&&(a=Math.max(1,u-f)),s.push(r.substring(n-=a,n+a)),!((f+=a+1)>u));)a=e[o=(o+1)%e.length];return s.reverse().join(t)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],72:[function(require,module,exports){
"use strict";function _default(e){return function(t){return t.replace(/[0-9]/g,function(t){return e[+t]})}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],73:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.prefixExponent=void 0;var prefixExponent,_formatDecimal=_interopRequireDefault(require("./formatDecimal.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){var r=(0,_formatDecimal.default)(e,t);if(!r)return e+"";var a=r[0],n=r[1],o=n-(exports.prefixExponent=prefixExponent=3*Math.max(-8,Math.min(8,Math.floor(n/3))))+1,i=a.length;return o===i?a:o>i?a+new Array(o-i+1).join("0"):o>0?a.slice(0,o)+"."+a.slice(o):"0."+new Array(1-o).join("0")+(0,_formatDecimal.default)(e,Math.max(0,t+o-1))[0]}exports.prefixExponent=prefixExponent;

},{"./formatDecimal.js":70}],74:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _formatDecimal=_interopRequireDefault(require("./formatDecimal.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,r){var t=(0,_formatDecimal.default)(e,r);if(!t)return e+"";var a=t[0],u=t[1];return u<0?"0."+new Array(-u).join("0")+a:a.length>u+1?a.slice(0,u+1)+"."+a.slice(u+1):a+new Array(u-a.length+2).join("0")}

},{"./formatDecimal.js":70}],75:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=formatSpecifier,exports.FormatSpecifier=FormatSpecifier;var re=/^(?:(.)?([<>=^]))?([+\-( ])?([$#])?(0)?(\d+)?(,)?(\.\d+)?(~)?([a-z%])?$/i;function formatSpecifier(i){if(!(t=re.exec(i)))throw new Error("invalid format: "+i);var t;return new FormatSpecifier({fill:t[1],align:t[2],sign:t[3],symbol:t[4],zero:t[5],width:t[6],comma:t[7],precision:t[8]&&t[8].slice(1),trim:t[9],type:t[10]})}function FormatSpecifier(i){this.fill=void 0===i.fill?" ":i.fill+"",this.align=void 0===i.align?">":i.align+"",this.sign=void 0===i.sign?"-":i.sign+"",this.symbol=void 0===i.symbol?"":i.symbol+"",this.zero=!!i.zero,this.width=void 0===i.width?void 0:+i.width,this.comma=!!i.comma,this.precision=void 0===i.precision?void 0:+i.precision,this.trim=!!i.trim,this.type=void 0===i.type?"":i.type+""}formatSpecifier.prototype=FormatSpecifier.prototype,FormatSpecifier.prototype.toString=function(){return this.fill+this.align+this.sign+this.symbol+(this.zero?"0":"")+(void 0===this.width?"":Math.max(1,0|this.width))+(this.comma?",":"")+(void 0===this.precision?"":"."+Math.max(0,0|this.precision))+(this.trim?"~":"")+this.type};

},{}],76:[function(require,module,exports){
"use strict";function _default(e){e:for(var t,r=e.length,a=1,s=-1;a<r;++a)switch(e[a]){case".":s=t=a;break;case"0":0===s&&(s=a),t=a;break;default:if(!+e[a])break e;s>0&&(s=0)}return s>0?e.slice(0,s)+e.slice(t+1):e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],77:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=void 0;var _formatPrefixAuto=_interopRequireDefault(require("./formatPrefixAuto.js")),_formatRounded=_interopRequireDefault(require("./formatRounded.js"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}var _default={"%":function(t,r){return(100*t).toFixed(r)},b:function(t){return Math.round(t).toString(2)},c:function(t){return t+""},d:function(t){return Math.round(t).toString(10)},e:function(t,r){return t.toExponential(r)},f:function(t,r){return t.toFixed(r)},g:function(t,r){return t.toPrecision(r)},o:function(t){return Math.round(t).toString(8)},p:function(t,r){return(0,_formatRounded.default)(100*t,r)},r:_formatRounded.default,s:_formatPrefixAuto.default,X:function(t){return Math.round(t).toString(16).toUpperCase()},x:function(t){return Math.round(t).toString(16)}};exports.default=_default;

},{"./formatPrefixAuto.js":73,"./formatRounded.js":74}],78:[function(require,module,exports){
"use strict";function _default(e){return e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],79:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"formatDefaultLocale",{enumerable:!0,get:function(){return _defaultLocale.default}}),Object.defineProperty(exports,"format",{enumerable:!0,get:function(){return _defaultLocale.format}}),Object.defineProperty(exports,"formatPrefix",{enumerable:!0,get:function(){return _defaultLocale.formatPrefix}}),Object.defineProperty(exports,"formatLocale",{enumerable:!0,get:function(){return _locale.default}}),Object.defineProperty(exports,"formatSpecifier",{enumerable:!0,get:function(){return _formatSpecifier.default}}),Object.defineProperty(exports,"FormatSpecifier",{enumerable:!0,get:function(){return _formatSpecifier.FormatSpecifier}}),Object.defineProperty(exports,"precisionFixed",{enumerable:!0,get:function(){return _precisionFixed.default}}),Object.defineProperty(exports,"precisionPrefix",{enumerable:!0,get:function(){return _precisionPrefix.default}}),Object.defineProperty(exports,"precisionRound",{enumerable:!0,get:function(){return _precisionRound.default}});var _defaultLocale=_interopRequireWildcard(require("./defaultLocale.js")),_locale=_interopRequireDefault(require("./locale.js")),_formatSpecifier=_interopRequireWildcard(require("./formatSpecifier.js")),_precisionFixed=_interopRequireDefault(require("./precisionFixed.js")),_precisionPrefix=_interopRequireDefault(require("./precisionPrefix.js")),_precisionRound=_interopRequireDefault(require("./precisionRound.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},i=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var n in e)if(Object.prototype.hasOwnProperty.call(e,n)){var o=i?Object.getOwnPropertyDescriptor(e,n):null;o&&(o.get||o.set)?Object.defineProperty(t,n,o):t[n]=e[n]}return t.default=e,r&&r.set(e,t),t}

},{"./defaultLocale.js":68,"./formatSpecifier.js":75,"./locale.js":80,"./precisionFixed.js":81,"./precisionPrefix.js":82,"./precisionRound.js":83}],80:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _exponent=_interopRequireDefault(require("./exponent.js")),_formatGroup=_interopRequireDefault(require("./formatGroup.js")),_formatNumerals=_interopRequireDefault(require("./formatNumerals.js")),_formatSpecifier=_interopRequireDefault(require("./formatSpecifier.js")),_formatTrim=_interopRequireDefault(require("./formatTrim.js")),_formatTypes=_interopRequireDefault(require("./formatTypes.js")),_formatPrefixAuto=require("./formatPrefixAuto.js"),_identity=_interopRequireDefault(require("./identity.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var map=Array.prototype.map,prefixes=["y","z","a","f","p","n","µ","m","","k","M","G","T","P","E","Z","Y"];function _default(e){var r=void 0===e.grouping||void 0===e.thousands?_identity.default:(0,_formatGroup.default)(map.call(e.grouping,Number),e.thousands+""),t=void 0===e.currency?"":e.currency[0]+"",a=void 0===e.currency?"":e.currency[1]+"",i=void 0===e.decimal?".":e.decimal+"",o=void 0===e.numerals?_identity.default:(0,_formatNumerals.default)(map.call(e.numerals,String)),n=void 0===e.percent?"%":e.percent+"",u=void 0===e.minus?"-":e.minus+"",f=void 0===e.nan?"NaN":e.nan+"";function l(e){var l=(e=(0,_formatSpecifier.default)(e)).fill,s=e.align,m=e.sign,p=e.symbol,c=e.zero,d=e.width,_=e.comma,h=e.precision,v=e.trim,y=e.type;"n"===y?(_=!0,y="g"):_formatTypes.default[y]||(void 0===h&&(h=12),v=!0,y="g"),(c||"0"===l&&"="===s)&&(c=!0,l="0",s="=");var g="$"===p?t:"#"===p&&/[boxX]/.test(y)?"0"+y.toLowerCase():"",x="$"===p?a:/[%p]/.test(y)?n:"",q=_formatTypes.default[y],M=/[defgprs%]/.test(y);function j(e){var t,a,n,p=g,j=x;if("c"===y)j=q(e)+j,e="";else{var b=(e=+e)<0||1/e<0;if(e=isNaN(e)?f:q(Math.abs(e),h),v&&(e=(0,_formatTrim.default)(e)),b&&0==+e&&"+"!==m&&(b=!1),p=(b?"("===m?m:u:"-"===m||"("===m?"":m)+p,j=("s"===y?prefixes[8+_formatPrefixAuto.prefixExponent/3]:"")+j+(b&&"("===m?")":""),M)for(t=-1,a=e.length;++t<a;)if(48>(n=e.charCodeAt(t))||n>57){j=(46===n?i+e.slice(t+1):e.slice(t))+j,e=e.slice(0,t);break}}_&&!c&&(e=r(e,1/0));var D=p.length+e.length+j.length,N=D<d?new Array(d-D+1).join(l):"";switch(_&&c&&(e=r(N+e,N.length?d-j.length:1/0),N=""),s){case"<":e=p+e+j+N;break;case"=":e=p+N+e+j;break;case"^":e=N.slice(0,D=N.length>>1)+p+e+j+N.slice(D);break;default:e=N+p+e+j}return o(e)}return h=void 0===h?6:/[gprs]/.test(y)?Math.max(1,Math.min(21,h)):Math.max(0,Math.min(20,h)),j.toString=function(){return e+""},j}return{format:l,formatPrefix:function(e,r){var t=l(((e=(0,_formatSpecifier.default)(e)).type="f",e)),a=3*Math.max(-8,Math.min(8,Math.floor((0,_exponent.default)(r)/3))),i=Math.pow(10,-a),o=prefixes[8+a/3];return function(e){return t(i*e)+o}}}}

},{"./exponent.js":69,"./formatGroup.js":71,"./formatNumerals.js":72,"./formatPrefixAuto.js":73,"./formatSpecifier.js":75,"./formatTrim.js":76,"./formatTypes.js":77,"./identity.js":78}],81:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _exponent=_interopRequireDefault(require("./exponent.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return Math.max(0,-(0,_exponent.default)(Math.abs(e)))}

},{"./exponent.js":69}],82:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _exponent=_interopRequireDefault(require("./exponent.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){return Math.max(0,3*Math.max(-8,Math.min(8,Math.floor((0,_exponent.default)(t)/3)))-(0,_exponent.default)(Math.abs(e)))}

},{"./exponent.js":69}],83:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _exponent=_interopRequireDefault(require("./exponent.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){return e=Math.abs(e),t=Math.abs(t)-e,Math.max(0,(0,_exponent.default)(t)-(0,_exponent.default)(e))+1}

},{"./exponent.js":69}],84:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.genericArray=genericArray;var _value=_interopRequireDefault(require("./value.js")),_numberArray=_interopRequireWildcard(require("./numberArray.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var a=n?Object.getOwnPropertyDescriptor(e,u):null;a&&(a.get||a.set)?Object.defineProperty(t,u,a):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,r){return((0,_numberArray.isNumberArray)(r)?_numberArray.default:genericArray)(e,r)}function genericArray(e,r){var t,n=r?r.length:0,u=e?Math.min(n,e.length):0,a=new Array(u),i=new Array(n);for(t=0;t<u;++t)a[t]=(0,_value.default)(e[t],r[t]);for(;t<n;++t)i[t]=r[t];return function(e){for(t=0;t<u;++t)i[t]=a[t](e);return i}}

},{"./numberArray.js":98,"./value.js":108}],85:[function(require,module,exports){
"use strict";function basis(e,t,r,s,a){var u=e*e,n=u*e;return((1-3*e+3*u-n)*t+(4-6*u+3*n)*r+(1+3*e+3*u-3*n)*s+n*a)/6}function _default(e){var t=e.length-1;return function(r){var s=r<=0?r=0:r>=1?(r=1,t-1):Math.floor(r*t),a=e[s],u=e[s+1],n=s>0?e[s-1]:2*a-u,o=s<t-1?e[s+2]:2*u-a;return basis((r-s/t)*t,n,a,u,o)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.basis=basis,exports.default=_default;

},{}],86:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _basis=require("./basis.js");function _default(e){var r=e.length;return function(t){var s=Math.floor(((t%=1)<0?++t:t)*r),a=e[(s+r-1)%r],u=e[s%r],i=e[(s+1)%r],n=e[(s+2)%r];return(0,_basis.basis)((t-s/r)*r,a,u,i,n)}}

},{"./basis.js":85}],87:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.hue=hue,exports.gamma=gamma,exports.default=nogamma;var _constant=_interopRequireDefault(require("./constant.js"));function _interopRequireDefault(n){return n&&n.__esModule?n:{default:n}}function linear(n,t){return function(e){return n+e*t}}function exponential(n,t,e){return n=Math.pow(n,e),t=Math.pow(t,e)-n,e=1/e,function(a){return Math.pow(n+a*t,e)}}function hue(n,t){var e=t-n;return e?linear(n,e>180||e<-180?e-360*Math.round(e/360):e):(0,_constant.default)(isNaN(n)?t:n)}function gamma(n){return 1==(n=+n)?nogamma:function(t,e){return e-t?exponential(t,e,n):(0,_constant.default)(isNaN(t)?e:t)}}function nogamma(n,t){var e=t-n;return e?linear(n,e):(0,_constant.default)(isNaN(n)?t:n)}

},{"./constant.js":88}],88:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],89:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.cubehelixLong=exports.default=void 0;var _d3Color=require("d3-color"),_color=_interopRequireWildcard(require("./color.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var l=o?Object.getOwnPropertyDescriptor(e,u):null;l&&(l.get||l.set)?Object.defineProperty(t,u,l):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}function cubehelix(e){return function r(t){function o(r,o){var u=e((r=(0,_d3Color.cubehelix)(r)).h,(o=(0,_d3Color.cubehelix)(o)).h),l=(0,_color.default)(r.s,o.s),c=(0,_color.default)(r.l,o.l),i=(0,_color.default)(r.opacity,o.opacity);return function(e){return r.h=u(e),r.s=l(e),r.l=c(Math.pow(e,t)),r.opacity=i(e),r+""}}return t=+t,o.gamma=r,o}(1)}var _default=cubehelix(_color.hue);exports.default=_default;var cubehelixLong=cubehelix(_color.default);exports.cubehelixLong=cubehelixLong;

},{"./color.js":87,"d3-color":46}],90:[function(require,module,exports){
"use strict";function _default(e,t){var r=new Date;return e=+e,t=+t,function(u){return r.setTime(e*(1-u)+t*u),r}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],91:[function(require,module,exports){
"use strict";function _default(t){var e=t.length;return function(r){return t[Math.max(0,Math.min(e-1,Math.floor(r*e)))]}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],92:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.hclLong=exports.default=void 0;var _d3Color=require("d3-color"),_color=_interopRequireWildcard(require("./color.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var c in e)if(Object.prototype.hasOwnProperty.call(e,c)){var l=o?Object.getOwnPropertyDescriptor(e,c):null;l&&(l.get||l.set)?Object.defineProperty(t,c,l):t[c]=e[c]}return t.default=e,r&&r.set(e,t),t}function hcl(e){return function(r,t){var o=e((r=(0,_d3Color.hcl)(r)).h,(t=(0,_d3Color.hcl)(t)).h),c=(0,_color.default)(r.c,t.c),l=(0,_color.default)(r.l,t.l),n=(0,_color.default)(r.opacity,t.opacity);return function(e){return r.h=o(e),r.c=c(e),r.l=l(e),r.opacity=n(e),r+""}}}var _default=hcl(_color.hue);exports.default=_default;var hclLong=hcl(_color.default);exports.hclLong=hclLong;

},{"./color.js":87,"d3-color":46}],93:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.hslLong=exports.default=void 0;var _d3Color=require("d3-color"),_color=_interopRequireWildcard(require("./color.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var l in e)if(Object.prototype.hasOwnProperty.call(e,l)){var n=o?Object.getOwnPropertyDescriptor(e,l):null;n&&(n.get||n.set)?Object.defineProperty(t,l,n):t[l]=e[l]}return t.default=e,r&&r.set(e,t),t}function hsl(e){return function(r,t){var o=e((r=(0,_d3Color.hsl)(r)).h,(t=(0,_d3Color.hsl)(t)).h),l=(0,_color.default)(r.s,t.s),n=(0,_color.default)(r.l,t.l),u=(0,_color.default)(r.opacity,t.opacity);return function(e){return r.h=o(e),r.s=l(e),r.l=n(e),r.opacity=u(e),r+""}}}var _default=hsl(_color.hue);exports.default=_default;var hslLong=hsl(_color.default);exports.hslLong=hslLong;

},{"./color.js":87,"d3-color":46}],94:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _color=require("./color.js");function _default(e,r){var o=(0,_color.hue)(+e,+r);return function(e){var r=o(e);return r-360*Math.floor(r/360)}}

},{"./color.js":87}],95:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"interpolate",{enumerable:!0,get:function(){return _value.default}}),Object.defineProperty(exports,"interpolateArray",{enumerable:!0,get:function(){return _array.default}}),Object.defineProperty(exports,"interpolateBasis",{enumerable:!0,get:function(){return _basis.default}}),Object.defineProperty(exports,"interpolateBasisClosed",{enumerable:!0,get:function(){return _basisClosed.default}}),Object.defineProperty(exports,"interpolateDate",{enumerable:!0,get:function(){return _date.default}}),Object.defineProperty(exports,"interpolateDiscrete",{enumerable:!0,get:function(){return _discrete.default}}),Object.defineProperty(exports,"interpolateHue",{enumerable:!0,get:function(){return _hue.default}}),Object.defineProperty(exports,"interpolateNumber",{enumerable:!0,get:function(){return _number.default}}),Object.defineProperty(exports,"interpolateNumberArray",{enumerable:!0,get:function(){return _numberArray.default}}),Object.defineProperty(exports,"interpolateObject",{enumerable:!0,get:function(){return _object.default}}),Object.defineProperty(exports,"interpolateRound",{enumerable:!0,get:function(){return _round.default}}),Object.defineProperty(exports,"interpolateString",{enumerable:!0,get:function(){return _string.default}}),Object.defineProperty(exports,"interpolateTransformCss",{enumerable:!0,get:function(){return _index.interpolateTransformCss}}),Object.defineProperty(exports,"interpolateTransformSvg",{enumerable:!0,get:function(){return _index.interpolateTransformSvg}}),Object.defineProperty(exports,"interpolateZoom",{enumerable:!0,get:function(){return _zoom.default}}),Object.defineProperty(exports,"interpolateRgb",{enumerable:!0,get:function(){return _rgb.default}}),Object.defineProperty(exports,"interpolateRgbBasis",{enumerable:!0,get:function(){return _rgb.rgbBasis}}),Object.defineProperty(exports,"interpolateRgbBasisClosed",{enumerable:!0,get:function(){return _rgb.rgbBasisClosed}}),Object.defineProperty(exports,"interpolateHsl",{enumerable:!0,get:function(){return _hsl.default}}),Object.defineProperty(exports,"interpolateHslLong",{enumerable:!0,get:function(){return _hsl.hslLong}}),Object.defineProperty(exports,"interpolateLab",{enumerable:!0,get:function(){return _lab.default}}),Object.defineProperty(exports,"interpolateHcl",{enumerable:!0,get:function(){return _hcl.default}}),Object.defineProperty(exports,"interpolateHclLong",{enumerable:!0,get:function(){return _hcl.hclLong}}),Object.defineProperty(exports,"interpolateCubehelix",{enumerable:!0,get:function(){return _cubehelix.default}}),Object.defineProperty(exports,"interpolateCubehelixLong",{enumerable:!0,get:function(){return _cubehelix.cubehelixLong}}),Object.defineProperty(exports,"piecewise",{enumerable:!0,get:function(){return _piecewise.default}}),Object.defineProperty(exports,"quantize",{enumerable:!0,get:function(){return _quantize.default}});var _value=_interopRequireDefault(require("./value.js")),_array=_interopRequireDefault(require("./array.js")),_basis=_interopRequireDefault(require("./basis.js")),_basisClosed=_interopRequireDefault(require("./basisClosed.js")),_date=_interopRequireDefault(require("./date.js")),_discrete=_interopRequireDefault(require("./discrete.js")),_hue=_interopRequireDefault(require("./hue.js")),_number=_interopRequireDefault(require("./number.js")),_numberArray=_interopRequireDefault(require("./numberArray.js")),_object=_interopRequireDefault(require("./object.js")),_round=_interopRequireDefault(require("./round.js")),_string=_interopRequireDefault(require("./string.js")),_index=require("./transform/index.js"),_zoom=_interopRequireDefault(require("./zoom.js")),_rgb=_interopRequireWildcard(require("./rgb.js")),_hsl=_interopRequireWildcard(require("./hsl.js")),_lab=_interopRequireDefault(require("./lab.js")),_hcl=_interopRequireWildcard(require("./hcl.js")),_cubehelix=_interopRequireWildcard(require("./cubehelix.js")),_piecewise=_interopRequireDefault(require("./piecewise.js")),_quantize=_interopRequireDefault(require("./quantize.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var i=n?Object.getOwnPropertyDescriptor(e,u):null;i&&(i.get||i.set)?Object.defineProperty(t,u,i):t[u]=e[u]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./array.js":84,"./basis.js":85,"./basisClosed.js":86,"./cubehelix.js":89,"./date.js":90,"./discrete.js":91,"./hcl.js":92,"./hsl.js":93,"./hue.js":94,"./lab.js":96,"./number.js":97,"./numberArray.js":98,"./object.js":99,"./piecewise.js":100,"./quantize.js":101,"./rgb.js":102,"./round.js":103,"./string.js":104,"./transform/index.js":106,"./value.js":108,"./zoom.js":109}],96:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=lab;var _d3Color=require("d3-color"),_color=_interopRequireDefault(require("./color.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function lab(e,o){var r=(0,_color.default)((e=(0,_d3Color.lab)(e)).l,(o=(0,_d3Color.lab)(o)).l),l=(0,_color.default)(e.a,o.a),t=(0,_color.default)(e.b,o.b),u=(0,_color.default)(e.opacity,o.opacity);return function(o){return e.l=r(o),e.a=l(o),e.b=t(o),e.opacity=u(o),e+""}}

},{"./color.js":87,"d3-color":46}],97:[function(require,module,exports){
"use strict";function _default(e,t){return e=+e,t=+t,function(u){return e*(1-u)+t*u}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],98:[function(require,module,exports){
"use strict";function _default(e,r){r||(r=[]);var t,u=e?Math.min(r.length,e.length):0,n=r.slice();return function(i){for(t=0;t<u;++t)n[t]=e[t]*(1-i)+r[t]*i;return n}}function isNumberArray(e){return ArrayBuffer.isView(e)&&!(e instanceof DataView)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.isNumberArray=isNumberArray;

},{}],99:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _value=_interopRequireDefault(require("./value.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){var u,r={},l={};for(u in null!==e&&"object"==typeof e||(e={}),null!==t&&"object"==typeof t||(t={}),t)u in e?r[u]=(0,_value.default)(e[u],t[u]):l[u]=t[u];return function(e){for(u in r)l[u]=r[u](e);return l}}

},{"./value.js":108}],100:[function(require,module,exports){
"use strict";function piecewise(e,r){for(var t=0,n=r.length-1,a=r[0],i=new Array(n<0?0:n);t<n;)i[t]=e(a,a=r[++t]);return function(e){var r=Math.max(0,Math.min(n-1,Math.floor(e*=n)));return i[r](e-r)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=piecewise;

},{}],101:[function(require,module,exports){
"use strict";function _default(e,r){for(var t=new Array(r),u=0;u<r;++u)t[u]=e(u/(r-1));return t}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],102:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.rgbBasisClosed=exports.rgbBasis=exports.default=void 0;var _d3Color=require("d3-color"),_basis=_interopRequireDefault(require("./basis.js")),_basisClosed=_interopRequireDefault(require("./basisClosed.js")),_color=_interopRequireWildcard(require("./color.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var r=new WeakMap;return _getRequireWildcardCache=function(){return r},r}function _interopRequireWildcard(r){if(r&&r.__esModule)return r;if(null===r||"object"!=typeof r&&"function"!=typeof r)return{default:r};var e=_getRequireWildcardCache();if(e&&e.has(r))return e.get(r);var t={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in r)if(Object.prototype.hasOwnProperty.call(r,i)){var a=o?Object.getOwnPropertyDescriptor(r,i):null;a&&(a.get||a.set)?Object.defineProperty(t,i,a):t[i]=r[i]}return t.default=r,e&&e.set(r,t),t}function _interopRequireDefault(r){return r&&r.__esModule?r:{default:r}}var _default=function r(e){var t=(0,_color.gamma)(e);function o(r,e){var o=t((r=(0,_d3Color.rgb)(r)).r,(e=(0,_d3Color.rgb)(e)).r),i=t(r.g,e.g),a=t(r.b,e.b),n=(0,_color.default)(r.opacity,e.opacity);return function(e){return r.r=o(e),r.g=i(e),r.b=a(e),r.opacity=n(e),r+""}}return o.gamma=r,o}(1);function rgbSpline(r){return function(e){var t,o,i=e.length,a=new Array(i),n=new Array(i),u=new Array(i);for(t=0;t<i;++t)o=(0,_d3Color.rgb)(e[t]),a[t]=o.r||0,n[t]=o.g||0,u[t]=o.b||0;return a=r(a),n=r(n),u=r(u),o.opacity=1,function(r){return o.r=a(r),o.g=n(r),o.b=u(r),o+""}}}exports.default=_default;var rgbBasis=rgbSpline(_basis.default);exports.rgbBasis=rgbBasis;var rgbBasisClosed=rgbSpline(_basisClosed.default);exports.rgbBasisClosed=rgbBasisClosed;

},{"./basis.js":85,"./basisClosed.js":86,"./color.js":87,"d3-color":46}],103:[function(require,module,exports){
"use strict";function _default(e,t){return e=+e,t=+t,function(u){return Math.round(e*(1-u)+t*u)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],104:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _number=_interopRequireDefault(require("./number.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var reA=/[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?/g,reB=new RegExp(reA.source,"g");function zero(e){return function(){return e}}function one(e){return function(r){return e(r)+""}}function _default(e,r){var n,t,u,o=reA.lastIndex=reB.lastIndex=0,i=-1,l=[],f=[];for(e+="",r+="";(n=reA.exec(e))&&(t=reB.exec(r));)(u=t.index)>o&&(u=r.slice(o,u),l[i]?l[i]+=u:l[++i]=u),(n=n[0])===(t=t[0])?l[i]?l[i]+=t:l[++i]=t:(l[++i]=null,f.push({i:i,x:(0,_number.default)(n,t)})),o=reB.lastIndex;return o<r.length&&(u=r.slice(o),l[i]?l[i]+=u:l[++i]=u),l.length<2?f[0]?one(f[0].x):zero(r):(r=f.length,function(e){for(var n,t=0;t<r;++t)l[(n=f[t]).i]=n.x(e);return l.join("")})}

},{"./number.js":97}],105:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.identity=void 0;var degrees=180/Math.PI,identity={translateX:0,translateY:0,rotate:0,skewX:0,scaleX:1,scaleY:1};function _default(t,e,a,r,s,n){var d,l,i;return(d=Math.sqrt(t*t+e*e))&&(t/=d,e/=d),(i=t*a+e*r)&&(a-=t*i,r-=e*i),(l=Math.sqrt(a*a+r*r))&&(a/=l,r/=l,i/=l),t*r<e*a&&(t=-t,e=-e,i=-i,d=-d),{translateX:s,translateY:n,rotate:Math.atan2(e,t)*degrees,skewX:Math.atan(i)*degrees,scaleX:d,scaleY:l}}exports.identity=identity;

},{}],106:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.interpolateTransformSvg=exports.interpolateTransformCss=void 0;var _number=_interopRequireDefault(require("../number.js")),_parse=require("./parse.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function interpolateTransform(e,r,t,n){function s(e){return e.length?e.pop()+" ":""}return function(a,u){var l=[],o=[];return a=e(a),u=e(u),function(e,n,s,a,u,l){if(e!==s||n!==a){var o=u.push("translate(",null,r,null,t);l.push({i:o-4,x:(0,_number.default)(e,s)},{i:o-2,x:(0,_number.default)(n,a)})}else(s||a)&&u.push("translate("+s+r+a+t)}(a.translateX,a.translateY,u.translateX,u.translateY,l,o),function(e,r,t,a){e!==r?(e-r>180?r+=360:r-e>180&&(e+=360),a.push({i:t.push(s(t)+"rotate(",null,n)-2,x:(0,_number.default)(e,r)})):r&&t.push(s(t)+"rotate("+r+n)}(a.rotate,u.rotate,l,o),function(e,r,t,a){e!==r?a.push({i:t.push(s(t)+"skewX(",null,n)-2,x:(0,_number.default)(e,r)}):r&&t.push(s(t)+"skewX("+r+n)}(a.skewX,u.skewX,l,o),function(e,r,t,n,a,u){if(e!==t||r!==n){var l=a.push(s(a)+"scale(",null,",",null,")");u.push({i:l-4,x:(0,_number.default)(e,t)},{i:l-2,x:(0,_number.default)(r,n)})}else 1===t&&1===n||a.push(s(a)+"scale("+t+","+n+")")}(a.scaleX,a.scaleY,u.scaleX,u.scaleY,l,o),a=u=null,function(e){for(var r,t=-1,n=o.length;++t<n;)l[(r=o[t]).i]=r.x(e);return l.join("")}}}var interpolateTransformCss=interpolateTransform(_parse.parseCss,"px, ","px)","deg)");exports.interpolateTransformCss=interpolateTransformCss;var interpolateTransformSvg=interpolateTransform(_parse.parseSvg,", ",")",")");exports.interpolateTransformSvg=interpolateTransformSvg;

},{"../number.js":97,"./parse.js":107}],107:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.parseCss=parseCss,exports.parseSvg=parseSvg;var cssNode,cssRoot,cssView,svgNode,_decompose=_interopRequireWildcard(require("./decompose.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var t=_getRequireWildcardCache();if(t&&t.has(e))return t.get(e);var r={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var s in e)if(Object.prototype.hasOwnProperty.call(e,s)){var n=o?Object.getOwnPropertyDescriptor(e,s):null;n&&(n.get||n.set)?Object.defineProperty(r,s,n):r[s]=e[s]}return r.default=e,t&&t.set(e,r),r}function parseCss(e){return"none"===e?_decompose.identity:(cssNode||(cssNode=document.createElement("DIV"),cssRoot=document.documentElement,cssView=document.defaultView),cssNode.style.transform=e,e=cssView.getComputedStyle(cssRoot.appendChild(cssNode),null).getPropertyValue("transform"),cssRoot.removeChild(cssNode),e=e.slice(7,-1).split(","),(0,_decompose.default)(+e[0],+e[1],+e[2],+e[3],+e[4],+e[5]))}function parseSvg(e){return null==e?_decompose.identity:(svgNode||(svgNode=document.createElementNS("http://www.w3.org/2000/svg","g")),svgNode.setAttribute("transform",e),(e=svgNode.transform.baseVal.consolidate())?(e=e.matrix,(0,_decompose.default)(e.a,e.b,e.c,e.d,e.e,e.f)):_decompose.identity)}

},{"./decompose.js":105}],108:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Color=require("d3-color"),_rgb=_interopRequireDefault(require("./rgb.js")),_array=require("./array.js"),_date=_interopRequireDefault(require("./date.js")),_number=_interopRequireDefault(require("./number.js")),_object=_interopRequireDefault(require("./object.js")),_string=_interopRequireDefault(require("./string.js")),_constant=_interopRequireDefault(require("./constant.js")),_numberArray=_interopRequireWildcard(require("./numberArray.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},u=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var n in e)if(Object.prototype.hasOwnProperty.call(e,n)){var a=u?Object.getOwnPropertyDescriptor(e,n):null;a&&(a.get||a.set)?Object.defineProperty(t,n,a):t[n]=e[n]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,r){var t,u=typeof r;return null==r||"boolean"===u?(0,_constant.default)(r):("number"===u?_number.default:"string"===u?(t=(0,_d3Color.color)(r))?(r=t,_rgb.default):_string.default:r instanceof _d3Color.color?_rgb.default:r instanceof Date?_date.default:(0,_numberArray.isNumberArray)(r)?_numberArray.default:Array.isArray(r)?_array.genericArray:"function"!=typeof r.valueOf&&"function"!=typeof r.toString||isNaN(r)?_object.default:_number.default)(e,r)}

},{"./array.js":84,"./constant.js":88,"./date.js":90,"./number.js":97,"./numberArray.js":98,"./object.js":99,"./rgb.js":102,"./string.js":104,"d3-color":46}],109:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var rho=Math.SQRT2,rho2=2,rho4=4,epsilon2=1e-12;function cosh(r){return((r=Math.exp(r))+1/r)/2}function sinh(r){return((r=Math.exp(r))-1/r)/2}function tanh(r){return((r=Math.exp(2*r))-1)/(r+1)}function _default(r,t){var o,h,e=r[0],n=r[1],a=r[2],u=t[0],s=t[1],i=t[2],M=u-e,c=s-n,f=M*M+c*c;if(f<epsilon2)h=Math.log(i/a)/rho,o=function(r){return[e+r*M,n+r*c,a*Math.exp(rho*r*h)]};else{var l=Math.sqrt(f),p=(i*i-a*a+rho4*f)/(2*a*rho2*l),d=(i*i-a*a-rho4*f)/(2*i*rho2*l),x=Math.log(Math.sqrt(p*p+1)-p),v=Math.log(Math.sqrt(d*d+1)-d);h=(v-x)/rho,o=function(r){var t=r*h,o=cosh(x),u=a/(rho2*l)*(o*tanh(rho*t+x)-sinh(x));return[e+u*M,n+u*c,a*o/cosh(rho*t+x)]}}return o.duration=1e3*h,o}

},{}],110:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=band,exports.point=point;var _d3Array=require("d3-array"),_init=require("./init.js"),_ordinal=_interopRequireDefault(require("./ordinal.js"));function _interopRequireDefault(n){return n&&n.__esModule?n:{default:n}}function band(){var n,r,t=(0,_ordinal.default)().unknown(void 0),e=t.domain,i=t.range,u=0,a=1,o=!1,d=0,p=0,l=.5;function f(){var t=e().length,f=a<u,g=f?a:u,c=f?u:a;n=(c-g)/Math.max(1,t-d+2*p),o&&(n=Math.floor(n)),g+=(c-g-n*(t-d))*l,r=n*(1-d),o&&(g=Math.round(g),r=Math.round(r));var h=(0,_d3Array.range)(t).map(function(r){return g+n*r});return i(f?h.reverse():h)}return delete t.unknown,t.domain=function(n){return arguments.length?(e(n),f()):e()},t.range=function(n){return arguments.length?([u,a]=n,u=+u,a=+a,f()):[u,a]},t.rangeRound=function(n){return[u,a]=n,u=+u,a=+a,o=!0,f()},t.bandwidth=function(){return r},t.step=function(){return n},t.round=function(n){return arguments.length?(o=!!n,f()):o},t.padding=function(n){return arguments.length?(d=Math.min(1,p=+n),f()):d},t.paddingInner=function(n){return arguments.length?(d=Math.min(1,n),f()):d},t.paddingOuter=function(n){return arguments.length?(p=+n,f()):p},t.align=function(n){return arguments.length?(l=Math.max(0,Math.min(1,n)),f()):l},t.copy=function(){return band(e(),[u,a]).round(o).paddingInner(d).paddingOuter(p).align(l)},_init.initRange.apply(f(),arguments)}function pointish(n){var r=n.copy;return n.padding=n.paddingOuter,delete n.paddingInner,delete n.paddingOuter,n.copy=function(){return pointish(r())},n}function point(){return pointish(band.apply(null,arguments).paddingInner(1))}

},{"./init.js":116,"./ordinal.js":121,"d3-array":17}],111:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],112:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.identity=identity,exports.copy=copy,exports.transformer=transformer,exports.default=continuous;var _d3Array=require("d3-array"),_d3Interpolate=require("d3-interpolate"),_constant=_interopRequireDefault(require("./constant.js")),_number=_interopRequireDefault(require("./number.js"));function _interopRequireDefault(n){return n&&n.__esModule?n:{default:n}}var unit=[0,1];function identity(n){return n}function normalize(n,r){return(r-=n=+n)?function(t){return(t-n)/r}:(0,_constant.default)(isNaN(r)?NaN:.5)}function clamper(n,r){var t;return n>r&&(t=n,n=r,r=t),function(t){return Math.max(n,Math.min(r,t))}}function bimap(n,r,t){var e=n[0],i=n[1],u=r[0],o=r[1];return i<e?(e=normalize(i,e),u=t(o,u)):(e=normalize(e,i),u=t(u,o)),function(n){return u(e(n))}}function polymap(n,r,t){var e=Math.min(n.length,r.length)-1,i=new Array(e),u=new Array(e),o=-1;for(n[e]<n[0]&&(n=n.slice().reverse(),r=r.slice().reverse());++o<e;)i[o]=normalize(n[o],n[o+1]),u[o]=t(r[o],r[o+1]);return function(r){var t=(0,_d3Array.bisect)(n,r,1,e)-1;return u[t](i[t](r))}}function copy(n,r){return r.domain(n.domain()).range(n.range()).interpolate(n.interpolate()).clamp(n.clamp()).unknown(n.unknown())}function transformer(){var n,r,t,e,i,u,o=unit,a=unit,l=_d3Interpolate.interpolate,c=identity;function f(){var n=Math.min(o.length,a.length);return c!==identity&&(c=clamper(o[0],o[n-1])),e=n>2?polymap:bimap,i=u=null,p}function p(r){return isNaN(r=+r)?t:(i||(i=e(o.map(n),a,l)))(n(c(r)))}return p.invert=function(t){return c(r((u||(u=e(a,o.map(n),_d3Interpolate.interpolateNumber)))(t)))},p.domain=function(n){return arguments.length?(o=Array.from(n,_number.default),f()):o.slice()},p.range=function(n){return arguments.length?(a=Array.from(n),f()):a.slice()},p.rangeRound=function(n){return a=Array.from(n),l=_d3Interpolate.interpolateRound,f()},p.clamp=function(n){return arguments.length?(c=!!n||identity,f()):c!==identity},p.interpolate=function(n){return arguments.length?(l=n,f()):l},p.unknown=function(n){return arguments.length?(t=n,p):t},function(t,e){return n=t,r=e,f()}}function continuous(){return transformer()(identity,identity)}

},{"./constant.js":111,"./number.js":120,"d3-array":17,"d3-interpolate":95}],113:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=diverging,exports.divergingLog=divergingLog,exports.divergingSymlog=divergingSymlog,exports.divergingPow=divergingPow,exports.divergingSqrt=divergingSqrt;var _d3Interpolate=require("d3-interpolate"),_continuous=require("./continuous.js"),_init=require("./init.js"),_linear=require("./linear.js"),_log=require("./log.js"),_sequential=require("./sequential.js"),_symlog=require("./symlog.js"),_pow=require("./pow.js");function transformer(){var n,e,r,i,t,o,u,g=0,a=.5,l=1,p=1,s=_continuous.identity,c=!1;function d(n){return isNaN(n=+n)?u:(n=.5+((n=+o(n))-e)*(p*n<p*e?i:t),s(c?Math.max(0,Math.min(1,n)):n))}function v(n){return function(e){var r,i,t;return arguments.length?([r,i,t]=e,s=(0,_d3Interpolate.piecewise)(n,[r,i,t]),d):[s(0),s(.5),s(1)]}}return d.domain=function(u){return arguments.length?([g,a,l]=u,n=o(g=+g),e=o(a=+a),r=o(l=+l),i=n===e?0:.5/(e-n),t=e===r?0:.5/(r-e),p=e<n?-1:1,d):[g,a,l]},d.clamp=function(n){return arguments.length?(c=!!n,d):c},d.interpolator=function(n){return arguments.length?(s=n,d):s},d.range=v(_d3Interpolate.interpolate),d.rangeRound=v(_d3Interpolate.interpolateRound),d.unknown=function(n){return arguments.length?(u=n,d):u},function(u){return o=u,n=u(g),e=u(a),r=u(l),i=n===e?0:.5/(e-n),t=e===r?0:.5/(r-e),p=e<n?-1:1,d}}function diverging(){var n=(0,_linear.linearish)(transformer()(_continuous.identity));return n.copy=function(){return(0,_sequential.copy)(n,diverging())},_init.initInterpolator.apply(n,arguments)}function divergingLog(){var n=(0,_log.loggish)(transformer()).domain([.1,1,10]);return n.copy=function(){return(0,_sequential.copy)(n,divergingLog()).base(n.base())},_init.initInterpolator.apply(n,arguments)}function divergingSymlog(){var n=(0,_symlog.symlogish)(transformer());return n.copy=function(){return(0,_sequential.copy)(n,divergingSymlog()).constant(n.constant())},_init.initInterpolator.apply(n,arguments)}function divergingPow(){var n=(0,_pow.powish)(transformer());return n.copy=function(){return(0,_sequential.copy)(n,divergingPow()).exponent(n.exponent())},_init.initInterpolator.apply(n,arguments)}function divergingSqrt(){return divergingPow.apply(null,arguments).exponent(.5)}

},{"./continuous.js":112,"./init.js":116,"./linear.js":117,"./log.js":118,"./pow.js":122,"./sequential.js":126,"./symlog.js":128,"d3-interpolate":95}],114:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=identity;var _linear=require("./linear.js"),_number=_interopRequireDefault(require("./number.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function identity(e){var n;function r(e){return isNaN(e=+e)?n:e}return r.invert=r,r.domain=r.range=function(n){return arguments.length?(e=Array.from(n,_number.default),r):e.slice()},r.unknown=function(e){return arguments.length?(n=e,r):n},r.copy=function(){return identity(e).unknown(n)},e=arguments.length?Array.from(e,_number.default):[0,1],(0,_linear.linearish)(r)}

},{"./linear.js":117,"./number.js":120}],115:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"scaleBand",{enumerable:!0,get:function(){return _band.default}}),Object.defineProperty(exports,"scalePoint",{enumerable:!0,get:function(){return _band.point}}),Object.defineProperty(exports,"scaleIdentity",{enumerable:!0,get:function(){return _identity.default}}),Object.defineProperty(exports,"scaleLinear",{enumerable:!0,get:function(){return _linear.default}}),Object.defineProperty(exports,"scaleLog",{enumerable:!0,get:function(){return _log.default}}),Object.defineProperty(exports,"scaleSymlog",{enumerable:!0,get:function(){return _symlog.default}}),Object.defineProperty(exports,"scaleOrdinal",{enumerable:!0,get:function(){return _ordinal.default}}),Object.defineProperty(exports,"scaleImplicit",{enumerable:!0,get:function(){return _ordinal.implicit}}),Object.defineProperty(exports,"scalePow",{enumerable:!0,get:function(){return _pow.default}}),Object.defineProperty(exports,"scaleSqrt",{enumerable:!0,get:function(){return _pow.sqrt}}),Object.defineProperty(exports,"scaleRadial",{enumerable:!0,get:function(){return _radial.default}}),Object.defineProperty(exports,"scaleQuantile",{enumerable:!0,get:function(){return _quantile.default}}),Object.defineProperty(exports,"scaleQuantize",{enumerable:!0,get:function(){return _quantize.default}}),Object.defineProperty(exports,"scaleThreshold",{enumerable:!0,get:function(){return _threshold.default}}),Object.defineProperty(exports,"scaleTime",{enumerable:!0,get:function(){return _time.default}}),Object.defineProperty(exports,"scaleUtc",{enumerable:!0,get:function(){return _utcTime.default}}),Object.defineProperty(exports,"scaleSequential",{enumerable:!0,get:function(){return _sequential.default}}),Object.defineProperty(exports,"scaleSequentialLog",{enumerable:!0,get:function(){return _sequential.sequentialLog}}),Object.defineProperty(exports,"scaleSequentialPow",{enumerable:!0,get:function(){return _sequential.sequentialPow}}),Object.defineProperty(exports,"scaleSequentialSqrt",{enumerable:!0,get:function(){return _sequential.sequentialSqrt}}),Object.defineProperty(exports,"scaleSequentialSymlog",{enumerable:!0,get:function(){return _sequential.sequentialSymlog}}),Object.defineProperty(exports,"scaleSequentialQuantile",{enumerable:!0,get:function(){return _sequentialQuantile.default}}),Object.defineProperty(exports,"scaleDiverging",{enumerable:!0,get:function(){return _diverging.default}}),Object.defineProperty(exports,"scaleDivergingLog",{enumerable:!0,get:function(){return _diverging.divergingLog}}),Object.defineProperty(exports,"scaleDivergingPow",{enumerable:!0,get:function(){return _diverging.divergingPow}}),Object.defineProperty(exports,"scaleDivergingSqrt",{enumerable:!0,get:function(){return _diverging.divergingSqrt}}),Object.defineProperty(exports,"scaleDivergingSymlog",{enumerable:!0,get:function(){return _diverging.divergingSymlog}}),Object.defineProperty(exports,"tickFormat",{enumerable:!0,get:function(){return _tickFormat.default}});var _band=_interopRequireWildcard(require("./band.js")),_identity=_interopRequireDefault(require("./identity.js")),_linear=_interopRequireDefault(require("./linear.js")),_log=_interopRequireDefault(require("./log.js")),_symlog=_interopRequireDefault(require("./symlog.js")),_ordinal=_interopRequireWildcard(require("./ordinal.js")),_pow=_interopRequireWildcard(require("./pow.js")),_radial=_interopRequireDefault(require("./radial.js")),_quantile=_interopRequireDefault(require("./quantile.js")),_quantize=_interopRequireDefault(require("./quantize.js")),_threshold=_interopRequireDefault(require("./threshold.js")),_time=_interopRequireDefault(require("./time.js")),_utcTime=_interopRequireDefault(require("./utcTime.js")),_sequential=_interopRequireWildcard(require("./sequential.js")),_sequentialQuantile=_interopRequireDefault(require("./sequentialQuantile.js")),_diverging=_interopRequireWildcard(require("./diverging.js")),_tickFormat=_interopRequireDefault(require("./tickFormat.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in e)if(Object.prototype.hasOwnProperty.call(e,i)){var u=n?Object.getOwnPropertyDescriptor(e,i):null;u&&(u.get||u.set)?Object.defineProperty(t,i,u):t[i]=e[i]}return t.default=e,r&&r.set(e,t),t}

},{"./band.js":110,"./diverging.js":113,"./identity.js":114,"./linear.js":117,"./log.js":118,"./ordinal.js":121,"./pow.js":122,"./quantile.js":123,"./quantize.js":124,"./radial.js":125,"./sequential.js":126,"./sequentialQuantile.js":127,"./symlog.js":128,"./threshold.js":129,"./tickFormat.js":130,"./time.js":131,"./utcTime.js":132}],116:[function(require,module,exports){
"use strict";function initRange(t,e){switch(arguments.length){case 0:break;case 1:this.range(t);break;default:this.range(e).domain(t)}return this}function initInterpolator(t,e){switch(arguments.length){case 0:break;case 1:"function"==typeof t?this.interpolator(t):this.range(t);break;default:this.domain(t),"function"==typeof e?this.interpolator(e):this.range(e)}return this}Object.defineProperty(exports,"__esModule",{value:!0}),exports.initRange=initRange,exports.initInterpolator=initInterpolator;

},{}],117:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.linearish=linearish,exports.default=linear;var _d3Array=require("d3-array"),_continuous=_interopRequireWildcard(require("./continuous.js")),_init=require("./init.js"),_tickFormat=_interopRequireDefault(require("./tickFormat.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in e)if(Object.prototype.hasOwnProperty.call(e,i)){var a=n?Object.getOwnPropertyDescriptor(e,i):null;a&&(a.get||a.set)?Object.defineProperty(t,i,a):t[i]=e[i]}return t.default=e,r&&r.set(e,t),t}function linearish(e){var r=e.domain;return e.ticks=function(e){var t=r();return(0,_d3Array.ticks)(t[0],t[t.length-1],null==e?10:e)},e.tickFormat=function(e,t){var n=r();return(0,_tickFormat.default)(n[0],n[n.length-1],null==e?10:e,t)},e.nice=function(t){null==t&&(t=10);var n,i=r(),a=0,u=i.length-1,o=i[a],c=i[u];return c<o&&(n=o,o=c,c=n,n=a,a=u,u=n),(n=(0,_d3Array.tickIncrement)(o,c,t))>0?(o=Math.floor(o/n)*n,c=Math.ceil(c/n)*n,n=(0,_d3Array.tickIncrement)(o,c,t)):n<0&&(o=Math.ceil(o*n)/n,c=Math.floor(c*n)/n,n=(0,_d3Array.tickIncrement)(o,c,t)),n>0?(i[a]=Math.floor(o/n)*n,i[u]=Math.ceil(c/n)*n,r(i)):n<0&&(i[a]=Math.ceil(o*n)/n,i[u]=Math.floor(c*n)/n,r(i)),e},e}function linear(){var e=(0,_continuous.default)();return e.copy=function(){return(0,_continuous.copy)(e,linear())},_init.initRange.apply(e,arguments),linearish(e)}

},{"./continuous.js":112,"./init.js":116,"./tickFormat.js":130,"d3-array":17}],118:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.loggish=loggish,exports.default=log;var _d3Array=require("d3-array"),_d3Format=require("d3-format"),_nice=_interopRequireDefault(require("./nice.js")),_continuous=require("./continuous.js"),_init=require("./init.js");function _interopRequireDefault(r){return r&&r.__esModule?r:{default:r}}function transformLog(r){return Math.log(r)}function transformExp(r){return Math.exp(r)}function transformLogn(r){return-Math.log(-r)}function transformExpn(r){return-Math.exp(-r)}function pow10(r){return isFinite(r)?+("1e"+r):r<0?0:r}function powp(r){return 10===r?pow10:r===Math.E?Math.exp:function(n){return Math.pow(r,n)}}function logp(r){return r===Math.E?Math.log:10===r&&Math.log10||2===r&&Math.log2||(r=Math.log(r),function(n){return Math.log(n)/r})}function reflect(r){return function(n){return-r(-n)}}function loggish(r){var n,t,o=r(transformLog,transformExp),e=o.domain,u=10;function i(){return n=logp(u),t=powp(u),e()[0]<0?(n=reflect(n),t=reflect(t),r(transformLogn,transformExpn)):r(transformLog,transformExp),o}return o.base=function(r){return arguments.length?(u=+r,i()):u},o.domain=function(r){return arguments.length?(e(r),i()):e()},o.ticks=function(r){var o,i=e(),a=i[0],f=i[i.length-1];(o=f<a)&&(g=a,a=f,f=g);var c,l,s,g=n(a),h=n(f),p=null==r?10:+r,m=[];if(!(u%1)&&h-g<p){if(g=Math.floor(g),h=Math.ceil(h),a>0){for(;g<=h;++g)for(l=1,c=t(g);l<u;++l)if(!((s=c*l)<a)){if(s>f)break;m.push(s)}}else for(;g<=h;++g)for(l=u-1,c=t(g);l>=1;--l)if(!((s=c*l)<a)){if(s>f)break;m.push(s)}2*m.length<p&&(m=(0,_d3Array.ticks)(a,f,p))}else m=(0,_d3Array.ticks)(g,h,Math.min(h-g,p)).map(t);return o?m.reverse():m},o.tickFormat=function(r,e){if(null==e&&(e=10===u?".0e":","),"function"!=typeof e&&(e=(0,_d3Format.format)(e)),r===1/0)return e;null==r&&(r=10);var i=Math.max(1,u*r/o.ticks().length);return function(r){var o=r/t(Math.round(n(r)));return o*u<u-.5&&(o*=u),o<=i?e(r):""}},o.nice=function(){return e((0,_nice.default)(e(),{floor:function(r){return t(Math.floor(n(r)))},ceil:function(r){return t(Math.ceil(n(r)))}}))},o}function log(){var r=loggish((0,_continuous.transformer)()).domain([1,10]);return r.copy=function(){return(0,_continuous.copy)(r,log()).base(r.base())},_init.initRange.apply(r,arguments),r}

},{"./continuous.js":112,"./init.js":116,"./nice.js":119,"d3-array":17,"d3-format":79}],119:[function(require,module,exports){
"use strict";function _default(e,t){var l,r=0,u=(e=e.slice()).length-1,o=e[r],f=e[u];return f<o&&(l=r,r=u,u=l,l=o,o=f,f=l),e[r]=t.floor(o),e[u]=t.ceil(f),e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],120:[function(require,module,exports){
"use strict";function _default(e){return+e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],121:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=ordinal,exports.implicit=void 0;var _init=require("./init.js");const implicit=Symbol("implicit");function ordinal(){var i=new Map,n=[],t=[],e=implicit;function r(r){var o=r+"",u=i.get(o);if(!u){if(e!==implicit)return e;i.set(o,u=n.push(r))}return t[(u-1)%t.length]}return r.domain=function(t){if(!arguments.length)return n.slice();n=[],i=new Map;for(const e of t){const t=e+"";i.has(t)||i.set(t,n.push(e))}return r},r.range=function(i){return arguments.length?(t=Array.from(i),r):t.slice()},r.unknown=function(i){return arguments.length?(e=i,r):e},r.copy=function(){return ordinal(n,t).unknown(e)},_init.initRange.apply(r,arguments),r}exports.implicit=implicit;

},{"./init.js":116}],122:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.powish=powish,exports.default=pow,exports.sqrt=sqrt;var _linear=require("./linear.js"),_continuous=require("./continuous.js"),_init=require("./init.js");function transformPow(n){return function(t){return t<0?-Math.pow(-t,n):Math.pow(t,n)}}function transformSqrt(n){return n<0?-Math.sqrt(-n):Math.sqrt(n)}function transformSquare(n){return n<0?-n*n:n*n}function powish(n){var t=n(_continuous.identity,_continuous.identity),r=1;return t.exponent=function(t){return arguments.length?1===(r=+t)?n(_continuous.identity,_continuous.identity):.5===r?n(transformSqrt,transformSquare):n(transformPow(r),transformPow(1/r)):r},(0,_linear.linearish)(t)}function pow(){var n=powish((0,_continuous.transformer)());return n.copy=function(){return(0,_continuous.copy)(n,pow()).exponent(n.exponent())},_init.initRange.apply(n,arguments),n}function sqrt(){return pow.apply(null,arguments).exponent(.5)}

},{"./continuous.js":112,"./init.js":116,"./linear.js":117}],123:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=quantile;var _d3Array=require("d3-array"),_init=require("./init.js");function quantile(){var n,r=[],e=[],t=[];function i(){var n=0,i=Math.max(1,e.length);for(t=new Array(i-1);++n<i;)t[n-1]=(0,_d3Array.quantile)(r,n/i);return u}function u(r){return isNaN(r=+r)?n:e[(0,_d3Array.bisect)(t,r)]}return u.invertExtent=function(n){var i=e.indexOf(n);return i<0?[NaN,NaN]:[i>0?t[i-1]:r[0],i<t.length?t[i]:r[r.length-1]]},u.domain=function(n){if(!arguments.length)return r.slice();r=[];for(let e of n)null==e||isNaN(e=+e)||r.push(e);return r.sort(_d3Array.ascending),i()},u.range=function(n){return arguments.length?(e=Array.from(n),i()):e.slice()},u.unknown=function(r){return arguments.length?(n=r,u):n},u.quantiles=function(){return t.slice()},u.copy=function(){return quantile().domain(r).range(e).unknown(n)},_init.initRange.apply(u,arguments)}

},{"./init.js":116,"d3-array":17}],124:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=quantize;var _d3Array=require("d3-array"),_linear=require("./linear.js"),_init=require("./init.js");function quantize(){var n,r=0,e=1,t=1,i=[.5],u=[0,1];function a(r){return r<=r?u[(0,_d3Array.bisect)(i,r,0,t)]:n}function o(){var n=-1;for(i=new Array(t);++n<t;)i[n]=((n+1)*e-(n-t)*r)/(t+1);return a}return a.domain=function(n){return arguments.length?([r,e]=n,r=+r,e=+e,o()):[r,e]},a.range=function(n){return arguments.length?(t=(u=Array.from(n)).length-1,o()):u.slice()},a.invertExtent=function(n){var a=u.indexOf(n);return a<0?[NaN,NaN]:a<1?[r,i[0]]:a>=t?[i[t-1],e]:[i[a-1],i[a]]},a.unknown=function(r){return arguments.length?(n=r,a):a},a.thresholds=function(){return i.slice()},a.copy=function(){return quantize().domain([r,e]).range(u).unknown(n)},_init.initRange.apply((0,_linear.linearish)(a),arguments)}

},{"./init.js":116,"./linear.js":117,"d3-array":17}],125:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=radial;var _continuous=_interopRequireDefault(require("./continuous.js")),_init=require("./init.js"),_linear=require("./linear.js"),_number=_interopRequireDefault(require("./number.js"));function _interopRequireDefault(n){return n&&n.__esModule?n:{default:n}}function square(n){return Math.sign(n)*n*n}function unsquare(n){return Math.sign(n)*Math.sqrt(Math.abs(n))}function radial(){var n,r=(0,_continuous.default)(),e=[0,1],u=!1;function t(e){var t=unsquare(r(e));return isNaN(t)?n:u?Math.round(t):t}return t.invert=function(n){return r.invert(square(n))},t.domain=function(n){return arguments.length?(r.domain(n),t):r.domain()},t.range=function(n){return arguments.length?(r.range((e=Array.from(n,_number.default)).map(square)),t):e.slice()},t.rangeRound=function(n){return t.range(n).round(!0)},t.round=function(n){return arguments.length?(u=!!n,t):u},t.clamp=function(n){return arguments.length?(r.clamp(n),t):r.clamp()},t.unknown=function(r){return arguments.length?(n=r,t):n},t.copy=function(){return radial(r.domain(),e).round(u).clamp(r.clamp()).unknown(n)},_init.initRange.apply(t,arguments),(0,_linear.linearish)(t)}

},{"./continuous.js":112,"./init.js":116,"./linear.js":117,"./number.js":120}],126:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.copy=copy,exports.default=sequential,exports.sequentialLog=sequentialLog,exports.sequentialSymlog=sequentialSymlog,exports.sequentialPow=sequentialPow,exports.sequentialSqrt=sequentialSqrt;var _d3Interpolate=require("d3-interpolate"),_continuous=require("./continuous.js"),_init=require("./init.js"),_linear=require("./linear.js"),_log=require("./log.js"),_symlog=require("./symlog.js"),_pow=require("./pow.js");function transformer(){var n,t,e,r,o,i=0,u=1,a=_continuous.identity,l=!1;function s(t){return isNaN(t=+t)?o:a(0===e?.5:(t=(r(t)-n)*e,l?Math.max(0,Math.min(1,t)):t))}function p(n){return function(t){var e,r;return arguments.length?([e,r]=t,a=n(e,r),s):[a(0),a(1)]}}return s.domain=function(o){return arguments.length?([i,u]=o,n=r(i=+i),t=r(u=+u),e=n===t?0:1/(t-n),s):[i,u]},s.clamp=function(n){return arguments.length?(l=!!n,s):l},s.interpolator=function(n){return arguments.length?(a=n,s):a},s.range=p(_d3Interpolate.interpolate),s.rangeRound=p(_d3Interpolate.interpolateRound),s.unknown=function(n){return arguments.length?(o=n,s):o},function(o){return r=o,n=o(i),t=o(u),e=n===t?0:1/(t-n),s}}function copy(n,t){return t.domain(n.domain()).interpolator(n.interpolator()).clamp(n.clamp()).unknown(n.unknown())}function sequential(){var n=(0,_linear.linearish)(transformer()(_continuous.identity));return n.copy=function(){return copy(n,sequential())},_init.initInterpolator.apply(n,arguments)}function sequentialLog(){var n=(0,_log.loggish)(transformer()).domain([1,10]);return n.copy=function(){return copy(n,sequentialLog()).base(n.base())},_init.initInterpolator.apply(n,arguments)}function sequentialSymlog(){var n=(0,_symlog.symlogish)(transformer());return n.copy=function(){return copy(n,sequentialSymlog()).constant(n.constant())},_init.initInterpolator.apply(n,arguments)}function sequentialPow(){var n=(0,_pow.powish)(transformer());return n.copy=function(){return copy(n,sequentialPow()).exponent(n.exponent())},_init.initInterpolator.apply(n,arguments)}function sequentialSqrt(){return sequentialPow.apply(null,arguments).exponent(.5)}

},{"./continuous.js":112,"./init.js":116,"./linear.js":117,"./log.js":118,"./pow.js":122,"./symlog.js":128,"d3-interpolate":95}],127:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=sequentialQuantile;var _d3Array=require("d3-array"),_continuous=require("./continuous.js"),_init=require("./init.js");function sequentialQuantile(){var n=[],t=_continuous.identity;function e(e){if(!isNaN(e=+e))return t(((0,_d3Array.bisect)(n,e,1)-1)/(n.length-1))}return e.domain=function(t){if(!arguments.length)return n.slice();n=[];for(let e of t)null==e||isNaN(e=+e)||n.push(e);return n.sort(_d3Array.ascending),e},e.interpolator=function(n){return arguments.length?(t=n,e):t},e.range=function(){return n.map((e,r)=>t(r/(n.length-1)))},e.quantiles=function(t){return Array.from({length:t+1},(e,r)=>(0,_d3Array.quantile)(n,r/t))},e.copy=function(){return sequentialQuantile(t).domain(n)},_init.initInterpolator.apply(e,arguments)}

},{"./continuous.js":112,"./init.js":116,"d3-array":17}],128:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.symlogish=symlogish,exports.default=symlog;var _linear=require("./linear.js"),_continuous=require("./continuous.js"),_init=require("./init.js");function transformSymlog(n){return function(t){return Math.sign(t)*Math.log1p(Math.abs(t/n))}}function transformSymexp(n){return function(t){return Math.sign(t)*Math.expm1(Math.abs(t))*n}}function symlogish(n){var t=1,r=n(transformSymlog(t),transformSymexp(t));return r.constant=function(r){return arguments.length?n(transformSymlog(t=+r),transformSymexp(t)):t},(0,_linear.linearish)(r)}function symlog(){var n=symlogish((0,_continuous.transformer)());return n.copy=function(){return(0,_continuous.copy)(n,symlog()).constant(n.constant())},_init.initRange.apply(n,arguments)}

},{"./continuous.js":112,"./init.js":116,"./linear.js":117}],129:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=threshold;var _d3Array=require("d3-array"),_init=require("./init.js");function threshold(){var n,r=[.5],e=[0,1],t=1;function i(i){return i<=i?e[(0,_d3Array.bisect)(r,i,0,t)]:n}return i.domain=function(n){return arguments.length?(r=Array.from(n),t=Math.min(r.length,e.length-1),i):r.slice()},i.range=function(n){return arguments.length?(e=Array.from(n),t=Math.min(r.length,e.length-1),i):e.slice()},i.invertExtent=function(n){var t=e.indexOf(n);return[r[t-1],r[t]]},i.unknown=function(r){return arguments.length?(n=r,i):n},i.copy=function(){return threshold().domain(r).range(e).unknown(n)},_init.initRange.apply(i,arguments)}

},{"./init.js":116,"d3-array":17}],130:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Array=require("d3-array"),_d3Format=require("d3-format");function _default(e,r,a,t){var i,s=(0,_d3Array.tickStep)(e,r,a);switch((t=(0,_d3Format.formatSpecifier)(null==t?",f":t)).type){case"s":var o=Math.max(Math.abs(e),Math.abs(r));return null!=t.precision||isNaN(i=(0,_d3Format.precisionPrefix)(s,o))||(t.precision=i),(0,_d3Format.formatPrefix)(t,o);case"":case"e":case"g":case"p":case"r":null!=t.precision||isNaN(i=(0,_d3Format.precisionRound)(s,Math.max(Math.abs(e),Math.abs(r))))||(t.precision=i-("e"===t.type));break;case"f":case"%":null!=t.precision||isNaN(i=(0,_d3Format.precisionFixed)(s))||(t.precision=i-2*("%"===t.type))}return(0,_d3Format.format)(t)}

},{"d3-array":17,"d3-format":79}],131:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.calendar=calendar,exports.default=_default;var _d3Array=require("d3-array"),_d3Time=require("d3-time"),_d3TimeFormat=require("d3-time-format"),_continuous=_interopRequireWildcard(require("./continuous.js")),_init=require("./init.js"),_nice=_interopRequireDefault(require("./nice.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in e)if(Object.prototype.hasOwnProperty.call(e,i)){var u=n?Object.getOwnPropertyDescriptor(e,i):null;u&&(u.get||u.set)?Object.defineProperty(t,i,u):t[i]=e[i]}return t.default=e,r&&r.set(e,t),t}var durationSecond=1e3,durationMinute=60*durationSecond,durationHour=60*durationMinute,durationDay=24*durationHour,durationWeek=7*durationDay,durationMonth=30*durationDay,durationYear=365*durationDay;function date(e){return new Date(e)}function number(e){return e instanceof Date?+e:+new Date(+e)}function calendar(e,r,t,n,i,u,a,o,d){var c=(0,_continuous.default)(),f=c.invert,l=c.domain,_=d(".%L"),m=d(":%S"),p=d("%I:%M"),y=d("%I %p"),s=d("%a %d"),M=d("%b %d"),D=d("%B"),h=d("%Y"),v=[[a,1,durationSecond],[a,5,5*durationSecond],[a,15,15*durationSecond],[a,30,30*durationSecond],[u,1,durationMinute],[u,5,5*durationMinute],[u,15,15*durationMinute],[u,30,30*durationMinute],[i,1,durationHour],[i,3,3*durationHour],[i,6,6*durationHour],[i,12,12*durationHour],[n,1,durationDay],[n,2,2*durationDay],[t,1,durationWeek],[r,1,durationMonth],[r,3,3*durationMonth],[e,1,durationYear]];function g(o){return(a(o)<o?_:u(o)<o?m:i(o)<o?p:n(o)<o?y:r(o)<o?t(o)<o?s:M:e(o)<o?D:h)(o)}function b(r,t,n){if(null==r&&(r=10),"number"==typeof r){var i,u=Math.abs(n-t)/r,a=(0,_d3Array.bisector)(function(e){return e[2]}).right(v,u);return a===v.length?(i=(0,_d3Array.tickStep)(t/durationYear,n/durationYear,r),r=e):a?(i=(a=v[u/v[a-1][2]<v[a][2]/u?a-1:a])[1],r=a[0]):(i=Math.max((0,_d3Array.tickStep)(t,n,r),1),r=o),r.every(i)}return r}return c.invert=function(e){return new Date(f(e))},c.domain=function(e){return arguments.length?l(Array.from(e,number)):l().map(date)},c.ticks=function(e){var r,t=l(),n=t[0],i=t[t.length-1],u=i<n;return u&&(r=n,n=i,i=r),r=(r=b(e,n,i))?r.range(n,i+1):[],u?r.reverse():r},c.tickFormat=function(e,r){return null==r?g:d(r)},c.nice=function(e){var r=l();return(e=b(e,r[0],r[r.length-1]))?l((0,_nice.default)(r,e)):c},c.copy=function(){return(0,_continuous.copy)(c,calendar(e,r,t,n,i,u,a,o,d))},c}function _default(){return _init.initRange.apply(calendar(_d3Time.timeYear,_d3Time.timeMonth,_d3Time.timeWeek,_d3Time.timeDay,_d3Time.timeHour,_d3Time.timeMinute,_d3Time.timeSecond,_d3Time.timeMillisecond,_d3TimeFormat.timeFormat).domain([new Date(2e3,0,1),new Date(2e3,0,2)]),arguments)}

},{"./continuous.js":112,"./init.js":116,"./nice.js":119,"d3-array":17,"d3-time":192,"d3-time-format":185}],132:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _time=require("./time.js"),_d3TimeFormat=require("d3-time-format"),_d3Time=require("d3-time"),_init=require("./init.js");function _default(){return _init.initRange.apply((0,_time.calendar)(_d3Time.utcYear,_d3Time.utcMonth,_d3Time.utcWeek,_d3Time.utcDay,_d3Time.utcHour,_d3Time.utcMinute,_d3Time.utcSecond,_d3Time.utcMillisecond,_d3TimeFormat.utcFormat).domain([Date.UTC(2e3,0,1),Date.UTC(2e3,0,2)]),arguments)}

},{"./init.js":116,"./time.js":131,"d3-time":192,"d3-time-format":185}],133:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],134:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _creator=_interopRequireDefault(require("./creator")),_select=_interopRequireDefault(require("./select"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return(0,_select.default)((0,_creator.default)(e).call(document.documentElement))}

},{"./creator":135,"./select":143}],135:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _namespace=_interopRequireDefault(require("./namespace")),_namespaces=require("./namespaces");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function creatorInherit(e){return function(){var t=this.ownerDocument,r=this.namespaceURI;return r===_namespaces.xhtml&&t.documentElement.namespaceURI===_namespaces.xhtml?t.createElement(e):t.createElementNS(r,e)}}function creatorFixed(e){return function(){return this.ownerDocument.createElementNS(e.space,e.local)}}function _default(e){var t=(0,_namespace.default)(e);return(t.local?creatorFixed:creatorInherit)(t)}

},{"./namespace":140,"./namespaces":141}],136:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"create",{enumerable:!0,get:function(){return _create.default}}),Object.defineProperty(exports,"creator",{enumerable:!0,get:function(){return _creator.default}}),Object.defineProperty(exports,"local",{enumerable:!0,get:function(){return _local.default}}),Object.defineProperty(exports,"matcher",{enumerable:!0,get:function(){return _matcher.default}}),Object.defineProperty(exports,"mouse",{enumerable:!0,get:function(){return _mouse.default}}),Object.defineProperty(exports,"namespace",{enumerable:!0,get:function(){return _namespace.default}}),Object.defineProperty(exports,"namespaces",{enumerable:!0,get:function(){return _namespaces.default}}),Object.defineProperty(exports,"clientPoint",{enumerable:!0,get:function(){return _point.default}}),Object.defineProperty(exports,"select",{enumerable:!0,get:function(){return _select.default}}),Object.defineProperty(exports,"selectAll",{enumerable:!0,get:function(){return _selectAll.default}}),Object.defineProperty(exports,"selection",{enumerable:!0,get:function(){return _index.default}}),Object.defineProperty(exports,"selector",{enumerable:!0,get:function(){return _selector.default}}),Object.defineProperty(exports,"selectorAll",{enumerable:!0,get:function(){return _selectorAll.default}}),Object.defineProperty(exports,"style",{enumerable:!0,get:function(){return _style.styleValue}}),Object.defineProperty(exports,"touch",{enumerable:!0,get:function(){return _touch.default}}),Object.defineProperty(exports,"touches",{enumerable:!0,get:function(){return _touches.default}}),Object.defineProperty(exports,"window",{enumerable:!0,get:function(){return _window.default}}),Object.defineProperty(exports,"event",{enumerable:!0,get:function(){return _on.event}}),Object.defineProperty(exports,"customEvent",{enumerable:!0,get:function(){return _on.customEvent}});var _create=_interopRequireDefault(require("./create")),_creator=_interopRequireDefault(require("./creator")),_local=_interopRequireDefault(require("./local")),_matcher=_interopRequireDefault(require("./matcher")),_mouse=_interopRequireDefault(require("./mouse")),_namespace=_interopRequireDefault(require("./namespace")),_namespaces=_interopRequireDefault(require("./namespaces")),_point=_interopRequireDefault(require("./point")),_select=_interopRequireDefault(require("./select")),_selectAll=_interopRequireDefault(require("./selectAll")),_index=_interopRequireDefault(require("./selection/index")),_selector=_interopRequireDefault(require("./selector")),_selectorAll=_interopRequireDefault(require("./selectorAll")),_style=require("./selection/style"),_touch=_interopRequireDefault(require("./touch")),_touches=_interopRequireDefault(require("./touches")),_window=_interopRequireDefault(require("./window")),_on=require("./selection/on");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./create":134,"./creator":135,"./local":137,"./matcher":138,"./mouse":139,"./namespace":140,"./namespaces":141,"./point":142,"./select":143,"./selectAll":144,"./selection/index":159,"./selection/on":166,"./selection/style":176,"./selector":178,"./selectorAll":179,"./touch":181,"./touches":182,"./window":183}],137:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=local;var nextId=0;function local(){return new Local}function Local(){this._="@"+(++nextId).toString(36)}Local.prototype=local.prototype={constructor:Local,get:function(t){for(var e=this._;!(e in t);)if(!(t=t.parentNode))return;return t[e]},set:function(t,e){return t[this._]=e},remove:function(t){return this._ in t&&delete t[this._]},toString:function(){return this._}};

},{}],138:[function(require,module,exports){
"use strict";function _default(e){return function(){return this.matches(e)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],139:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _sourceEvent=_interopRequireDefault(require("./sourceEvent")),_point=_interopRequireDefault(require("./point"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){var t=(0,_sourceEvent.default)();return t.changedTouches&&(t=t.changedTouches[0]),(0,_point.default)(e,t)}

},{"./point":142,"./sourceEvent":180}],140:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _namespaces=_interopRequireDefault(require("./namespaces"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){var a=e+="",t=a.indexOf(":");return t>=0&&"xmlns"!==(a=e.slice(0,t))&&(e=e.slice(t+1)),_namespaces.default.hasOwnProperty(a)?{space:_namespaces.default[a],local:e}:e}

},{"./namespaces":141}],141:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=exports.xhtml=void 0;var xhtml="http://www.w3.org/1999/xhtml";exports.xhtml=xhtml;var _default={svg:"http://www.w3.org/2000/svg",xhtml:xhtml,xlink:"http://www.w3.org/1999/xlink",xml:"http://www.w3.org/XML/1998/namespace",xmlns:"http://www.w3.org/2000/xmlns/"};exports.default=_default;

},{}],142:[function(require,module,exports){
"use strict";function _default(e,t){var n=e.ownerSVGElement||e;if(n.createSVGPoint){var r=n.createSVGPoint();return r.x=t.clientX,r.y=t.clientY,[(r=r.matrixTransform(e.getScreenCTM().inverse())).x,r.y]}var i=e.getBoundingClientRect();return[t.clientX-i.left-e.clientLeft,t.clientY-i.top-e.clientTop]}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],143:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./selection/index");function _default(e){return"string"==typeof e?new _index.Selection([[document.querySelector(e)]],[document.documentElement]):new _index.Selection([[e]],_index.root)}

},{"./selection/index":159}],144:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./selection/index");function _default(e){return"string"==typeof e?new _index.Selection([document.querySelectorAll(e)],[document.documentElement]):new _index.Selection([null==e?[]:e],_index.root)}

},{"./selection/index":159}],145:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _creator=_interopRequireDefault(require("../creator"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){var t="function"==typeof e?e:(0,_creator.default)(e);return this.select(function(){return this.appendChild(t.apply(this,arguments))})}

},{"../creator":135}],146:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _namespace=_interopRequireDefault(require("../namespace"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function attrRemove(t){return function(){this.removeAttribute(t)}}function attrRemoveNS(t){return function(){this.removeAttributeNS(t.space,t.local)}}function attrConstant(t,e){return function(){this.setAttribute(t,e)}}function attrConstantNS(t,e){return function(){this.setAttributeNS(t.space,t.local,e)}}function attrFunction(t,e){return function(){var n=e.apply(this,arguments);null==n?this.removeAttribute(t):this.setAttribute(t,n)}}function attrFunctionNS(t,e){return function(){var n=e.apply(this,arguments);null==n?this.removeAttributeNS(t.space,t.local):this.setAttributeNS(t.space,t.local,n)}}function _default(t,e){var n=(0,_namespace.default)(t);if(arguments.length<2){var r=this.node();return n.local?r.getAttributeNS(n.space,n.local):r.getAttribute(n)}return this.each((null==e?n.local?attrRemoveNS:attrRemove:"function"==typeof e?n.local?attrFunctionNS:attrFunction:n.local?attrConstantNS:attrConstant)(n,e))}

},{"../namespace":140}],147:[function(require,module,exports){
"use strict";function _default(){var e=arguments[0];return arguments[0]=this,e.apply(null,arguments),this}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],148:[function(require,module,exports){
"use strict";function classArray(s){return s.trim().split(/^|\s+/)}function classList(s){return s.classList||new ClassList(s)}function ClassList(s){this._node=s,this._names=classArray(s.getAttribute("class")||"")}function classedAdd(s,t){for(var e=classList(s),n=-1,i=t.length;++n<i;)e.add(t[n])}function classedRemove(s,t){for(var e=classList(s),n=-1,i=t.length;++n<i;)e.remove(t[n])}function classedTrue(s){return function(){classedAdd(this,s)}}function classedFalse(s){return function(){classedRemove(this,s)}}function classedFunction(s,t){return function(){(t.apply(this,arguments)?classedAdd:classedRemove)(this,s)}}function _default(s,t){var e=classArray(s+"");if(arguments.length<2){for(var n=classList(this.node()),i=-1,a=e.length;++i<a;)if(!n.contains(e[i]))return!1;return!0}return this.each(("function"==typeof t?classedFunction:t?classedTrue:classedFalse)(e,t))}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,ClassList.prototype={add:function(s){this._names.indexOf(s)<0&&(this._names.push(s),this._node.setAttribute("class",this._names.join(" ")))},remove:function(s){var t=this._names.indexOf(s);t>=0&&(this._names.splice(t,1),this._node.setAttribute("class",this._names.join(" ")))},contains:function(s){return this._names.indexOf(s)>=0}};

},{}],149:[function(require,module,exports){
"use strict";function selection_cloneShallow(){var e=this.cloneNode(!1),t=this.parentNode;return t?t.insertBefore(e,this.nextSibling):e}function selection_cloneDeep(){var e=this.cloneNode(!0),t=this.parentNode;return t?t.insertBefore(e,this.nextSibling):e}function _default(e){return this.select(e?selection_cloneDeep:selection_cloneShallow)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],150:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index"),_enter=require("./enter"),_constant=_interopRequireDefault(require("../constant"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var keyPrefix="$";function bindIndex(e,r,n,t,a,_){for(var i,o=0,f=r.length,l=_.length;o<l;++o)(i=r[o])?(i.__data__=_[o],t[o]=i):n[o]=new _enter.EnterNode(e,_[o]);for(;o<f;++o)(i=r[o])&&(a[o]=i)}function bindKey(e,r,n,t,a,_,i){var o,f,l,u={},d=r.length,c=_.length,s=new Array(d);for(o=0;o<d;++o)(f=r[o])&&(s[o]=l=keyPrefix+i.call(f,f.__data__,o,r),l in u?a[o]=f:u[l]=f);for(o=0;o<c;++o)(f=u[l=keyPrefix+i.call(e,_[o],o,_)])?(t[o]=f,f.__data__=_[o],u[l]=null):n[o]=new _enter.EnterNode(e,_[o]);for(o=0;o<d;++o)(f=r[o])&&u[s[o]]===f&&(a[o]=f)}function _default(e,r){if(!e)return s=new Array(this.size()),l=-1,this.each(function(e){s[++l]=e}),s;var n=r?bindKey:bindIndex,t=this._parents,a=this._groups;"function"!=typeof e&&(e=(0,_constant.default)(e));for(var _=a.length,i=new Array(_),o=new Array(_),f=new Array(_),l=0;l<_;++l){var u=t[l],d=a[l],c=d.length,s=e.call(u,u&&u.__data__,l,t),y=s.length,h=o[l]=new Array(y),x=i[l]=new Array(y);n(u,d,h,x,f[l]=new Array(c),s,r);for(var w,v,g=0,p=0;g<y;++g)if(w=h[g]){for(g>=p&&(p=g+1);!(v=x[p])&&++p<y;);w._next=v||null}}return(i=new _index.Selection(i,t))._enter=o,i._exit=f,i}

},{"../constant":133,"./enter":155,"./index":159}],151:[function(require,module,exports){
"use strict";function _default(e){return arguments.length?this.property("__data__",e):this.node().__data__}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],152:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _window=_interopRequireDefault(require("../window"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function dispatchEvent(t,e,n){var i=(0,_window.default)(t),u=i.CustomEvent;"function"==typeof u?u=new u(e,n):(u=i.document.createEvent("Event"),n?(u.initEvent(e,n.bubbles,n.cancelable),u.detail=n.detail):u.initEvent(e,!1,!1)),t.dispatchEvent(u)}function dispatchConstant(t,e){return function(){return dispatchEvent(this,t,e)}}function dispatchFunction(t,e){return function(){return dispatchEvent(this,t,e.apply(this,arguments))}}function _default(t,e){return this.each(("function"==typeof e?dispatchFunction:dispatchConstant)(t,e))}

},{"../window":183}],153:[function(require,module,exports){
"use strict";function _default(e){for(var t=this._groups,r=0,a=t.length;r<a;++r)for(var l,u=t[r],_=0,o=u.length;_<o;++_)(l=u[_])&&e.call(l,l.__data__,_,u);return this}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],154:[function(require,module,exports){
"use strict";function _default(){return!this.node()}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],155:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.EnterNode=EnterNode;var _sparse=_interopRequireDefault(require("./sparse")),_index=require("./index");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(){return new _index.Selection(this._enter||this._groups.map(_sparse.default),this._parents)}function EnterNode(e,t){this.ownerDocument=e.ownerDocument,this.namespaceURI=e.namespaceURI,this._next=null,this._parent=e,this.__data__=t}EnterNode.prototype={constructor:EnterNode,appendChild:function(e){return this._parent.insertBefore(e,this._next)},insertBefore:function(e,t){return this._parent.insertBefore(e,t)},querySelector:function(e){return this._parent.querySelector(e)},querySelectorAll:function(e){return this._parent.querySelectorAll(e)}};

},{"./index":159,"./sparse":175}],156:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _sparse=_interopRequireDefault(require("./sparse")),_index=require("./index");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(){return new _index.Selection(this._exit||this._groups.map(_sparse.default),this._parents)}

},{"./index":159,"./sparse":175}],157:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index"),_matcher=_interopRequireDefault(require("../matcher"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){"function"!=typeof e&&(e=(0,_matcher.default)(e));for(var t=this._groups,r=t.length,u=new Array(r),n=0;n<r;++n)for(var a,_=t[n],i=_.length,l=u[n]=[],o=0;o<i;++o)(a=_[o])&&e.call(a,a.__data__,o,_)&&l.push(a);return new _index.Selection(u,this._parents)}

},{"../matcher":138,"./index":159}],158:[function(require,module,exports){
"use strict";function htmlRemove(){this.innerHTML=""}function htmlConstant(t){return function(){this.innerHTML=t}}function htmlFunction(t){return function(){var n=t.apply(this,arguments);this.innerHTML=null==n?"":n}}function _default(t){return arguments.length?this.each(null==t?htmlRemove:("function"==typeof t?htmlFunction:htmlConstant)(t)):this.node().innerHTML}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],159:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.Selection=Selection,exports.default=exports.root=void 0;var _select=_interopRequireDefault(require("./select")),_selectAll=_interopRequireDefault(require("./selectAll")),_filter=_interopRequireDefault(require("./filter")),_data=_interopRequireDefault(require("./data")),_enter=_interopRequireDefault(require("./enter")),_exit=_interopRequireDefault(require("./exit")),_join=_interopRequireDefault(require("./join")),_merge=_interopRequireDefault(require("./merge")),_order=_interopRequireDefault(require("./order")),_sort=_interopRequireDefault(require("./sort")),_call=_interopRequireDefault(require("./call")),_nodes=_interopRequireDefault(require("./nodes")),_node=_interopRequireDefault(require("./node")),_size=_interopRequireDefault(require("./size")),_empty=_interopRequireDefault(require("./empty")),_each=_interopRequireDefault(require("./each")),_attr=_interopRequireDefault(require("./attr")),_style=_interopRequireDefault(require("./style")),_property=_interopRequireDefault(require("./property")),_classed=_interopRequireDefault(require("./classed")),_text=_interopRequireDefault(require("./text")),_html=_interopRequireDefault(require("./html")),_raise=_interopRequireDefault(require("./raise")),_lower=_interopRequireDefault(require("./lower")),_append=_interopRequireDefault(require("./append")),_insert=_interopRequireDefault(require("./insert")),_remove=_interopRequireDefault(require("./remove")),_clone=_interopRequireDefault(require("./clone")),_datum=_interopRequireDefault(require("./datum")),_on=_interopRequireDefault(require("./on")),_dispatch=_interopRequireDefault(require("./dispatch"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var root=[null];function Selection(e,t){this._groups=e,this._parents=t}function selection(){return new Selection([[document.documentElement]],root)}exports.root=root,Selection.prototype=selection.prototype={constructor:Selection,select:_select.default,selectAll:_selectAll.default,filter:_filter.default,data:_data.default,enter:_enter.default,exit:_exit.default,join:_join.default,merge:_merge.default,order:_order.default,sort:_sort.default,call:_call.default,nodes:_nodes.default,node:_node.default,size:_size.default,empty:_empty.default,each:_each.default,attr:_attr.default,style:_style.default,property:_property.default,classed:_classed.default,text:_text.default,html:_html.default,raise:_raise.default,lower:_lower.default,append:_append.default,insert:_insert.default,remove:_remove.default,clone:_clone.default,datum:_datum.default,on:_on.default,dispatch:_dispatch.default};var _default=selection;exports.default=_default;

},{"./append":145,"./attr":146,"./call":147,"./classed":148,"./clone":149,"./data":150,"./datum":151,"./dispatch":152,"./each":153,"./empty":154,"./enter":155,"./exit":156,"./filter":157,"./html":158,"./insert":160,"./join":161,"./lower":162,"./merge":163,"./node":164,"./nodes":165,"./on":166,"./order":167,"./property":168,"./raise":169,"./remove":170,"./select":171,"./selectAll":172,"./size":173,"./sort":174,"./style":176,"./text":177}],160:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _creator=_interopRequireDefault(require("../creator")),_selector=_interopRequireDefault(require("../selector"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function constantNull(){return null}function _default(e,t){var r="function"==typeof e?e:(0,_creator.default)(e),u=null==t?constantNull:"function"==typeof t?t:(0,_selector.default)(t);return this.select(function(){return this.insertBefore(r.apply(this,arguments),u.apply(this,arguments)||null)})}

},{"../creator":135,"../selector":178}],161:[function(require,module,exports){
"use strict";function _default(e,t,r){var u=this.enter(),n=this,l=this.exit();return u="function"==typeof e?e(u):u.append(e+""),null!=t&&(n=t(n)),null==r?l.remove():r(l),u&&n?u.merge(n).order():n}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],162:[function(require,module,exports){
"use strict";function lower(){this.previousSibling&&this.parentNode.insertBefore(this,this.parentNode.firstChild)}function _default(){return this.each(lower)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],163:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index");function _default(e){for(var r=this._groups,t=e._groups,n=r.length,a=t.length,i=Math.min(n,a),o=new Array(n),u=0;u<i;++u)for(var s,l=r[u],_=t[u],d=l.length,f=o[u]=new Array(d),h=0;h<d;++h)(s=l[h]||_[h])&&(f[h]=s);for(;u<n;++u)o[u]=r[u];return new _index.Selection(o,this._parents)}

},{"./index":159}],164:[function(require,module,exports){
"use strict";function _default(){for(var e=this._groups,r=0,t=e.length;r<t;++r)for(var u=e[r],l=0,f=u.length;l<f;++l){var n=u[l];if(n)return n}return null}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],165:[function(require,module,exports){
"use strict";function _default(){var e=new Array(this.size()),t=-1;return this.each(function(){e[++t]=this}),e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],166:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.customEvent=customEvent,exports.event=void 0;var filterEvents={},event=null;if(exports.event=event,"undefined"!=typeof document){var element=document.documentElement;"onmouseenter"in element||(filterEvents={mouseenter:"mouseover",mouseleave:"mouseout"})}function filterContextListener(e,t,n){return e=contextListener(e,t,n),function(t){var n=t.relatedTarget;n&&(n===this||8&n.compareDocumentPosition(this))||e.call(this,t)}}function contextListener(e,t,n){return function(r){var o=event;exports.event=event=r;try{e.call(this,this.__data__,t,n)}finally{exports.event=event=o}}}function parseTypenames(e){return e.trim().split(/^|\s+/).map(function(e){var t="",n=e.indexOf(".");return n>=0&&(t=e.slice(n+1),e=e.slice(0,n)),{type:e,name:t}})}function onRemove(e){return function(){var t=this.__on;if(t){for(var n,r=0,o=-1,i=t.length;r<i;++r)n=t[r],e.type&&n.type!==e.type||n.name!==e.name?t[++o]=n:this.removeEventListener(n.type,n.listener,n.capture);++o?t.length=o:delete this.__on}}}function onAdd(e,t,n){var r=filterEvents.hasOwnProperty(e.type)?filterContextListener:contextListener;return function(o,i,s){var a,u=this.__on,v=r(t,i,s);if(u)for(var l=0,p=u.length;l<p;++l)if((a=u[l]).type===e.type&&a.name===e.name)return this.removeEventListener(a.type,a.listener,a.capture),this.addEventListener(a.type,a.listener=v,a.capture=n),void(a.value=t);this.addEventListener(e.type,v,n),a={type:e.type,name:e.name,value:t,listener:v,capture:n},u?u.push(a):this.__on=[a]}}function _default(e,t,n){var r,o,i=parseTypenames(e+""),s=i.length;if(!(arguments.length<2)){for(a=t?onAdd:onRemove,null==n&&(n=!1),r=0;r<s;++r)this.each(a(i[r],t,n));return this}var a=this.node().__on;if(a)for(var u,v=0,l=a.length;v<l;++v)for(r=0,u=a[v];r<s;++r)if((o=i[r]).type===u.type&&o.name===u.name)return u.value}function customEvent(e,t,n,r){var o=event;e.sourceEvent=event,exports.event=event=e;try{return t.apply(n,r)}finally{exports.event=event=o}}

},{}],167:[function(require,module,exports){
"use strict";function _default(){for(var e=this._groups,t=-1,r=e.length;++t<r;)for(var o,n=e[t],s=n.length-1,u=n[s];--s>=0;)(o=n[s])&&(u&&4^o.compareDocumentPosition(u)&&u.parentNode.insertBefore(o,u),u=o);return this}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],168:[function(require,module,exports){
"use strict";function propertyRemove(t){return function(){delete this[t]}}function propertyConstant(t,e){return function(){this[t]=e}}function propertyFunction(t,e){return function(){var n=e.apply(this,arguments);null==n?delete this[t]:this[t]=n}}function _default(t,e){return arguments.length>1?this.each((null==e?propertyRemove:"function"==typeof e?propertyFunction:propertyConstant)(t,e)):this.node()[t]}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],169:[function(require,module,exports){
"use strict";function raise(){this.nextSibling&&this.parentNode.appendChild(this)}function _default(){return this.each(raise)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],170:[function(require,module,exports){
"use strict";function remove(){var e=this.parentNode;e&&e.removeChild(this)}function _default(){return this.each(remove)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],171:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index"),_selector=_interopRequireDefault(require("../selector"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){"function"!=typeof e&&(e=(0,_selector.default)(e));for(var t=this._groups,r=t.length,_=new Array(r),a=0;a<r;++a)for(var n,u,i=t[a],l=i.length,o=_[a]=new Array(l),d=0;d<l;++d)(n=i[d])&&(u=e.call(n,n.__data__,d,i))&&("__data__"in n&&(u.__data__=n.__data__),o[d]=u);return new _index.Selection(_,this._parents)}

},{"../selector":178,"./index":159}],172:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index"),_selectorAll=_interopRequireDefault(require("../selectorAll"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){"function"!=typeof e&&(e=(0,_selectorAll.default)(e));for(var t=this._groups,r=t.length,l=[],u=[],n=0;n<r;++n)for(var o,_=t[n],i=_.length,a=0;a<i;++a)(o=_[a])&&(l.push(e.call(o,o.__data__,a,_)),u.push(o));return new _index.Selection(l,u)}

},{"../selectorAll":179,"./index":159}],173:[function(require,module,exports){
"use strict";function _default(){var e=0;return this.each(function(){++e}),e}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],174:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index");function _default(e){function r(r,n){return r&&n?e(r.__data__,n.__data__):!r-!n}e||(e=ascending);for(var n=this._groups,t=n.length,a=new Array(t),_=0;_<t;++_){for(var i,u=n[_],d=u.length,o=a[_]=new Array(d),s=0;s<d;++s)(i=u[s])&&(o[s]=i);o.sort(r)}return new _index.Selection(a,this._parents).order()}function ascending(e,r){return e<r?-1:e>r?1:e>=r?0:NaN}

},{"./index":159}],175:[function(require,module,exports){
"use strict";function _default(e){return new Array(e.length)}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],176:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.styleValue=styleValue;var _window=_interopRequireDefault(require("../window"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function styleRemove(e){return function(){this.style.removeProperty(e)}}function styleConstant(e,t,n){return function(){this.style.setProperty(e,t,n)}}function styleFunction(e,t,n){return function(){var u=t.apply(this,arguments);null==u?this.style.removeProperty(e):this.style.setProperty(e,u,n)}}function _default(e,t,n){return arguments.length>1?this.each((null==t?styleRemove:"function"==typeof t?styleFunction:styleConstant)(e,t,null==n?"":n)):styleValue(this.node(),e)}function styleValue(e,t){return e.style.getPropertyValue(t)||(0,_window.default)(e).getComputedStyle(e,null).getPropertyValue(t)}

},{"../window":183}],177:[function(require,module,exports){
"use strict";function textRemove(){this.textContent=""}function textConstant(t){return function(){this.textContent=t}}function textFunction(t){return function(){var e=t.apply(this,arguments);this.textContent=null==e?"":e}}function _default(t){return arguments.length?this.each(null==t?textRemove:("function"==typeof t?textFunction:textConstant)(t)):this.node().textContent}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],178:[function(require,module,exports){
"use strict";function none(){}function _default(e){return null==e?none:function(){return this.querySelector(e)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],179:[function(require,module,exports){
"use strict";function empty(){return[]}function _default(e){return null==e?empty:function(){return this.querySelectorAll(e)}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],180:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _on=require("./selection/on");function _default(){for(var e,t=_on.event;e=t.sourceEvent;)t=e;return t}

},{"./selection/on":166}],181:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _sourceEvent=_interopRequireDefault(require("./sourceEvent")),_point=_interopRequireDefault(require("./point"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t,r){arguments.length<3&&(r=t,t=(0,_sourceEvent.default)().changedTouches);for(var u,n=0,i=t?t.length:0;n<i;++n)if((u=t[n]).identifier===r)return(0,_point.default)(e,u);return null}

},{"./point":142,"./sourceEvent":180}],182:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _sourceEvent=_interopRequireDefault(require("./sourceEvent")),_point=_interopRequireDefault(require("./point"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e,t){null==t&&(t=(0,_sourceEvent.default)().touches);for(var r=0,u=t?t.length:0,n=new Array(u);r<u;++r)n[r]=(0,_point.default)(e,t[r]);return n}

},{"./point":142,"./sourceEvent":180}],183:[function(require,module,exports){
"use strict";function _default(e){return e.ownerDocument&&e.ownerDocument.defaultView||e.document&&e||e.defaultView}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],184:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=defaultLocale,exports.utcParse=exports.utcFormat=exports.timeParse=exports.timeFormat=void 0;var locale,timeFormat,timeParse,utcFormat,utcParse,_locale=_interopRequireDefault(require("./locale.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function defaultLocale(e){return locale=(0,_locale.default)(e),exports.timeFormat=timeFormat=locale.format,exports.timeParse=timeParse=locale.parse,exports.utcFormat=utcFormat=locale.utcFormat,exports.utcParse=utcParse=locale.utcParse,locale}exports.timeFormat=timeFormat,exports.timeParse=timeParse,exports.utcFormat=utcFormat,exports.utcParse=utcParse,defaultLocale({dateTime:"%x, %X",date:"%-m/%-d/%Y",time:"%-I:%M:%S %p",periods:["AM","PM"],days:["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],shortDays:["Sun","Mon","Tue","Wed","Thu","Fri","Sat"],months:["January","February","March","April","May","June","July","August","September","October","November","December"],shortMonths:["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]});

},{"./locale.js":188}],185:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"timeFormatDefaultLocale",{enumerable:!0,get:function(){return _defaultLocale.default}}),Object.defineProperty(exports,"timeFormat",{enumerable:!0,get:function(){return _defaultLocale.timeFormat}}),Object.defineProperty(exports,"timeParse",{enumerable:!0,get:function(){return _defaultLocale.timeParse}}),Object.defineProperty(exports,"utcFormat",{enumerable:!0,get:function(){return _defaultLocale.utcFormat}}),Object.defineProperty(exports,"utcParse",{enumerable:!0,get:function(){return _defaultLocale.utcParse}}),Object.defineProperty(exports,"timeFormatLocale",{enumerable:!0,get:function(){return _locale.default}}),Object.defineProperty(exports,"isoFormat",{enumerable:!0,get:function(){return _isoFormat.default}}),Object.defineProperty(exports,"isoParse",{enumerable:!0,get:function(){return _isoParse.default}});var _defaultLocale=_interopRequireWildcard(require("./defaultLocale.js")),_locale=_interopRequireDefault(require("./locale.js")),_isoFormat=_interopRequireDefault(require("./isoFormat.js")),_isoParse=_interopRequireDefault(require("./isoParse.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var t=_getRequireWildcardCache();if(t&&t.has(e))return t.get(e);var r={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var a=o?Object.getOwnPropertyDescriptor(e,u):null;a&&(a.get||a.set)?Object.defineProperty(r,u,a):r[u]=e[u]}return r.default=e,t&&t.set(e,r),r}

},{"./defaultLocale.js":184,"./isoFormat.js":186,"./isoParse.js":187,"./locale.js":188}],186:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=exports.isoSpecifier=void 0;var _defaultLocale=require("./defaultLocale.js"),isoSpecifier="%Y-%m-%dT%H:%M:%S.%LZ";function formatIsoNative(e){return e.toISOString()}exports.isoSpecifier=isoSpecifier;var formatIso=Date.prototype.toISOString?formatIsoNative:(0,_defaultLocale.utcFormat)(isoSpecifier),_default=formatIso;exports.default=_default;

},{"./defaultLocale.js":184}],187:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=void 0;var _isoFormat=require("./isoFormat.js"),_defaultLocale=require("./defaultLocale.js");function parseIsoNative(e){var a=new Date(e);return isNaN(a)?null:a}var parseIso=+new Date("2000-01-01T00:00:00.000Z")?parseIsoNative:(0,_defaultLocale.utcParse)(_isoFormat.isoSpecifier),_default=parseIso;exports.default=_default;

},{"./defaultLocale.js":184,"./isoFormat.js":186}],188:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=formatLocale;var _d3Time=require("d3-time");function localDate(e){if(0<=e.y&&e.y<100){var r=new Date(-1,e.m,e.d,e.H,e.M,e.S,e.L);return r.setFullYear(e.y),r}return new Date(e.y,e.m,e.d,e.H,e.M,e.S,e.L)}function utcDate(e){if(0<=e.y&&e.y<100){var r=new Date(Date.UTC(-1,e.m,e.d,e.H,e.M,e.S,e.L));return r.setUTCFullYear(e.y),r}return new Date(Date.UTC(e.y,e.m,e.d,e.H,e.M,e.S,e.L))}function newDate(e,r,t){return{y:e,m:r,d:t,H:0,M:0,S:0,L:0}}function formatLocale(e){var r=e.dateTime,t=e.date,n=e.time,a=e.periods,u=e.days,o=e.shortDays,i=e.months,c=e.shortMonths,m=formatRe(a),f=formatLookup(a),s=formatRe(u),d=formatLookup(u),l=formatRe(o),T=formatLookup(o),p=formatRe(i),y=formatLookup(i),g=formatRe(c),M=formatLookup(c),U={a:function(e){return o[e.getDay()]},A:function(e){return u[e.getDay()]},b:function(e){return c[e.getMonth()]},B:function(e){return i[e.getMonth()]},c:null,d:formatDayOfMonth,e:formatDayOfMonth,f:formatMicroseconds,H:formatHour24,I:formatHour12,j:formatDayOfYear,L:formatMilliseconds,m:formatMonthNumber,M:formatMinutes,p:function(e){return a[+(e.getHours()>=12)]},q:function(e){return 1+~~(e.getMonth()/3)},Q:formatUnixTimestamp,s:formatUnixTimestampSeconds,S:formatSeconds,u:formatWeekdayNumberMonday,U:formatWeekNumberSunday,V:formatWeekNumberISO,w:formatWeekdayNumberSunday,W:formatWeekNumberMonday,x:null,X:null,y:formatYear,Y:formatFullYear,Z:formatZone,"%":formatLiteralPercent},h={a:function(e){return o[e.getUTCDay()]},A:function(e){return u[e.getUTCDay()]},b:function(e){return c[e.getUTCMonth()]},B:function(e){return i[e.getUTCMonth()]},c:null,d:formatUTCDayOfMonth,e:formatUTCDayOfMonth,f:formatUTCMicroseconds,H:formatUTCHour24,I:formatUTCHour12,j:formatUTCDayOfYear,L:formatUTCMilliseconds,m:formatUTCMonthNumber,M:formatUTCMinutes,p:function(e){return a[+(e.getUTCHours()>=12)]},q:function(e){return 1+~~(e.getUTCMonth()/3)},Q:formatUnixTimestamp,s:formatUnixTimestampSeconds,S:formatUTCSeconds,u:formatUTCWeekdayNumberMonday,U:formatUTCWeekNumberSunday,V:formatUTCWeekNumberISO,w:formatUTCWeekdayNumberSunday,W:formatUTCWeekNumberMonday,x:null,X:null,y:formatUTCYear,Y:formatUTCFullYear,Z:formatUTCZone,"%":formatLiteralPercent},C={a:function(e,r,t){var n=l.exec(r.slice(t));return n?(e.w=T[n[0].toLowerCase()],t+n[0].length):-1},A:function(e,r,t){var n=s.exec(r.slice(t));return n?(e.w=d[n[0].toLowerCase()],t+n[0].length):-1},b:function(e,r,t){var n=g.exec(r.slice(t));return n?(e.m=M[n[0].toLowerCase()],t+n[0].length):-1},B:function(e,r,t){var n=p.exec(r.slice(t));return n?(e.m=y[n[0].toLowerCase()],t+n[0].length):-1},c:function(e,t,n){return v(e,r,t,n)},d:parseDayOfMonth,e:parseDayOfMonth,f:parseMicroseconds,H:parseHour24,I:parseHour24,j:parseDayOfYear,L:parseMilliseconds,m:parseMonthNumber,M:parseMinutes,p:function(e,r,t){var n=m.exec(r.slice(t));return n?(e.p=f[n[0].toLowerCase()],t+n[0].length):-1},q:parseQuarter,Q:parseUnixTimestamp,s:parseUnixTimestampSeconds,S:parseSeconds,u:parseWeekdayNumberMonday,U:parseWeekNumberSunday,V:parseWeekNumberISO,w:parseWeekdayNumberSunday,W:parseWeekNumberMonday,x:function(e,r,n){return v(e,t,r,n)},X:function(e,r,t){return v(e,n,r,t)},y:parseYear,Y:parseFullYear,Z:parseZone,"%":parseLiteralPercent};function D(e,r){return function(t){var n,a,u,o=[],i=-1,c=0,m=e.length;for(t instanceof Date||(t=new Date(+t));++i<m;)37===e.charCodeAt(i)&&(o.push(e.slice(c,i)),null!=(a=pads[n=e.charAt(++i)])?n=e.charAt(++i):a="e"===n?" ":"0",(u=r[n])&&(n=u(t,a)),o.push(n),c=i+1);return o.push(e.slice(c,i)),o.join("")}}function b(e,r){return function(t){var n,a,u=newDate(1900,void 0,1);if(v(u,e,t+="",0)!=t.length)return null;if("Q"in u)return new Date(u.Q);if("s"in u)return new Date(1e3*u.s+("L"in u?u.L:0));if(!r||"Z"in u||(u.Z=0),"p"in u&&(u.H=u.H%12+12*u.p),void 0===u.m&&(u.m="q"in u?u.q:0),"V"in u){if(u.V<1||u.V>53)return null;"w"in u||(u.w=1),"Z"in u?(a=(n=utcDate(newDate(u.y,0,1))).getUTCDay(),n=a>4||0===a?_d3Time.utcMonday.ceil(n):(0,_d3Time.utcMonday)(n),n=_d3Time.utcDay.offset(n,7*(u.V-1)),u.y=n.getUTCFullYear(),u.m=n.getUTCMonth(),u.d=n.getUTCDate()+(u.w+6)%7):(a=(n=localDate(newDate(u.y,0,1))).getDay(),n=a>4||0===a?_d3Time.timeMonday.ceil(n):(0,_d3Time.timeMonday)(n),n=_d3Time.timeDay.offset(n,7*(u.V-1)),u.y=n.getFullYear(),u.m=n.getMonth(),u.d=n.getDate()+(u.w+6)%7)}else("W"in u||"U"in u)&&("w"in u||(u.w="u"in u?u.u%7:"W"in u?1:0),a="Z"in u?utcDate(newDate(u.y,0,1)).getUTCDay():localDate(newDate(u.y,0,1)).getDay(),u.m=0,u.d="W"in u?(u.w+6)%7+7*u.W-(a+5)%7:u.w+7*u.U-(a+6)%7);return"Z"in u?(u.H+=u.Z/100|0,u.M+=u.Z%100,utcDate(u)):localDate(u)}}function v(e,r,t,n){for(var a,u,o=0,i=r.length,c=t.length;o<i;){if(n>=c)return-1;if(37===(a=r.charCodeAt(o++))){if(a=r.charAt(o++),!(u=C[a in pads?r.charAt(o++):a])||(n=u(e,t,n))<0)return-1}else if(a!=t.charCodeAt(n++))return-1}return n}return U.x=D(t,U),U.X=D(n,U),U.c=D(r,U),h.x=D(t,h),h.X=D(n,h),h.c=D(r,h),{format:function(e){var r=D(e+="",U);return r.toString=function(){return e},r},parse:function(e){var r=b(e+="",!1);return r.toString=function(){return e},r},utcFormat:function(e){var r=D(e+="",h);return r.toString=function(){return e},r},utcParse:function(e){var r=b(e+="",!0);return r.toString=function(){return e},r}}}var pads={"-":"",_:" ",0:"0"},numberRe=/^\s*\d+/,percentRe=/^%/,requoteRe=/[\\^$*+?|[\]().{}]/g;function pad(e,r,t){var n=e<0?"-":"",a=(n?-e:e)+"",u=a.length;return n+(u<t?new Array(t-u+1).join(r)+a:a)}function requote(e){return e.replace(requoteRe,"\\$&")}function formatRe(e){return new RegExp("^(?:"+e.map(requote).join("|")+")","i")}function formatLookup(e){for(var r={},t=-1,n=e.length;++t<n;)r[e[t].toLowerCase()]=t;return r}function parseWeekdayNumberSunday(e,r,t){var n=numberRe.exec(r.slice(t,t+1));return n?(e.w=+n[0],t+n[0].length):-1}function parseWeekdayNumberMonday(e,r,t){var n=numberRe.exec(r.slice(t,t+1));return n?(e.u=+n[0],t+n[0].length):-1}function parseWeekNumberSunday(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.U=+n[0],t+n[0].length):-1}function parseWeekNumberISO(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.V=+n[0],t+n[0].length):-1}function parseWeekNumberMonday(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.W=+n[0],t+n[0].length):-1}function parseFullYear(e,r,t){var n=numberRe.exec(r.slice(t,t+4));return n?(e.y=+n[0],t+n[0].length):-1}function parseYear(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.y=+n[0]+(+n[0]>68?1900:2e3),t+n[0].length):-1}function parseZone(e,r,t){var n=/^(Z)|([+-]\d\d)(?::?(\d\d))?/.exec(r.slice(t,t+6));return n?(e.Z=n[1]?0:-(n[2]+(n[3]||"00")),t+n[0].length):-1}function parseQuarter(e,r,t){var n=numberRe.exec(r.slice(t,t+1));return n?(e.q=3*n[0]-3,t+n[0].length):-1}function parseMonthNumber(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.m=n[0]-1,t+n[0].length):-1}function parseDayOfMonth(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.d=+n[0],t+n[0].length):-1}function parseDayOfYear(e,r,t){var n=numberRe.exec(r.slice(t,t+3));return n?(e.m=0,e.d=+n[0],t+n[0].length):-1}function parseHour24(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.H=+n[0],t+n[0].length):-1}function parseMinutes(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.M=+n[0],t+n[0].length):-1}function parseSeconds(e,r,t){var n=numberRe.exec(r.slice(t,t+2));return n?(e.S=+n[0],t+n[0].length):-1}function parseMilliseconds(e,r,t){var n=numberRe.exec(r.slice(t,t+3));return n?(e.L=+n[0],t+n[0].length):-1}function parseMicroseconds(e,r,t){var n=numberRe.exec(r.slice(t,t+6));return n?(e.L=Math.floor(n[0]/1e3),t+n[0].length):-1}function parseLiteralPercent(e,r,t){var n=percentRe.exec(r.slice(t,t+1));return n?t+n[0].length:-1}function parseUnixTimestamp(e,r,t){var n=numberRe.exec(r.slice(t));return n?(e.Q=+n[0],t+n[0].length):-1}function parseUnixTimestampSeconds(e,r,t){var n=numberRe.exec(r.slice(t));return n?(e.s=+n[0],t+n[0].length):-1}function formatDayOfMonth(e,r){return pad(e.getDate(),r,2)}function formatHour24(e,r){return pad(e.getHours(),r,2)}function formatHour12(e,r){return pad(e.getHours()%12||12,r,2)}function formatDayOfYear(e,r){return pad(1+_d3Time.timeDay.count((0,_d3Time.timeYear)(e),e),r,3)}function formatMilliseconds(e,r){return pad(e.getMilliseconds(),r,3)}function formatMicroseconds(e,r){return formatMilliseconds(e,r)+"000"}function formatMonthNumber(e,r){return pad(e.getMonth()+1,r,2)}function formatMinutes(e,r){return pad(e.getMinutes(),r,2)}function formatSeconds(e,r){return pad(e.getSeconds(),r,2)}function formatWeekdayNumberMonday(e){var r=e.getDay();return 0===r?7:r}function formatWeekNumberSunday(e,r){return pad(_d3Time.timeSunday.count((0,_d3Time.timeYear)(e)-1,e),r,2)}function formatWeekNumberISO(e,r){var t=e.getDay();return e=t>=4||0===t?(0,_d3Time.timeThursday)(e):_d3Time.timeThursday.ceil(e),pad(_d3Time.timeThursday.count((0,_d3Time.timeYear)(e),e)+(4===(0,_d3Time.timeYear)(e).getDay()),r,2)}function formatWeekdayNumberSunday(e){return e.getDay()}function formatWeekNumberMonday(e,r){return pad(_d3Time.timeMonday.count((0,_d3Time.timeYear)(e)-1,e),r,2)}function formatYear(e,r){return pad(e.getFullYear()%100,r,2)}function formatFullYear(e,r){return pad(e.getFullYear()%1e4,r,4)}function formatZone(e){var r=e.getTimezoneOffset();return(r>0?"-":(r*=-1,"+"))+pad(r/60|0,"0",2)+pad(r%60,"0",2)}function formatUTCDayOfMonth(e,r){return pad(e.getUTCDate(),r,2)}function formatUTCHour24(e,r){return pad(e.getUTCHours(),r,2)}function formatUTCHour12(e,r){return pad(e.getUTCHours()%12||12,r,2)}function formatUTCDayOfYear(e,r){return pad(1+_d3Time.utcDay.count((0,_d3Time.utcYear)(e),e),r,3)}function formatUTCMilliseconds(e,r){return pad(e.getUTCMilliseconds(),r,3)}function formatUTCMicroseconds(e,r){return formatUTCMilliseconds(e,r)+"000"}function formatUTCMonthNumber(e,r){return pad(e.getUTCMonth()+1,r,2)}function formatUTCMinutes(e,r){return pad(e.getUTCMinutes(),r,2)}function formatUTCSeconds(e,r){return pad(e.getUTCSeconds(),r,2)}function formatUTCWeekdayNumberMonday(e){var r=e.getUTCDay();return 0===r?7:r}function formatUTCWeekNumberSunday(e,r){return pad(_d3Time.utcSunday.count((0,_d3Time.utcYear)(e)-1,e),r,2)}function formatUTCWeekNumberISO(e,r){var t=e.getUTCDay();return e=t>=4||0===t?(0,_d3Time.utcThursday)(e):_d3Time.utcThursday.ceil(e),pad(_d3Time.utcThursday.count((0,_d3Time.utcYear)(e),e)+(4===(0,_d3Time.utcYear)(e).getUTCDay()),r,2)}function formatUTCWeekdayNumberSunday(e){return e.getUTCDay()}function formatUTCWeekNumberMonday(e,r){return pad(_d3Time.utcMonday.count((0,_d3Time.utcYear)(e)-1,e),r,2)}function formatUTCYear(e,r){return pad(e.getUTCFullYear()%100,r,2)}function formatUTCFullYear(e,r){return pad(e.getUTCFullYear()%1e4,r,4)}function formatUTCZone(){return"+0000"}function formatLiteralPercent(){return"%"}function formatUnixTimestamp(e){return+e}function formatUnixTimestampSeconds(e){return Math.floor(+e/1e3)}

},{"d3-time":192}],189:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.days=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var day=(0,_interval.default)(function(e){e.setHours(0,0,0,0)},function(e,t){e.setDate(e.getDate()+t)},function(e,t){return(t-e-(t.getTimezoneOffset()-e.getTimezoneOffset())*_duration.durationMinute)/_duration.durationDay},function(e){return e.getDate()-1}),_default=day;exports.default=_default;var days=day.range;exports.days=days;

},{"./duration.js":190,"./interval.js":193}],190:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.durationWeek=exports.durationDay=exports.durationHour=exports.durationMinute=exports.durationSecond=void 0;var durationSecond=1e3;exports.durationSecond=durationSecond;var durationMinute=6e4;exports.durationMinute=durationMinute;var durationHour=36e5;exports.durationHour=durationHour;var durationDay=864e5;exports.durationDay=durationDay;var durationWeek=6048e5;exports.durationWeek=durationWeek;

},{}],191:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.hours=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var hour=(0,_interval.default)(function(e){e.setTime(e-e.getMilliseconds()-e.getSeconds()*_duration.durationSecond-e.getMinutes()*_duration.durationMinute)},function(e,r){e.setTime(+e+r*_duration.durationHour)},function(e,r){return(r-e)/_duration.durationHour},function(e){return e.getHours()}),_default=hour;exports.default=_default;var hours=hour.range;exports.hours=hours;

},{"./duration.js":190,"./interval.js":193}],192:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"timeInterval",{enumerable:!0,get:function(){return _interval.default}}),Object.defineProperty(exports,"timeMillisecond",{enumerable:!0,get:function(){return _millisecond.default}}),Object.defineProperty(exports,"timeMilliseconds",{enumerable:!0,get:function(){return _millisecond.milliseconds}}),Object.defineProperty(exports,"utcMillisecond",{enumerable:!0,get:function(){return _millisecond.default}}),Object.defineProperty(exports,"utcMilliseconds",{enumerable:!0,get:function(){return _millisecond.milliseconds}}),Object.defineProperty(exports,"timeSecond",{enumerable:!0,get:function(){return _second.default}}),Object.defineProperty(exports,"timeSeconds",{enumerable:!0,get:function(){return _second.seconds}}),Object.defineProperty(exports,"utcSecond",{enumerable:!0,get:function(){return _second.default}}),Object.defineProperty(exports,"utcSeconds",{enumerable:!0,get:function(){return _second.seconds}}),Object.defineProperty(exports,"timeMinute",{enumerable:!0,get:function(){return _minute.default}}),Object.defineProperty(exports,"timeMinutes",{enumerable:!0,get:function(){return _minute.minutes}}),Object.defineProperty(exports,"timeHour",{enumerable:!0,get:function(){return _hour.default}}),Object.defineProperty(exports,"timeHours",{enumerable:!0,get:function(){return _hour.hours}}),Object.defineProperty(exports,"timeDay",{enumerable:!0,get:function(){return _day.default}}),Object.defineProperty(exports,"timeDays",{enumerable:!0,get:function(){return _day.days}}),Object.defineProperty(exports,"timeWeek",{enumerable:!0,get:function(){return _week.sunday}}),Object.defineProperty(exports,"timeWeeks",{enumerable:!0,get:function(){return _week.sundays}}),Object.defineProperty(exports,"timeSunday",{enumerable:!0,get:function(){return _week.sunday}}),Object.defineProperty(exports,"timeSundays",{enumerable:!0,get:function(){return _week.sundays}}),Object.defineProperty(exports,"timeMonday",{enumerable:!0,get:function(){return _week.monday}}),Object.defineProperty(exports,"timeMondays",{enumerable:!0,get:function(){return _week.mondays}}),Object.defineProperty(exports,"timeTuesday",{enumerable:!0,get:function(){return _week.tuesday}}),Object.defineProperty(exports,"timeTuesdays",{enumerable:!0,get:function(){return _week.tuesdays}}),Object.defineProperty(exports,"timeWednesday",{enumerable:!0,get:function(){return _week.wednesday}}),Object.defineProperty(exports,"timeWednesdays",{enumerable:!0,get:function(){return _week.wednesdays}}),Object.defineProperty(exports,"timeThursday",{enumerable:!0,get:function(){return _week.thursday}}),Object.defineProperty(exports,"timeThursdays",{enumerable:!0,get:function(){return _week.thursdays}}),Object.defineProperty(exports,"timeFriday",{enumerable:!0,get:function(){return _week.friday}}),Object.defineProperty(exports,"timeFridays",{enumerable:!0,get:function(){return _week.fridays}}),Object.defineProperty(exports,"timeSaturday",{enumerable:!0,get:function(){return _week.saturday}}),Object.defineProperty(exports,"timeSaturdays",{enumerable:!0,get:function(){return _week.saturdays}}),Object.defineProperty(exports,"timeMonth",{enumerable:!0,get:function(){return _month.default}}),Object.defineProperty(exports,"timeMonths",{enumerable:!0,get:function(){return _month.months}}),Object.defineProperty(exports,"timeYear",{enumerable:!0,get:function(){return _year.default}}),Object.defineProperty(exports,"timeYears",{enumerable:!0,get:function(){return _year.years}}),Object.defineProperty(exports,"utcMinute",{enumerable:!0,get:function(){return _utcMinute.default}}),Object.defineProperty(exports,"utcMinutes",{enumerable:!0,get:function(){return _utcMinute.utcMinutes}}),Object.defineProperty(exports,"utcHour",{enumerable:!0,get:function(){return _utcHour.default}}),Object.defineProperty(exports,"utcHours",{enumerable:!0,get:function(){return _utcHour.utcHours}}),Object.defineProperty(exports,"utcDay",{enumerable:!0,get:function(){return _utcDay.default}}),Object.defineProperty(exports,"utcDays",{enumerable:!0,get:function(){return _utcDay.utcDays}}),Object.defineProperty(exports,"utcWeek",{enumerable:!0,get:function(){return _utcWeek.utcSunday}}),Object.defineProperty(exports,"utcWeeks",{enumerable:!0,get:function(){return _utcWeek.utcSundays}}),Object.defineProperty(exports,"utcSunday",{enumerable:!0,get:function(){return _utcWeek.utcSunday}}),Object.defineProperty(exports,"utcSundays",{enumerable:!0,get:function(){return _utcWeek.utcSundays}}),Object.defineProperty(exports,"utcMonday",{enumerable:!0,get:function(){return _utcWeek.utcMonday}}),Object.defineProperty(exports,"utcMondays",{enumerable:!0,get:function(){return _utcWeek.utcMondays}}),Object.defineProperty(exports,"utcTuesday",{enumerable:!0,get:function(){return _utcWeek.utcTuesday}}),Object.defineProperty(exports,"utcTuesdays",{enumerable:!0,get:function(){return _utcWeek.utcTuesdays}}),Object.defineProperty(exports,"utcWednesday",{enumerable:!0,get:function(){return _utcWeek.utcWednesday}}),Object.defineProperty(exports,"utcWednesdays",{enumerable:!0,get:function(){return _utcWeek.utcWednesdays}}),Object.defineProperty(exports,"utcThursday",{enumerable:!0,get:function(){return _utcWeek.utcThursday}}),Object.defineProperty(exports,"utcThursdays",{enumerable:!0,get:function(){return _utcWeek.utcThursdays}}),Object.defineProperty(exports,"utcFriday",{enumerable:!0,get:function(){return _utcWeek.utcFriday}}),Object.defineProperty(exports,"utcFridays",{enumerable:!0,get:function(){return _utcWeek.utcFridays}}),Object.defineProperty(exports,"utcSaturday",{enumerable:!0,get:function(){return _utcWeek.utcSaturday}}),Object.defineProperty(exports,"utcSaturdays",{enumerable:!0,get:function(){return _utcWeek.utcSaturdays}}),Object.defineProperty(exports,"utcMonth",{enumerable:!0,get:function(){return _utcMonth.default}}),Object.defineProperty(exports,"utcMonths",{enumerable:!0,get:function(){return _utcMonth.utcMonths}}),Object.defineProperty(exports,"utcYear",{enumerable:!0,get:function(){return _utcYear.default}}),Object.defineProperty(exports,"utcYears",{enumerable:!0,get:function(){return _utcYear.utcYears}});var _interval=_interopRequireDefault(require("./interval.js")),_millisecond=_interopRequireWildcard(require("./millisecond.js")),_second=_interopRequireWildcard(require("./second.js")),_minute=_interopRequireWildcard(require("./minute.js")),_hour=_interopRequireWildcard(require("./hour.js")),_day=_interopRequireWildcard(require("./day.js")),_week=require("./week.js"),_month=_interopRequireWildcard(require("./month.js")),_year=_interopRequireWildcard(require("./year.js")),_utcMinute=_interopRequireWildcard(require("./utcMinute.js")),_utcHour=_interopRequireWildcard(require("./utcHour.js")),_utcDay=_interopRequireWildcard(require("./utcDay.js")),_utcWeek=require("./utcWeek.js"),_utcMonth=_interopRequireWildcard(require("./utcMonth.js")),_utcYear=_interopRequireWildcard(require("./utcYear.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var t=_getRequireWildcardCache();if(t&&t.has(e))return t.get(e);var r={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var u in e)if(Object.prototype.hasOwnProperty.call(e,u)){var i=n?Object.getOwnPropertyDescriptor(e,u):null;i&&(i.get||i.set)?Object.defineProperty(r,u,i):r[u]=e[u]}return r.default=e,t&&t.set(e,r),r}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./day.js":189,"./hour.js":191,"./interval.js":193,"./millisecond.js":194,"./minute.js":195,"./month.js":196,"./second.js":197,"./utcDay.js":198,"./utcHour.js":199,"./utcMinute.js":200,"./utcMonth.js":201,"./utcWeek.js":202,"./utcYear.js":203,"./week.js":204,"./year.js":205}],193:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=newInterval;var t0=new Date,t1=new Date;function newInterval(t,e,n,r){function o(e){return t(e=0===arguments.length?new Date:new Date(+e)),e}return o.floor=function(e){return t(e=new Date(+e)),e},o.ceil=function(n){return t(n=new Date(n-1)),e(n,1),t(n),n},o.round=function(t){var e=o(t),n=o.ceil(t);return t-e<n-t?e:n},o.offset=function(t,n){return e(t=new Date(+t),null==n?1:Math.floor(n)),t},o.range=function(n,r,u){var f,i=[];if(n=o.ceil(n),u=null==u?1:Math.floor(u),!(n<r&&u>0))return i;do{i.push(f=new Date(+n)),e(n,u),t(n)}while(f<n&&n<r);return i},o.filter=function(n){return newInterval(function(e){if(e>=e)for(;t(e),!n(e);)e.setTime(e-1)},function(t,r){if(t>=t)if(r<0)for(;++r<=0;)for(;e(t,-1),!n(t););else for(;--r>=0;)for(;e(t,1),!n(t););})},n&&(o.count=function(e,r){return t0.setTime(+e),t1.setTime(+r),t(t0),t(t1),Math.floor(n(t0,t1))},o.every=function(t){return t=Math.floor(t),isFinite(t)&&t>0?t>1?o.filter(r?function(e){return r(e)%t==0}:function(e){return o.count(0,e)%t==0}):o:null}),o}

},{}],194:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.milliseconds=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var millisecond=(0,_interval.default)(function(){},function(e,i){e.setTime(+e+i)},function(e,i){return i-e});millisecond.every=function(e){return e=Math.floor(e),isFinite(e)&&e>0?e>1?(0,_interval.default)(function(i){i.setTime(Math.floor(i/e)*e)},function(i,t){i.setTime(+i+t*e)},function(i,t){return(t-i)/e}):millisecond:null};var _default=millisecond;exports.default=_default;var milliseconds=millisecond.range;exports.milliseconds=milliseconds;

},{"./interval.js":193}],195:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.minutes=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var minute=(0,_interval.default)(function(e){e.setTime(e-e.getMilliseconds()-e.getSeconds()*_duration.durationSecond)},function(e,t){e.setTime(+e+t*_duration.durationMinute)},function(e,t){return(t-e)/_duration.durationMinute},function(e){return e.getMinutes()}),_default=minute;exports.default=_default;var minutes=minute.range;exports.minutes=minutes;

},{"./duration.js":190,"./interval.js":193}],196:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.months=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}var month=(0,_interval.default)(function(t){t.setDate(1),t.setHours(0,0,0,0)},function(t,e){t.setMonth(t.getMonth()+e)},function(t,e){return e.getMonth()-t.getMonth()+12*(e.getFullYear()-t.getFullYear())},function(t){return t.getMonth()}),_default=month;exports.default=_default;var months=month.range;exports.months=months;

},{"./interval.js":193}],197:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.seconds=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var second=(0,_interval.default)(function(e){e.setTime(e-e.getMilliseconds())},function(e,t){e.setTime(+e+t*_duration.durationSecond)},function(e,t){return(t-e)/_duration.durationSecond},function(e){return e.getUTCSeconds()}),_default=second;exports.default=_default;var seconds=second.range;exports.seconds=seconds;

},{"./duration.js":190,"./interval.js":193}],198:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcDays=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}var utcDay=(0,_interval.default)(function(t){t.setUTCHours(0,0,0,0)},function(t,e){t.setUTCDate(t.getUTCDate()+e)},function(t,e){return(e-t)/_duration.durationDay},function(t){return t.getUTCDate()-1}),_default=utcDay;exports.default=_default;var utcDays=utcDay.range;exports.utcDays=utcDays;

},{"./duration.js":190,"./interval.js":193}],199:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcHours=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(u){return u&&u.__esModule?u:{default:u}}var utcHour=(0,_interval.default)(function(u){u.setUTCMinutes(0,0,0)},function(u,t){u.setTime(+u+t*_duration.durationHour)},function(u,t){return(t-u)/_duration.durationHour},function(u){return u.getUTCHours()}),_default=utcHour;exports.default=_default;var utcHours=utcHour.range;exports.utcHours=utcHours;

},{"./duration.js":190,"./interval.js":193}],200:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcMinutes=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}var utcMinute=(0,_interval.default)(function(t){t.setUTCSeconds(0,0)},function(t,e){t.setTime(+t+e*_duration.durationMinute)},function(t,e){return(e-t)/_duration.durationMinute},function(t){return t.getUTCMinutes()}),_default=utcMinute;exports.default=_default;var utcMinutes=utcMinute.range;exports.utcMinutes=utcMinutes;

},{"./duration.js":190,"./interval.js":193}],201:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcMonths=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}var utcMonth=(0,_interval.default)(function(t){t.setUTCDate(1),t.setUTCHours(0,0,0,0)},function(t,e){t.setUTCMonth(t.getUTCMonth()+e)},function(t,e){return e.getUTCMonth()-t.getUTCMonth()+12*(e.getUTCFullYear()-t.getUTCFullYear())},function(t){return t.getUTCMonth()}),_default=utcMonth;exports.default=_default;var utcMonths=utcMonth.range;exports.utcMonths=utcMonths;

},{"./interval.js":193}],202:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcSaturdays=exports.utcFridays=exports.utcThursdays=exports.utcWednesdays=exports.utcTuesdays=exports.utcMondays=exports.utcSundays=exports.utcSaturday=exports.utcFriday=exports.utcThursday=exports.utcWednesday=exports.utcTuesday=exports.utcMonday=exports.utcSunday=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function utcWeekday(t){return(0,_interval.default)(function(u){u.setUTCDate(u.getUTCDate()-(u.getUTCDay()+7-t)%7),u.setUTCHours(0,0,0,0)},function(t,u){t.setUTCDate(t.getUTCDate()+7*u)},function(t,u){return(u-t)/_duration.durationWeek})}var utcSunday=utcWeekday(0);exports.utcSunday=utcSunday;var utcMonday=utcWeekday(1);exports.utcMonday=utcMonday;var utcTuesday=utcWeekday(2);exports.utcTuesday=utcTuesday;var utcWednesday=utcWeekday(3);exports.utcWednesday=utcWednesday;var utcThursday=utcWeekday(4);exports.utcThursday=utcThursday;var utcFriday=utcWeekday(5);exports.utcFriday=utcFriday;var utcSaturday=utcWeekday(6);exports.utcSaturday=utcSaturday;var utcSundays=utcSunday.range;exports.utcSundays=utcSundays;var utcMondays=utcMonday.range;exports.utcMondays=utcMondays;var utcTuesdays=utcTuesday.range;exports.utcTuesdays=utcTuesdays;var utcWednesdays=utcWednesday.range;exports.utcWednesdays=utcWednesdays;var utcThursdays=utcThursday.range;exports.utcThursdays=utcThursdays;var utcFridays=utcFriday.range;exports.utcFridays=utcFridays;var utcSaturdays=utcSaturday.range;exports.utcSaturdays=utcSaturdays;

},{"./duration.js":190,"./interval.js":193}],203:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.utcYears=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var utcYear=(0,_interval.default)(function(e){e.setUTCMonth(0,1),e.setUTCHours(0,0,0,0)},function(e,t){e.setUTCFullYear(e.getUTCFullYear()+t)},function(e,t){return t.getUTCFullYear()-e.getUTCFullYear()},function(e){return e.getUTCFullYear()});utcYear.every=function(e){return isFinite(e=Math.floor(e))&&e>0?(0,_interval.default)(function(t){t.setUTCFullYear(Math.floor(t.getUTCFullYear()/e)*e),t.setUTCMonth(0,1),t.setUTCHours(0,0,0,0)},function(t,r){t.setUTCFullYear(t.getUTCFullYear()+r*e)}):null};var _default=utcYear;exports.default=_default;var utcYears=utcYear.range;exports.utcYears=utcYears;

},{"./interval.js":193}],204:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.saturdays=exports.fridays=exports.thursdays=exports.wednesdays=exports.tuesdays=exports.mondays=exports.sundays=exports.saturday=exports.friday=exports.thursday=exports.wednesday=exports.tuesday=exports.monday=exports.sunday=void 0;var _interval=_interopRequireDefault(require("./interval.js")),_duration=require("./duration.js");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function weekday(e){return(0,_interval.default)(function(a){a.setDate(a.getDate()-(a.getDay()+7-e)%7),a.setHours(0,0,0,0)},function(e,a){e.setDate(e.getDate()+7*a)},function(e,a){return(a-e-(a.getTimezoneOffset()-e.getTimezoneOffset())*_duration.durationMinute)/_duration.durationWeek})}var sunday=weekday(0);exports.sunday=sunday;var monday=weekday(1);exports.monday=monday;var tuesday=weekday(2);exports.tuesday=tuesday;var wednesday=weekday(3);exports.wednesday=wednesday;var thursday=weekday(4);exports.thursday=thursday;var friday=weekday(5);exports.friday=friday;var saturday=weekday(6);exports.saturday=saturday;var sundays=sunday.range;exports.sundays=sundays;var mondays=monday.range;exports.mondays=mondays;var tuesdays=tuesday.range;exports.tuesdays=tuesdays;var wednesdays=wednesday.range;exports.wednesdays=wednesdays;var thursdays=thursday.range;exports.thursdays=thursdays;var fridays=friday.range;exports.fridays=fridays;var saturdays=saturday.range;exports.saturdays=saturdays;

},{"./duration.js":190,"./interval.js":193}],205:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.years=exports.default=void 0;var _interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var year=(0,_interval.default)(function(e){e.setMonth(0,1),e.setHours(0,0,0,0)},function(e,t){e.setFullYear(e.getFullYear()+t)},function(e,t){return t.getFullYear()-e.getFullYear()},function(e){return e.getFullYear()});year.every=function(e){return isFinite(e=Math.floor(e))&&e>0?(0,_interval.default)(function(t){t.setFullYear(Math.floor(t.getFullYear()/e)*e),t.setMonth(0,1),t.setHours(0,0,0,0)},function(t,r){t.setFullYear(t.getFullYear()+r*e)}):null};var _default=year;exports.default=_default;var years=year.range;exports.years=years;

},{"./interval.js":193}],206:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"now",{enumerable:!0,get:function(){return _timer.now}}),Object.defineProperty(exports,"timer",{enumerable:!0,get:function(){return _timer.timer}}),Object.defineProperty(exports,"timerFlush",{enumerable:!0,get:function(){return _timer.timerFlush}}),Object.defineProperty(exports,"timeout",{enumerable:!0,get:function(){return _timeout.default}}),Object.defineProperty(exports,"interval",{enumerable:!0,get:function(){return _interval.default}});var _timer=require("./timer.js"),_timeout=_interopRequireDefault(require("./timeout.js")),_interval=_interopRequireDefault(require("./interval.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./interval.js":207,"./timeout.js":208,"./timer.js":209}],207:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _timer=require("./timer.js");function _default(e,r,t){var u=new _timer.Timer,i=r;return null==r?(u.restart(e,r,t),u):(r=+r,t=null==t?(0,_timer.now)():+t,u.restart(function n(a){a+=i,u.restart(n,i+=r,t),e(a)},r,t),u)}

},{"./timer.js":209}],208:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _timer=require("./timer.js");function _default(e,t,r){var u=new _timer.Timer;return t=null==t?0:+t,u.restart(function(r){u.stop(),e(r+t)},t,r),u}

},{"./timer.js":209}],209:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.now=now,exports.Timer=Timer,exports.timer=timer,exports.timerFlush=timerFlush;var taskHead,taskTail,frame=0,timeout=0,interval=0,pokeDelay=1e3,clockLast=0,clockNow=0,clockSkew=0,clock="object"==typeof performance&&performance.now?performance:Date,setFrame="object"==typeof window&&window.requestAnimationFrame?window.requestAnimationFrame.bind(window):function(e){setTimeout(e,17)};function now(){return clockNow||(setFrame(clearNow),clockNow=clock.now()+clockSkew)}function clearNow(){clockNow=0}function Timer(){this._call=this._time=this._next=null}function timer(e,t,o){var l=new Timer;return l.restart(e,t,o),l}function timerFlush(){now(),++frame;for(var e,t=taskHead;t;)(e=clockNow-t._time)>=0&&t._call.call(null,e),t=t._next;--frame}function wake(){clockNow=(clockLast=clock.now())+clockSkew,frame=timeout=0;try{timerFlush()}finally{frame=0,nap(),clockNow=0}}function poke(){var e=clock.now(),t=e-clockLast;t>pokeDelay&&(clockSkew-=t,clockLast=e)}function nap(){for(var e,t,o=taskHead,l=1/0;o;)o._call?(l>o._time&&(l=o._time),e=o,o=o._next):(t=o._next,o._next=null,o=e?e._next=t:taskHead=t);taskTail=e,sleep(l)}function sleep(e){frame||(timeout&&(timeout=clearTimeout(timeout)),e-clockNow>24?(e<1/0&&(timeout=setTimeout(wake,e-clock.now()-clockSkew)),interval&&(interval=clearInterval(interval))):(interval||(clockLast=clock.now(),interval=setInterval(poke,pokeDelay)),frame=1,setFrame(wake)))}Timer.prototype=timer.prototype={constructor:Timer,restart:function(e,t,o){if("function"!=typeof e)throw new TypeError("callback is not a function");o=(null==o?now():+o)+(null==t?0:+t),this._next||taskTail===this||(taskTail?taskTail._next=this:taskHead=this,taskTail=this),this._call=e,this._time=o,sleep()},stop:function(){this._call&&(this._call=null,this._time=1/0,sleep())}};

},{}],210:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./transition/index.js"),_schedule=require("./transition/schedule.js"),root=[null];function _default(e,n){var t,r,i=e.__transition;if(i)for(r in n=null==n?null:n+"",i)if((t=i[r]).state>_schedule.SCHEDULED&&t.name===n)return new _index.Transition([[e]],root,n,+r);return null}

},{"./transition/index.js":223,"./transition/schedule.js":228}],211:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"transition",{enumerable:!0,get:function(){return _index2.default}}),Object.defineProperty(exports,"active",{enumerable:!0,get:function(){return _active.default}}),Object.defineProperty(exports,"interrupt",{enumerable:!0,get:function(){return _interrupt.default}}),require("./selection/index.js");var _index2=_interopRequireDefault(require("./transition/index.js")),_active=_interopRequireDefault(require("./active.js")),_interrupt=_interopRequireDefault(require("./interrupt.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./active.js":210,"./interrupt.js":212,"./selection/index.js":213,"./transition/index.js":223}],212:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./transition/schedule.js");function _default(e,t){var l,s,a,n=e.__transition,r=!0;if(n){for(a in t=null==t?null:t+"",n)(l=n[a]).name===t?(s=l.state>_schedule.STARTING&&l.state<_schedule.ENDING,l.state=_schedule.ENDED,l.timer.stop(),l.on.call(s?"interrupt":"cancel",e,e.__data__,l.index,l.group),delete n[a]):r=!1;r&&delete e.__transition}}

},{"./transition/schedule.js":228}],213:[function(require,module,exports){
"use strict";var _d3Selection=require("d3-selection"),_interrupt=_interopRequireDefault(require("./interrupt.js")),_transition=_interopRequireDefault(require("./transition.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}_d3Selection.selection.prototype.interrupt=_interrupt.default,_d3Selection.selection.prototype.transition=_transition.default;

},{"./interrupt.js":214,"./transition.js":215,"d3-selection":136}],214:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _interrupt=_interopRequireDefault(require("../interrupt.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function _default(e){return this.each(function(){(0,_interrupt.default)(this,e)})}

},{"../interrupt.js":212}],215:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("../transition/index.js"),_schedule=_interopRequireDefault(require("../transition/schedule.js")),_d3Ease=require("d3-ease"),_d3Timer=require("d3-timer");function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var defaultTiming={time:null,delay:0,duration:250,ease:_d3Ease.easeCubicInOut};function inherit(e,i){for(var n;!(n=e.__transition)||!(n=n[i]);)if(!(e=e.parentNode))return defaultTiming.time=(0,_d3Timer.now)(),defaultTiming;return n}function _default(e){var i,n;e instanceof _index.Transition?(i=e._id,e=e._name):(i=(0,_index.newId)(),(n=defaultTiming).time=(0,_d3Timer.now)(),e=null==e?null:e+"");for(var r=this._groups,t=r.length,u=0;u<t;++u)for(var a,d=r[u],l=d.length,s=0;s<l;++s)(a=d[s])&&(0,_schedule.default)(a,e,i,s,d,n||inherit(a,i));return new _index.Transition(r,this._parents,e,i)}

},{"../transition/index.js":223,"../transition/schedule.js":228,"d3-ease":63,"d3-timer":206}],216:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Interpolate=require("d3-interpolate"),_d3Selection=require("d3-selection"),_tween=require("./tween.js"),_interpolate=_interopRequireDefault(require("./interpolate.js"));function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function attrRemove(t){return function(){this.removeAttribute(t)}}function attrRemoveNS(t){return function(){this.removeAttributeNS(t.space,t.local)}}function attrConstant(t,e,r){var n,u,a=r+"";return function(){var i=this.getAttribute(t);return i===a?null:i===n?u:u=e(n=i,r)}}function attrConstantNS(t,e,r){var n,u,a=r+"";return function(){var i=this.getAttributeNS(t.space,t.local);return i===a?null:i===n?u:u=e(n=i,r)}}function attrFunction(t,e,r){var n,u,a;return function(){var i,o,l=r(this);if(null!=l)return(i=this.getAttribute(t))===(o=l+"")?null:i===n&&o===u?a:(u=o,a=e(n=i,l));this.removeAttribute(t)}}function attrFunctionNS(t,e,r){var n,u,a;return function(){var i,o,l=r(this);if(null!=l)return(i=this.getAttributeNS(t.space,t.local))===(o=l+"")?null:i===n&&o===u?a:(u=o,a=e(n=i,l));this.removeAttributeNS(t.space,t.local)}}function _default(t,e){var r=(0,_d3Selection.namespace)(t),n="transform"===r?_d3Interpolate.interpolateTransformSvg:_interpolate.default;return this.attrTween(t,"function"==typeof e?(r.local?attrFunctionNS:attrFunction)(r,n,(0,_tween.tweenValue)(this,"attr."+t,e)):null==e?(r.local?attrRemoveNS:attrRemove)(r):(r.local?attrConstantNS:attrConstant)(r,n,e))}

},{"./interpolate.js":224,"./tween.js":237,"d3-interpolate":95,"d3-selection":136}],217:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Selection=require("d3-selection");function attrInterpolate(t,e){return function(r){this.setAttribute(t,e.call(this,r))}}function attrInterpolateNS(t,e){return function(r){this.setAttributeNS(t.space,t.local,e.call(this,r))}}function attrTweenNS(t,e){var r,n;function a(){var a=e.apply(this,arguments);return a!==n&&(r=(n=a)&&attrInterpolateNS(t,a)),r}return a._value=e,a}function attrTween(t,e){var r,n;function a(){var a=e.apply(this,arguments);return a!==n&&(r=(n=a)&&attrInterpolate(t,a)),r}return a._value=e,a}function _default(t,e){var r="attr."+t;if(arguments.length<2)return(r=this.tween(r))&&r._value;if(null==e)return this.tween(r,null);if("function"!=typeof e)throw new Error;var n=(0,_d3Selection.namespace)(t);return this.tween(r,(n.local?attrTweenNS:attrTween)(n,e))}

},{"d3-selection":136}],218:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./schedule.js");function delayFunction(e,t){return function(){(0,_schedule.init)(this,e).delay=+t.apply(this,arguments)}}function delayConstant(e,t){return t=+t,function(){(0,_schedule.init)(this,e).delay=t}}function _default(e){var t=this._id;return arguments.length?this.each(("function"==typeof e?delayFunction:delayConstant)(t,e)):(0,_schedule.get)(this.node(),t).delay}

},{"./schedule.js":228}],219:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./schedule.js");function durationFunction(t,e){return function(){(0,_schedule.set)(this,t).duration=+e.apply(this,arguments)}}function durationConstant(t,e){return e=+e,function(){(0,_schedule.set)(this,t).duration=e}}function _default(t){var e=this._id;return arguments.length?this.each(("function"==typeof t?durationFunction:durationConstant)(e,t)):(0,_schedule.get)(this.node(),e).duration}

},{"./schedule.js":228}],220:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./schedule.js");function easeConstant(e,t){if("function"!=typeof t)throw new Error;return function(){(0,_schedule.set)(this,e).ease=t}}function _default(e){var t=this._id;return arguments.length?this.each(easeConstant(t,e)):(0,_schedule.get)(this.node(),t).ease}

},{"./schedule.js":228}],221:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./schedule.js");function _default(){var e,u,t=this,s=t._id,n=t.size();return new Promise(function(r,c){var i={value:c},a={value:function(){0==--n&&r()}};t.each(function(){var t=(0,_schedule.set)(this,s),n=t.on;n!==e&&((u=(e=n).copy())._.cancel.push(i),u._.interrupt.push(i),u._.end.push(a)),t.on=u})})}

},{"./schedule.js":228}],222:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Selection=require("d3-selection"),_index=require("./index.js");function _default(e){"function"!=typeof e&&(e=(0,_d3Selection.matcher)(e));for(var t=this._groups,r=t.length,n=new Array(r),i=0;i<r;++i)for(var _,a=t[i],s=a.length,o=n[i]=[],d=0;d<s;++d)(_=a[d])&&e.call(_,_.__data__,d,a)&&o.push(_);return new _index.Transition(n,this._parents,this._name,this._id)}

},{"./index.js":223,"d3-selection":136}],223:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.Transition=Transition,exports.default=transition,exports.newId=newId;var _d3Selection=require("d3-selection"),_attr=_interopRequireDefault(require("./attr.js")),_attrTween=_interopRequireDefault(require("./attrTween.js")),_delay=_interopRequireDefault(require("./delay.js")),_duration=_interopRequireDefault(require("./duration.js")),_ease=_interopRequireDefault(require("./ease.js")),_filter=_interopRequireDefault(require("./filter.js")),_merge=_interopRequireDefault(require("./merge.js")),_on=_interopRequireDefault(require("./on.js")),_remove=_interopRequireDefault(require("./remove.js")),_select=_interopRequireDefault(require("./select.js")),_selectAll=_interopRequireDefault(require("./selectAll.js")),_selection=_interopRequireDefault(require("./selection.js")),_style=_interopRequireDefault(require("./style.js")),_styleTween=_interopRequireDefault(require("./styleTween.js")),_text=_interopRequireDefault(require("./text.js")),_textTween=_interopRequireDefault(require("./textTween.js")),_transition=_interopRequireDefault(require("./transition.js")),_tween=_interopRequireDefault(require("./tween.js")),_end=_interopRequireDefault(require("./end.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}var id=0;function Transition(e,t,r,i){this._groups=e,this._parents=t,this._name=r,this._id=i}function transition(e){return(0,_d3Selection.selection)().transition(e)}function newId(){return++id}var selection_prototype=_d3Selection.selection.prototype;Transition.prototype=transition.prototype={constructor:Transition,select:_select.default,selectAll:_selectAll.default,filter:_filter.default,merge:_merge.default,selection:_selection.default,transition:_transition.default,call:selection_prototype.call,nodes:selection_prototype.nodes,node:selection_prototype.node,size:selection_prototype.size,empty:selection_prototype.empty,each:selection_prototype.each,on:_on.default,attr:_attr.default,attrTween:_attrTween.default,style:_style.default,styleTween:_styleTween.default,text:_text.default,textTween:_textTween.default,remove:_remove.default,tween:_tween.default,delay:_delay.default,duration:_duration.default,ease:_ease.default,end:_end.default};

},{"./attr.js":216,"./attrTween.js":217,"./delay.js":218,"./duration.js":219,"./ease.js":220,"./end.js":221,"./filter.js":222,"./merge.js":225,"./on.js":226,"./remove.js":227,"./select.js":229,"./selectAll.js":230,"./selection.js":231,"./style.js":232,"./styleTween.js":233,"./text.js":234,"./textTween.js":235,"./transition.js":236,"./tween.js":237,"d3-selection":136}],224:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Color=require("d3-color"),_d3Interpolate=require("d3-interpolate");function _default(e,t){var r;return("number"==typeof t?_d3Interpolate.interpolateNumber:t instanceof _d3Color.color?_d3Interpolate.interpolateRgb:(r=(0,_d3Color.color)(t))?(t=r,_d3Interpolate.interpolateRgb):_d3Interpolate.interpolateString)(e,t)}

},{"d3-color":46,"d3-interpolate":95}],225:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index.js");function _default(e){if(e._id!==this._id)throw new Error;for(var r=this._groups,t=e._groups,i=r.length,n=t.length,s=Math.min(i,n),a=new Array(i),o=0;o<s;++o)for(var _,d=r[o],u=t[o],h=d.length,f=a[o]=new Array(h),l=0;l<h;++l)(_=d[l]||u[l])&&(f[l]=_);for(;o<i;++o)a[o]=r[o];return new _index.Transition(a,this._parents,this._name,this._id)}

},{"./index.js":223}],226:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _schedule=require("./schedule.js");function start(e){return(e+"").trim().split(/^|\s+/).every(function(e){var t=e.indexOf(".");return t>=0&&(e=e.slice(0,t)),!e||"start"===e})}function onFunction(e,t,n){var r,u,s=start(t)?_schedule.init:_schedule.set;return function(){var i=s(this,e),o=i.on;o!==r&&(u=(r=o).copy()).on(t,n),i.on=u}}function _default(e,t){var n=this._id;return arguments.length<2?(0,_schedule.get)(this.node(),n).on.on(e):this.each(onFunction(n,e,t))}

},{"./schedule.js":228}],227:[function(require,module,exports){
"use strict";function removeFunction(e){return function(){var t=this.parentNode;for(var n in this.__transition)if(+n!==e)return;t&&t.removeChild(this)}}function _default(){return this.on("end.remove",removeFunction(this._id))}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],228:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.init=init,exports.set=set,exports.get=get,exports.ENDED=exports.ENDING=exports.RUNNING=exports.STARTED=exports.STARTING=exports.SCHEDULED=exports.CREATED=void 0;var _d3Dispatch=require("d3-dispatch"),_d3Timer=require("d3-timer"),emptyOn=(0,_d3Dispatch.dispatch)("start","end","cancel","interrupt"),emptyTween=[],CREATED=0;exports.CREATED=CREATED;var SCHEDULED=1;exports.SCHEDULED=SCHEDULED;var STARTING=2;exports.STARTING=STARTING;var STARTED=3;exports.STARTED=STARTED;var RUNNING=4;exports.RUNNING=RUNNING;var ENDING=5;exports.ENDING=ENDING;var ENDED=6;function _default(t,e,r,a,n,i){var o=t.__transition;if(o){if(r in o)return}else t.__transition={};create(t,r,{name:e,index:a,group:n,on:emptyOn,tween:emptyTween,time:i.time,delay:i.delay,duration:i.duration,ease:i.ease,timer:null,state:CREATED})}function init(t,e){var r=get(t,e);if(r.state>CREATED)throw new Error("too late; already scheduled");return r}function set(t,e){var r=get(t,e);if(r.state>STARTED)throw new Error("too late; already running");return r}function get(t,e){var r=t.__transition;if(!r||!(r=r[e]))throw new Error("transition not found");return r}function create(t,e,r){var a,n=t.__transition;function i(E){var d,D,l,u;if(r.state!==SCHEDULED)return s();for(d in n)if((u=n[d]).name===r.name){if(u.state===STARTED)return(0,_d3Timer.timeout)(i);u.state===RUNNING?(u.state=ENDED,u.timer.stop(),u.on.call("interrupt",t,t.__data__,u.index,u.group),delete n[d]):+d<e&&(u.state=ENDED,u.timer.stop(),u.on.call("cancel",t,t.__data__,u.index,u.group),delete n[d])}if((0,_d3Timer.timeout)(function(){r.state===STARTED&&(r.state=RUNNING,r.timer.restart(o,r.delay,r.time),o(E))}),r.state=STARTING,r.on.call("start",t,t.__data__,r.index,r.group),r.state===STARTING){for(r.state=STARTED,a=new Array(l=r.tween.length),d=0,D=-1;d<l;++d)(u=r.tween[d].value.call(t,t.__data__,r.index,r.group))&&(a[++D]=u);a.length=D+1}}function o(e){for(var n=e<r.duration?r.ease.call(null,e/r.duration):(r.timer.restart(s),r.state=ENDING,1),i=-1,o=a.length;++i<o;)a[i].call(t,n);r.state===ENDING&&(r.on.call("end",t,t.__data__,r.index,r.group),s())}function s(){for(var a in r.state=ENDED,r.timer.stop(),delete n[e],n)return;delete t.__transition}n[e]=r,r.timer=(0,_d3Timer.timer)(function(t){r.state=SCHEDULED,r.timer.restart(i,r.delay,r.time),r.delay<=t&&i(t-r.delay)},0,r.time)}exports.ENDED=ENDED;

},{"d3-dispatch":50,"d3-timer":206}],229:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Selection=require("d3-selection"),_index=require("./index.js"),_schedule=_interopRequireWildcard(require("./schedule.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in e)if(Object.prototype.hasOwnProperty.call(e,i)){var a=n?Object.getOwnPropertyDescriptor(e,i):null;a&&(a.get||a.set)?Object.defineProperty(t,i,a):t[i]=e[i]}return t.default=e,r&&r.set(e,t),t}function _default(e){var r=this._name,t=this._id;"function"!=typeof e&&(e=(0,_d3Selection.selector)(e));for(var n=this._groups,i=n.length,a=new Array(i),u=0;u<i;++u)for(var _,o,d=n[u],c=d.length,l=a[u]=new Array(c),f=0;f<c;++f)(_=d[f])&&(o=e.call(_,_.__data__,f,d))&&("__data__"in _&&(o.__data__=_.__data__),l[f]=o,(0,_schedule.default)(l[f],r,t,f,l,(0,_schedule.get)(_,t)));return new _index.Transition(a,this._parents,r,t)}

},{"./index.js":223,"./schedule.js":228,"d3-selection":136}],230:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Selection=require("d3-selection"),_index=require("./index.js"),_schedule=_interopRequireWildcard(require("./schedule.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in e)if(Object.prototype.hasOwnProperty.call(e,i)){var u=n?Object.getOwnPropertyDescriptor(e,i):null;u&&(u.get||u.set)?Object.defineProperty(t,i,u):t[i]=e[i]}return t.default=e,r&&r.set(e,t),t}function _default(e){var r=this._name,t=this._id;"function"!=typeof e&&(e=(0,_d3Selection.selectorAll)(e));for(var n=this._groups,i=n.length,u=[],o=[],a=0;a<i;++a)for(var l,c=n[a],d=c.length,f=0;f<d;++f)if(l=c[f]){for(var s,_=e.call(l,l.__data__,f,c),p=(0,_schedule.get)(l,t),h=0,g=_.length;h<g;++h)(s=_[h])&&(0,_schedule.default)(s,r,t,h,_,p);u.push(_),o.push(l)}return new _index.Transition(u,o,r,t)}

},{"./index.js":223,"./schedule.js":228,"d3-selection":136}],231:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Selection=require("d3-selection"),Selection=_d3Selection.selection.prototype.constructor;function _default(){return new Selection(this._groups,this._parents)}

},{"d3-selection":136}],232:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Interpolate=require("d3-interpolate"),_d3Selection=require("d3-selection"),_schedule=require("./schedule.js"),_tween=require("./tween.js"),_interpolate=_interopRequireDefault(require("./interpolate.js"));function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}function styleNull(e,t){var n,l,r;return function(){var s=(0,_d3Selection.style)(this,e),o=(this.style.removeProperty(e),(0,_d3Selection.style)(this,e));return s===o?null:s===n&&o===l?r:r=t(n=s,l=o)}}function styleRemove(e){return function(){this.style.removeProperty(e)}}function styleConstant(e,t,n){var l,r,s=n+"";return function(){var o=(0,_d3Selection.style)(this,e);return o===s?null:o===l?r:r=t(l=o,n)}}function styleFunction(e,t,n){var l,r,s;return function(){var o=(0,_d3Selection.style)(this,e),u=n(this),i=u+"";return null==u&&(this.style.removeProperty(e),i=u=(0,_d3Selection.style)(this,e)),o===i?null:o===l&&i===r?s:(r=i,s=t(l=o,u))}}function styleMaybeRemove(e,t){var n,l,r,s,o="style."+t,u="end."+o;return function(){var i=(0,_schedule.set)(this,e),a=i.on,y=null==i.value[o]?s||(s=styleRemove(t)):void 0;a===n&&r===y||(l=(n=a).copy()).on(u,r=y),i.on=l}}function _default(e,t,n){var l="transform"==(e+="")?_d3Interpolate.interpolateTransformCss:_interpolate.default;return null==t?this.styleTween(e,styleNull(e,l)).on("end.style."+e,styleRemove(e)):"function"==typeof t?this.styleTween(e,styleFunction(e,l,(0,_tween.tweenValue)(this,"style."+e,t))).each(styleMaybeRemove(this._id,e)):this.styleTween(e,styleConstant(e,l,t),n).on("end.style."+e,null)}

},{"./interpolate.js":224,"./schedule.js":228,"./tween.js":237,"d3-interpolate":95,"d3-selection":136}],233:[function(require,module,exports){
"use strict";function styleInterpolate(e,t,n){return function(r){this.style.setProperty(e,t.call(this,r),n)}}function styleTween(e,t,n){var r,l;function u(){var u=t.apply(this,arguments);return u!==l&&(r=(l=u)&&styleInterpolate(e,u,n)),r}return u._value=t,u}function _default(e,t,n){var r="style."+(e+="");if(arguments.length<2)return(r=this.tween(r))&&r._value;if(null==t)return this.tween(r,null);if("function"!=typeof t)throw new Error;return this.tween(r,styleTween(e,t,null==n?"":n))}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],234:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _tween=require("./tween.js");function textConstant(t){return function(){this.textContent=t}}function textFunction(t){return function(){var e=t(this);this.textContent=null==e?"":e}}function _default(t){return this.tween("text","function"==typeof t?textFunction((0,_tween.tweenValue)(this,"text",t)):textConstant(null==t?"":t+""))}

},{"./tween.js":237}],235:[function(require,module,exports){
"use strict";function textInterpolate(t){return function(e){this.textContent=t.call(this,e)}}function textTween(t){var e,n;function r(){var r=t.apply(this,arguments);return r!==n&&(e=(n=r)&&textInterpolate(r)),e}return r._value=t,r}function _default(t){var e="text";if(arguments.length<1)return(e=this.tween(e))&&e._value;if(null==t)return this.tween(e,null);if("function"!=typeof t)throw new Error;return this.tween(e,textTween(t))}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],236:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _index=require("./index.js"),_schedule=_interopRequireWildcard(require("./schedule.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},i=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var n in e)if(Object.prototype.hasOwnProperty.call(e,n)){var u=i?Object.getOwnPropertyDescriptor(e,n):null;u&&(u.get||u.set)?Object.defineProperty(t,n,u):t[n]=e[n]}return t.default=e,r&&r.set(e,t),t}function _default(){for(var e=this._name,r=this._id,t=(0,_index.newId)(),i=this._groups,n=i.length,u=0;u<n;++u)for(var a,d=i[u],o=d.length,l=0;l<o;++l)if(a=d[l]){var c=(0,_schedule.get)(a,r);(0,_schedule.default)(a,e,t,l,d,{time:c.time+c.delay+c.duration,delay:0,duration:c.duration,ease:c.ease})}return new _index.Transition(i,this._parents,e,t)}

},{"./index.js":223,"./schedule.js":228}],237:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default,exports.tweenValue=tweenValue;var _schedule=require("./schedule.js");function tweenRemove(e,t){var n,u;return function(){var r=(0,_schedule.set)(this,e),i=r.tween;if(i!==n)for(var a=0,l=(u=n=i).length;a<l;++a)if(u[a].name===t){(u=u.slice()).splice(a,1);break}r.tween=u}}function tweenFunction(e,t,n){var u,r;if("function"!=typeof n)throw new Error;return function(){var i=(0,_schedule.set)(this,e),a=i.tween;if(a!==u){r=(u=a).slice();for(var l={name:t,value:n},s=0,c=r.length;s<c;++s)if(r[s].name===t){r[s]=l;break}s===c&&r.push(l)}i.tween=r}}function _default(e,t){var n=this._id;if(e+="",arguments.length<2){for(var u,r=(0,_schedule.get)(this.node(),n).tween,i=0,a=r.length;i<a;++i)if((u=r[i]).name===e)return u.value;return null}return this.each((null==t?tweenRemove:tweenFunction)(n,e,t))}function tweenValue(e,t,n){var u=e._id;return e.each(function(){var e=(0,_schedule.set)(this,u);(e.value||(e.value={}))[t]=n.apply(this,arguments)}),function(e){return(0,_schedule.get)(e,u).value[t]}}

},{"./schedule.js":228}],238:[function(require,module,exports){
"use strict";function _default(e){return function(){return e}}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;

},{}],239:[function(require,module,exports){
"use strict";function ZoomEvent(t,e,o){this.target=t,this.type=e,this.transform=o}Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=ZoomEvent;

},{}],240:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"zoom",{enumerable:!0,get:function(){return _zoom.default}}),Object.defineProperty(exports,"zoomTransform",{enumerable:!0,get:function(){return _transform.default}}),Object.defineProperty(exports,"zoomIdentity",{enumerable:!0,get:function(){return _transform.identity}});var _zoom=_interopRequireDefault(require("./zoom.js")),_transform=_interopRequireWildcard(require("./transform.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var e=new WeakMap;return _getRequireWildcardCache=function(){return e},e}function _interopRequireWildcard(e){if(e&&e.__esModule)return e;if(null===e||"object"!=typeof e&&"function"!=typeof e)return{default:e};var r=_getRequireWildcardCache();if(r&&r.has(e))return r.get(e);var t={},n=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var o in e)if(Object.prototype.hasOwnProperty.call(e,o)){var u=n?Object.getOwnPropertyDescriptor(e,o):null;u&&(u.get||u.set)?Object.defineProperty(t,o,u):t[o]=e[o]}return t.default=e,r&&r.set(e,t),t}function _interopRequireDefault(e){return e&&e.__esModule?e:{default:e}}

},{"./transform.js":242,"./zoom.js":243}],241:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.nopropagation=nopropagation,exports.default=_default;var _d3Selection=require("d3-selection");function nopropagation(){_d3Selection.event.stopImmediatePropagation()}function _default(){_d3Selection.event.preventDefault(),_d3Selection.event.stopImmediatePropagation()}

},{"d3-selection":136}],242:[function(require,module,exports){
"use strict";function Transform(t,r,n){this.k=t,this.x=r,this.y=n}Object.defineProperty(exports,"__esModule",{value:!0}),exports.Transform=Transform,exports.default=transform,exports.identity=void 0,Transform.prototype={constructor:Transform,scale:function(t){return 1===t?this:new Transform(this.k*t,this.x,this.y)},translate:function(t,r){return 0===t&0===r?this:new Transform(this.k,this.x+this.k*t,this.y+this.k*r)},apply:function(t){return[t[0]*this.k+this.x,t[1]*this.k+this.y]},applyX:function(t){return t*this.k+this.x},applyY:function(t){return t*this.k+this.y},invert:function(t){return[(t[0]-this.x)/this.k,(t[1]-this.y)/this.k]},invertX:function(t){return(t-this.x)/this.k},invertY:function(t){return(t-this.y)/this.k},rescaleX:function(t){return t.copy().domain(t.range().map(this.invertX,this).map(t.invert,t))},rescaleY:function(t){return t.copy().domain(t.range().map(this.invertY,this).map(t.invert,t))},toString:function(){return"translate("+this.x+","+this.y+") scale("+this.k+")"}};var identity=new Transform(1,0,0);function transform(t){for(;!t.__zoom;)if(!(t=t.parentNode))return identity;return t.__zoom}exports.identity=identity,transform.prototype=Transform.prototype;

},{}],243:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),exports.default=_default;var _d3Dispatch=require("d3-dispatch"),_d3Drag=require("d3-drag"),_d3Interpolate=require("d3-interpolate"),_d3Selection=require("d3-selection"),_d3Transition=require("d3-transition"),_constant=_interopRequireDefault(require("./constant.js")),_event=_interopRequireDefault(require("./event.js")),_transform=require("./transform.js"),_noevent=_interopRequireWildcard(require("./noevent.js"));function _getRequireWildcardCache(){if("function"!=typeof WeakMap)return null;var t=new WeakMap;return _getRequireWildcardCache=function(){return t},t}function _interopRequireWildcard(t){if(t&&t.__esModule)return t;if(null===t||"object"!=typeof t&&"function"!=typeof t)return{default:t};var e=_getRequireWildcardCache();if(e&&e.has(t))return e.get(t);var n={},o=Object.defineProperty&&Object.getOwnPropertyDescriptor;for(var i in t)if(Object.prototype.hasOwnProperty.call(t,i)){var r=o?Object.getOwnPropertyDescriptor(t,i):null;r&&(r.get||r.set)?Object.defineProperty(n,i,r):n[i]=t[i]}return n.default=t,e&&e.set(t,n),n}function _interopRequireDefault(t){return t&&t.__esModule?t:{default:t}}function defaultFilter(){return!_d3Selection.event.ctrlKey&&!_d3Selection.event.button}function defaultExtent(){var t=this;return t instanceof SVGElement?(t=t.ownerSVGElement||t).hasAttribute("viewBox")?[[(t=t.viewBox.baseVal).x,t.y],[t.x+t.width,t.y+t.height]]:[[0,0],[t.width.baseVal.value,t.height.baseVal.value]]:[[0,0],[t.clientWidth,t.clientHeight]]}function defaultTransform(){return this.__zoom||_transform.identity}function defaultWheelDelta(){return-_d3Selection.event.deltaY*(1===_d3Selection.event.deltaMode?.05:_d3Selection.event.deltaMode?1:.002)}function defaultTouchable(){return navigator.maxTouchPoints||"ontouchstart"in this}function defaultConstrain(t,e,n){var o=t.invertX(e[0][0])-n[0][0],i=t.invertX(e[1][0])-n[1][0],r=t.invertY(e[0][1])-n[0][1],u=t.invertY(e[1][1])-n[1][1];return t.translate(i>o?(o+i)/2:Math.min(0,o)||Math.max(0,i),u>r?(r+u)/2:Math.min(0,r)||Math.max(0,u))}function _default(){var t,e,n=defaultFilter,o=defaultExtent,i=defaultConstrain,r=defaultWheelDelta,u=defaultTouchable,a=[0,1/0],c=[[-1/0,-1/0],[1/0,1/0]],s=250,l=_d3Interpolate.interpolateZoom,h=(0,_d3Dispatch.dispatch)("start","zoom","end"),f=500,d=150,p=0;function _(t){t.property("__zoom",defaultTransform).on("wheel.zoom",w).on("mousedown.zoom",T).on("dblclick.zoom",x).filter(u).on("touchstart.zoom",b).on("touchmove.zoom",q).on("touchend.zoom touchcancel.zoom",M).style("touch-action","none").style("-webkit-tap-highlight-color","rgba(0,0,0,0)")}function m(t,e){return(e=Math.max(a[0],Math.min(a[1],e)))===t.k?t:new _transform.Transform(e,t.x,t.y)}function v(t,e,n){var o=e[0]-n[0]*t.k,i=e[1]-n[1]*t.k;return o===t.x&&i===t.y?t:new _transform.Transform(t.k,o,i)}function y(t){return[(+t[0][0]+ +t[1][0])/2,(+t[0][1]+ +t[1][1])/2]}function g(t,e,n){t.on("start.zoom",function(){z(this,arguments).start()}).on("interrupt.zoom end.zoom",function(){z(this,arguments).end()}).tween("zoom",function(){var t=arguments,i=z(this,t),r=o.apply(this,t),u=null==n?y(r):"function"==typeof n?n.apply(this,t):n,a=Math.max(r[1][0]-r[0][0],r[1][1]-r[0][1]),c=this.__zoom,s="function"==typeof e?e.apply(this,t):e,h=l(c.invert(u).concat(a/c.k),s.invert(u).concat(a/s.k));return function(t){if(1===t)t=s;else{var e=h(t),n=a/e[2];t=new _transform.Transform(n,u[0]-e[0]*n,u[1]-e[1]*n)}i.zoom(null,t)}})}function z(t,e,n){return!n&&t.__zooming||new S(t,e)}function S(t,e){this.that=t,this.args=e,this.active=0,this.extent=o.apply(t,e),this.taps=0}function w(){if(n.apply(this,arguments)){var t=z(this,arguments),e=this.__zoom,o=Math.max(a[0],Math.min(a[1],e.k*Math.pow(2,r.apply(this,arguments)))),u=(0,_d3Selection.mouse)(this);if(t.wheel)t.mouse[0][0]===u[0]&&t.mouse[0][1]===u[1]||(t.mouse[1]=e.invert(t.mouse[0]=u)),clearTimeout(t.wheel);else{if(e.k===o)return;t.mouse=[u,e.invert(u)],(0,_d3Transition.interrupt)(this),t.start()}(0,_noevent.default)(),t.wheel=setTimeout(function(){t.wheel=null,t.end()},d),t.zoom("mouse",i(v(m(e,o),t.mouse[0],t.mouse[1]),t.extent,c))}}function T(){if(!e&&n.apply(this,arguments)){var t=z(this,arguments,!0),o=(0,_d3Selection.select)(_d3Selection.event.view).on("mousemove.zoom",function(){if((0,_noevent.default)(),!t.moved){var e=_d3Selection.event.clientX-u,n=_d3Selection.event.clientY-a;t.moved=e*e+n*n>p}t.zoom("mouse",i(v(t.that.__zoom,t.mouse[0]=(0,_d3Selection.mouse)(t.that),t.mouse[1]),t.extent,c))},!0).on("mouseup.zoom",function(){o.on("mousemove.zoom mouseup.zoom",null),(0,_d3Drag.dragEnable)(_d3Selection.event.view,t.moved),(0,_noevent.default)(),t.end()},!0),r=(0,_d3Selection.mouse)(this),u=_d3Selection.event.clientX,a=_d3Selection.event.clientY;(0,_d3Drag.dragDisable)(_d3Selection.event.view),(0,_noevent.nopropagation)(),t.mouse=[r,this.__zoom.invert(r)],(0,_d3Transition.interrupt)(this),t.start()}}function x(){if(n.apply(this,arguments)){var t=this.__zoom,e=(0,_d3Selection.mouse)(this),r=t.invert(e),u=t.k*(_d3Selection.event.shiftKey?.5:2),a=i(v(m(t,u),e,r),o.apply(this,arguments),c);(0,_noevent.default)(),s>0?(0,_d3Selection.select)(this).transition().duration(s).call(g,a,e):(0,_d3Selection.select)(this).call(_.transform,a)}}function b(){if(n.apply(this,arguments)){var e,o,i,r,u=_d3Selection.event.touches,a=u.length,c=z(this,arguments,_d3Selection.event.changedTouches.length===a);for((0,_noevent.nopropagation)(),o=0;o<a;++o)i=u[o],r=[r=(0,_d3Selection.touch)(this,u,i.identifier),this.__zoom.invert(r),i.identifier],c.touch0?c.touch1||c.touch0[2]===r[2]||(c.touch1=r,c.taps=0):(c.touch0=r,e=!0,c.taps=1+!!t);t&&(t=clearTimeout(t)),e&&(c.taps<2&&(t=setTimeout(function(){t=null},f)),(0,_d3Transition.interrupt)(this),c.start())}}function q(){if(this.__zooming){var e,n,o,r,u=z(this,arguments),a=_d3Selection.event.changedTouches,s=a.length;for((0,_noevent.default)(),t&&(t=clearTimeout(t)),u.taps=0,e=0;e<s;++e)n=a[e],o=(0,_d3Selection.touch)(this,a,n.identifier),u.touch0&&u.touch0[2]===n.identifier?u.touch0[0]=o:u.touch1&&u.touch1[2]===n.identifier&&(u.touch1[0]=o);if(n=u.that.__zoom,u.touch1){var l=u.touch0[0],h=u.touch0[1],f=u.touch1[0],d=u.touch1[1],p=(p=f[0]-l[0])*p+(p=f[1]-l[1])*p,_=(_=d[0]-h[0])*_+(_=d[1]-h[1])*_;n=m(n,Math.sqrt(p/_)),o=[(l[0]+f[0])/2,(l[1]+f[1])/2],r=[(h[0]+d[0])/2,(h[1]+d[1])/2]}else{if(!u.touch0)return;o=u.touch0[0],r=u.touch0[1]}u.zoom("touch",i(v(n,o,r),u.extent,c))}}function M(){if(this.__zooming){var t,n,o=z(this,arguments),i=_d3Selection.event.changedTouches,r=i.length;for((0,_noevent.nopropagation)(),e&&clearTimeout(e),e=setTimeout(function(){e=null},f),t=0;t<r;++t)n=i[t],o.touch0&&o.touch0[2]===n.identifier?delete o.touch0:o.touch1&&o.touch1[2]===n.identifier&&delete o.touch1;if(o.touch1&&!o.touch0&&(o.touch0=o.touch1,delete o.touch1),o.touch0)o.touch0[1]=this.__zoom.invert(o.touch0[0]);else if(o.end(),2===o.taps){var u=(0,_d3Selection.select)(this).on("dblclick.zoom");u&&u.apply(this,arguments)}}}return _.transform=function(t,e,n){var o=t.selection?t.selection():t;o.property("__zoom",defaultTransform),t!==o?g(t,e,n):o.interrupt().each(function(){z(this,arguments).start().zoom(null,"function"==typeof e?e.apply(this,arguments):e).end()})},_.scaleBy=function(t,e,n){_.scaleTo(t,function(){return this.__zoom.k*("function"==typeof e?e.apply(this,arguments):e)},n)},_.scaleTo=function(t,e,n){_.transform(t,function(){var t=o.apply(this,arguments),r=this.__zoom,u=null==n?y(t):"function"==typeof n?n.apply(this,arguments):n,a=r.invert(u),s="function"==typeof e?e.apply(this,arguments):e;return i(v(m(r,s),u,a),t,c)},n)},_.translateBy=function(t,e,n){_.transform(t,function(){return i(this.__zoom.translate("function"==typeof e?e.apply(this,arguments):e,"function"==typeof n?n.apply(this,arguments):n),o.apply(this,arguments),c)})},_.translateTo=function(t,e,n,r){_.transform(t,function(){var t=o.apply(this,arguments),u=this.__zoom,a=null==r?y(t):"function"==typeof r?r.apply(this,arguments):r;return i(_transform.identity.translate(a[0],a[1]).scale(u.k).translate("function"==typeof e?-e.apply(this,arguments):-e,"function"==typeof n?-n.apply(this,arguments):-n),t,c)},r)},S.prototype={start:function(){return 1==++this.active&&(this.that.__zooming=this,this.emit("start")),this},zoom:function(t,e){return this.mouse&&"mouse"!==t&&(this.mouse[1]=e.invert(this.mouse[0])),this.touch0&&"touch"!==t&&(this.touch0[1]=e.invert(this.touch0[0])),this.touch1&&"touch"!==t&&(this.touch1[1]=e.invert(this.touch1[0])),this.that.__zoom=e,this.emit("zoom"),this},end:function(){return 0==--this.active&&(delete this.that.__zooming,this.emit("end")),this},emit:function(t){(0,_d3Selection.customEvent)(new _event.default(_,t,this.that.__zoom),h.apply,h,[t,this.that,this.args])}},_.wheelDelta=function(t){return arguments.length?(r="function"==typeof t?t:(0,_constant.default)(+t),_):r},_.filter=function(t){return arguments.length?(n="function"==typeof t?t:(0,_constant.default)(!!t),_):n},_.touchable=function(t){return arguments.length?(u="function"==typeof t?t:(0,_constant.default)(!!t),_):u},_.extent=function(t){return arguments.length?(o="function"==typeof t?t:(0,_constant.default)([[+t[0][0],+t[0][1]],[+t[1][0],+t[1][1]]]),_):o},_.scaleExtent=function(t){return arguments.length?(a[0]=+t[0],a[1]=+t[1],_):[a[0],a[1]]},_.translateExtent=function(t){return arguments.length?(c[0][0]=+t[0][0],c[1][0]=+t[1][0],c[0][1]=+t[0][1],c[1][1]=+t[1][1],_):[[c[0][0],c[0][1]],[c[1][0],c[1][1]]]},_.constrain=function(t){return arguments.length?(i=t,_):i},_.duration=function(t){return arguments.length?(s=+t,_):s},_.interpolate=function(t){return arguments.length?(l=t,_):l},_.on=function(){var t=h.on.apply(h,arguments);return t===h?_:t},_.clickDistance=function(t){return arguments.length?(p=(t=+t)*t,_):Math.sqrt(p)},_}

},{"./constant.js":238,"./event.js":239,"./noevent.js":241,"./transform.js":242,"d3-dispatch":50,"d3-drag":54,"d3-interpolate":95,"d3-selection":136,"d3-transition":211}],244:[function(require,module,exports){
"use strict";Object.defineProperty(exports,"__esModule",{value:!0}),Object.defineProperty(exports,"select",{enumerable:!0,get:function(){return _d3Selection.select}}),Object.defineProperty(exports,"selectAll",{enumerable:!0,get:function(){return _d3Selection.selectAll}}),Object.defineProperty(exports,"scaleLinear",{enumerable:!0,get:function(){return _d3Scale.scaleLinear}}),Object.defineProperty(exports,"zoom",{enumerable:!0,get:function(){return _d3Zoom.zoom}}),Object.defineProperty(exports,"zoomTransform",{enumerable:!0,get:function(){return _d3Zoom.zoomTransform}}),Object.defineProperty(exports,"interval",{enumerable:!0,get:function(){return _d3Timer.interval}});var _d3Selection=require("d3-selection"),_d3Scale=require("d3-scale"),_d3Zoom=require("d3-zoom"),_d3Timer=require("d3-timer");

},{"d3-scale":115,"d3-selection":136,"d3-timer":206,"d3-zoom":240}]},{},[244])(244)
});


/***/ }),
/* 15 */
/***/ (function(module, exports) {

exports.INFINITY = 65535;

/***/ }),
/* 16 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__SidebarDisplay_js__ = __webpack_require__(8);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__ = __webpack_require__(3);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__Tooltip_js__ = __webpack_require__(2);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__Tooltip_js___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_2__Tooltip_js__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_3__Latex_js__ = __webpack_require__(0);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_4_mousetrap__ = __webpack_require__(5);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_4_mousetrap___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_4_mousetrap__);








const STATE_ADD_DIFFERENTIAL = 1;
const STATE_RM_DIFFERENTIAL = 2;
const STATE_ADD_STRUCTLINE = 3;
const STATE_RM_STRUCTLINE = 4;
const STATE_RM_EDGE = 5;
const STATE_ADD_CLASS = 6;

class EditorDisplay extends __WEBPACK_IMPORTED_MODULE_0__SidebarDisplay_js__["a" /* SidebarDisplay */] {
    constructor(container, sseq) {
        super(container);

        this.differentialColors = {};

        // Footer
        this.sidebar.footer.newGroup();
        this.sidebar.footer.addButtonRow([
            ["Undo", () => this.sseq.undo.undo()],
            ["Redo", () => this.sseq.undo.redo()]
        ]);

        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG("sseq.svg"));
        this.sidebar.footer.addButton("Save", () => this.sseq.download("sseq.json"));

        // General Panel
        this.generalPanel = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["b" /* Panel */](this.sidebar.main_div, this);
        this.generalPanel.newGroup();
        this.pageLabel = document.createElement("span");
        this.on("page-change", (r) => {
            this.pageLabel.innerHTML = this.getPageDescriptor(r);
            this._unselect();
        });
        this.generalPanel.addObject(this.pageLabel);

        this.generalPanel.newGroup();
        this.generalPanel.addButton("Add class", () => this.state = STATE_ADD_CLASS, { shortcuts: ["n"] });

        this.generalPanel.newGroup();
        this.generalPanel.addLinkedInput("Min X", "sseq.minX", "number");
        this.generalPanel.addLinkedInput("Max X", "sseq.maxX", "number");
        this.generalPanel.addLinkedInput("Min Y", "sseq.minY", "number");
        this.generalPanel.addLinkedInput("Max Y", "sseq.maxY", "number");
        this.sidebar.addPanel(this.generalPanel);

        // Class panel
        this.classPanel = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["d" /* TabbedPanel */](this.sidebar.main_div, this);
        this.sidebar.addPanel(this.classPanel);

        // Node tab
        this.nodeTab = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["b" /* Panel */](this.classPanel.container, this);
        this.nodeTab.newGroup();

        this.title_text = document.createElement("span");
        this.nodeTab.addObject(this.title_text);

        this.title_edit_link = document.createElement("a");
        this.title_edit_link.className = "card-link-body";
        this.title_edit_link.href = "#";
        this.title_edit_link.style.float = "right";
        this.title_edit_link.innerHTML = "Edit";
        this.title_edit_link.addEventListener("click", () => {
            let c = this.selected.c;
            if (this.title_edit_link.innerHTML == "OK") {
                let old_name = c.name;
                c.name = this.title_edit_input.value;
                this.sseq.undo.addValueChange(c, "name", old_name, c.name, () => this.sidebar.showPanel());
                this.sseq.emit("update");
                this.nodeTab.show();
            } else {
                this.title_edit_link.innerHTML = "OK";
                if (c.name) this.title_edit_input.value = c.name;
                this.title_edit_input.style.removeProperty("display");
            }
        });
        this.nodeTab.addObject(this.title_edit_link);

        this.title_edit_input = document.createElement("input");
        this.title_edit_input.className = "form-control mt-2";
        this.title_edit_input.type = "text";
        this.title_edit_input.placeholder = "Enter class name";
        this.nodeTab.addObject(this.title_edit_input);

        this.nodeTab.on("show", () => {
            this.title_edit_input.style.display = "none";
            this.title_edit_input.value = "";
            this.title_edit_link.innerHTML = "Edit";
            let c = this.selected.c;
            if (c.name) {
                this.title_text.innerHTML = Object(__WEBPACK_IMPORTED_MODULE_3__Latex_js__["b" /* renderMath */])(c.name) + ` - (${c.x}, ${c.y})`;
            } else {
                this.title_text.innerHTML = `<span style='color: gray'>unnamed</span> - (${c.x}, ${c.y})`;
            }
        });

        this.nodeTab.newGroup();
        this.nodeTab.addLinkedInput("Color", "selected.color", "text", "selected.c");
        this.nodeTab.addLinkedInput("Size", "selected.size", "number", "selected.c");
        this.nodeTab.addButton("Delete class", () => {
            this.sseq.startMutationTracking();
            this.sseq.deleteClass(this.selected.c);
            this.sseq.addMutationsToUndoStack();
            this.sidebar.showPanel(this.generalPanel)
        }, { style: "danger" });
        this.classPanel.addTab("Node", this.nodeTab);

        // Differentials tab
        this.differentialTab = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["a" /* DifferentialPanel */](this.classPanel.container, this);
        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('d', () => this.state = STATE_ADD_DIFFERENTIAL);
        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('r', () => this.state = STATE_RM_EDGE);
        this.classPanel.addTab("Diff", this.differentialTab);

        // Structline tab
        this.structlineTab = new __WEBPACK_IMPORTED_MODULE_1__panel_mod_js__["c" /* StructlinePanel */](this.classPanel.container, this);
        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('s', () => this.state = STATE_ADD_STRUCTLINE);
        this.classPanel.addTab("Struct", this.structlineTab);

        this.sidebar.showPanel(this.generalPanel);

        this.tooltip = new __WEBPACK_IMPORTED_MODULE_2__Tooltip_js__["Tooltip"](this);
        this.on("mouseover", this._onMouseover.bind(this));
        this.on("mouseout", this._onMouseout.bind(this));
        this.on("click", this.__onClick.bind(this)); // Display already has an _onClick

        this._onDifferentialAdded = this._onDifferentialAdded.bind(this);

        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('left',  this.previousPage);
        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('right', this.nextPage);
        __WEBPACK_IMPORTED_MODULE_4_mousetrap__["bind"]('x', () => { if(this.selected){ console.log(this.selected.c); } });

        if (sseq) this.setSseq(sseq);
    }

    setDifferentialColor(page, color) {
        this.differentialColors[page] = color;
    }

    setSseq(sseq) {
        if (this.sseq) {
            this.sseq.removeListener("differential-added", this._onDifferentialAdded);
        }

        super.setSseq(sseq)
        this.sidebar.showPanel(this.generalPanel);

        this.sseq.on("differential-added", this._onDifferentialAdded);
    }

    _onMouseover(node) {
        this.tooltip.setHTML(`(${node.c.x}, ${node.c.y})`);
        this.tooltip.show(node.canvas_x, node.canvas_y);
    }

    _onMouseout() {
        if (this.selected){
            this.selected.highlight = true;  
        }
        this.tooltip.hide();
    }

    _unselect() {
        if (!this.selected) return;

        this.selected.highlight = false;
        this.selected = null;
        this.state = null;

        this.sidebar.showPanel(this.generalPanel);

        this._drawSseq(this.context);
    }

    __onClick(node, e) {
        if (this.state == STATE_ADD_CLASS) {
            let x = Math.round(this.xScale.invert(e.clientX));
            let y = Math.round(this.yScale.invert(e.clientY));
            this.sseq.undo.startMutationTracking();
            this.sseq.addClass(x, y);
            this.sseq.undo.addMutationsToUndoStack();
            this.state = null;
            return;
        }

        if (!node) {
            this._unselect();
            return;
        }

        if (!this.selected) {
            this._unselect();
            this.selected = node;
            this.sidebar.showPanel(this.classPanel);
            this.state = null;
            return;
        }

        let s = this.selected.c;
        let t = node.c;
        switch (this.state) {
            case STATE_ADD_DIFFERENTIAL:
                if(s.x !== t.x + 1){
                    this._unselect();
                    break;
                }
                let length = t.y - s.y;
                this.sseq.undo.startMutationTracking();
                this.sseq.addDifferential(s, t, length);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_DIFFERENTIAL:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.type === "Differential" && e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_ADD_STRUCTLINE:
                this.sseq.undo.startMutationTracking();
                this.sseq.addStructline(s, t);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_STRUCTLINE:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.type === "Structline" && e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            case STATE_RM_EDGE:
                this.sseq.undo.startMutationTracking();
                for (let e of s.edges)
                    if (e.target == t)
                        sseq.deleteEdge(e);
                this.sseq.undo.addMutationsToUndoStack();
                this.sidebar.showPanel();
                break;
            default:
                this._unselect();
                this.selected = node;
                this.sidebar.showPanel(this.classPanel);
                break;
        }
        this.state = null;
    }

    _onDifferentialAdded(d) {
        if (this.differentialColors[d.page])
            d.color = this.differentialColors[d.page];
    }
}
/* unused harmony export EditorDisplay */



/***/ }),
/* 17 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Latex_js__ = __webpack_require__(0);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Panel_js__ = __webpack_require__(1);



class DifferentialPanel extends __WEBPACK_IMPORTED_MODULE_1__Panel_js__["a" /* Panel */] {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.differential_list = document.createElement("ul");
        this.differential_list.className = "list-group list-group-flush";
        this.differential_list.style["text-align"] = "center";
        this.addObject(this.differential_list);

        this.on("show", () => {
            while(this.differential_list.firstChild) {
                this.differential_list.removeChild(this.differential_list.firstChild);
            }

            let edges = this.display.selected.c.edges.filter(e => e.type === "Differential").sort((a, b) => a.page - b.page);

            let sname, tname;
            for (let e of edges) {
                sname = e.source.name ? e.source.name : "?"
                tname = e.target.name ? e.target.name : "?"
                if (e.source == this.display.selected.c){
                    this.addLI(Object(__WEBPACK_IMPORTED_MODULE_0__Latex_js__["b" /* renderMath */])(`d_${e.page}({\\color{blue}${sname}}) = ${tname}`));
                } else {
                    this.addLI(Object(__WEBPACK_IMPORTED_MODULE_0__Latex_js__["b" /* renderMath */])(`d_${e.page}(${sname}) = {\\color{blue}${tname}}`));
                }
            }

            this.addLI("<a href='#'>Add differential</a>", () => this.display.state = STATE_ADD_DIFFERENTIAL );
            this.addLI("<a href='#'>Remove differential</a>", () => this.display.state = STATE_RM_DIFFERENTIAL );
        });
    }

    addLI(html, callback) {
        let node = document.createElement("li");
        node.className = "list-group-item";
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback) {
            node.addEventListener("click", callback);
        }
        this.differential_list.appendChild(node);
    }
}
/* harmony export (immutable) */ __webpack_exports__["a"] = DifferentialPanel;




/***/ }),
/* 18 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return _StructlinePanel; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Latex_js__ = __webpack_require__(0);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Panel_js__ = __webpack_require__(1);



class StructlinePanel extends __WEBPACK_IMPORTED_MODULE_1__Panel_js__["a" /* Panel */] {
    constructor(parentContainer, display) {
        super(parentContainer, display);

        this.structline_list = document.createElement("ul");
        this.structline_list.className = "list-group list-group-flush";
        this.structline_list.style["text-align"] = "center";
        this.addObject(this.structline_list);

        this.on("show", () => {
            while(this.structline_list.firstChild)
                this.structline_list.removeChild(this.structline_list.firstChild);

            let edges = this.display.selected.c.edges.filter(e => e.type === "Structline").sort((a, b) => a.page - b.page);

            let sname, tname;
            for (let e of edges) {
                sname = e.source.name ? e.source.name : "?"
                tname = e.target.name ? e.target.name : "?"
                if (e.source == this.display.selected.c)
                    this.addLI(Object(__WEBPACK_IMPORTED_MODULE_0__Latex_js__["b" /* renderMath */])(`{\\color{blue}${sname}} \\text{---} ${tname}`));
                else
                    this.addLI(Object(__WEBPACK_IMPORTED_MODULE_0__Latex_js__["b" /* renderMath */])(`${sname} \\text{---} {\\color{blue}${tname}}`));
            }

            this.addLI("<a href='#'>Add structline</a>", () => this.display.state = STATE_ADD_STRUCTLINE );
            this.addLI("<a href='#'>Remove structline</a>", () => this.display.state = STATE_RM_STRUCTLINE );
        });

    }

    addLI(html, callback) {
        let node = document.createElement("li");
        node.className = "list-group-item";
        node.style = "padding: 0.75rem 0";
        node.innerHTML = html;
        if (callback)
            node.addEventListener("click", callback);
        this.structline_list.appendChild(node);
    }
}

const _StructlinePanel = StructlinePanel;



/***/ }),
/* 19 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, "a", function() { return _TabbedPanel; });
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0__Panel_js__ = __webpack_require__(1);


/**
 * This is a panel whose some purpose is to contain further panels arranged in
 * tabs. This is used, for example, in EditorDisplay for configuring different
 * properties of a class.
 *
 * @property {Panel} currentTab - The current tab that is displayed.
 */
class TabbedPanel extends __WEBPACK_IMPORTED_MODULE_0__Panel_js__["a" /* Panel */] {
    constructor (parentContainer, display) {
        super(parentContainer, display);

        let head = document.createElement("div");
        head.className = "card-header";
        this.container.appendChild(head);

        this.header = document.createElement("ul");
        this.header.className = "nav nav-tabs card-header-tabs";
        head.appendChild(this.header);

        this.tabs = [];
        this.currentTab = null;
    }

    /**
     * This adds a tab to TabbedPanel.
     *
     * @param {string} name - The name of the tab, to be displayed in the
     * header. Avoid making this too long.
     * @param {Panel} tab - The tab to be added.
     */
    addTab(name, tab) {
        let li = document.createElement("li");
        li.className = "nav-item";
        this.header.appendChild(li);

        let a = document.createElement("a");
        a.className = "nav-link";
        a.href = "#";
        a.innerHTML = name;
        li.appendChild(a);

        a.addEventListener("click", () => this.showTab(tab));
        this.tabs[this.tabs.length] = [tab, a];

        if (!this.currentTab) this.currentTab = tab;
    }

    show() {
        super.show();
        this.showTab(this.currentTab);
    }

    /**
     * Sets the corresponding tab to be the active tab and shows it (of course,
     * the tab will not be actually shown if the panel itself is hidden).
     *
     * @param {Panel} tab - Tab to be shown.
     */
    showTab(tab) {
        this.currentTab = tab;
        for (let t of this.tabs) {
            if (t[0] == tab) {
                t[1].className = "nav-link active";
                t[0].show();
            } else {
                t[1].className = "nav-link";
                t[0].hide();
            }
        }
    }
}

const _TabbedPanel = TabbedPanel;



/***/ }),
/* 20 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_katex__ = __webpack_require__(7);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_0_katex___default = __webpack_require__.n(__WEBPACK_IMPORTED_MODULE_0_katex__);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_1__Latex_js__ = __webpack_require__(0);
/* harmony import */ var __WEBPACK_IMPORTED_MODULE_2__Panel_js__ = __webpack_require__(1);




class TablePanel extends __WEBPACK_IMPORTED_MODULE_2__Panel_js__["a" /* Panel */] {
    constructor(parentContainer, display) {
        super(parentContainer, display);
    }

    show() {
        // if (!this.display.selected){
        //     return;
        // }

        this.container.style.removeProperty("display");
        this.container.style.textAlign = "center";
        this.clear();

        this.newGroup();
        this.addHeader("Classes");
        // let [x, y] = this.display.selected;
        // let page = this.display.page;
        // let sseq = this.display.sseq;

        // let classes = sseq.getClasses(x, y, page);
        // let names = sseq.classNames.get(x, y);

        let div = document.createElement("div");
        for (let c of ["x^2", "h_0^2", "c_1"]) {
            let n = document.createElement("span");
            n.style.padding = "0 0.6em";
            n.innerHTML = __WEBPACK_IMPORTED_MODULE_0_katex__["renderToString"](c);
            n.addEventListener("click", () => {
                let name = prompt("New class name");
                if (name !== null) {
                    
                }
            });
            div.appendChild(n);
        }
        this.addObject(div);

        // let decompositions = sseq.decompositions.get(x, y);
        // if (decompositions && decompositions.length > 0) {
        //     this.newGroup();
        //     this.addHeader("Decompositions");
        //     for (let d of decompositions) {
        //         let single = d[0].reduce((a, b) => a + b, 0) == 1;
        //         single = single && this.display.constructor.name != "CalculationDisplay";

        //         let highlights = [[x - d[2], y - d[3]]];
        //         if (this.display.isUnit) {
        //             highlights[1] = [d[2], d[3]]
        //         }
        //         if (single) {
        //             let idx = d[0].indexOf(1);
        //             // If we named the element after the decomposition, there is no point in displaying it...
        //             if (katex.renderToString(names[idx]) != katex.renderToString(d[1])) {
        //                 this.addLine(katex.renderToString(names[idx] + " = " + d[1]), () => {
        //                     if (confirm(`Rename ${names[idx]} as ${d[1]}?`)) {
        //                         sseq.setClassName(x, y, idx, d[1]);
        //                         this.display.clearHighlight();
        //                     }
        //                 }, highlights);
        //             }
        //         } else {
        //             this.addLine(katex.renderToString(vecToName(d[0], names) + " = " + d[1]), undefined, highlights);
        //         }
        //     }
        // }
    }
}
/* harmony export (immutable) */ __webpack_exports__["a"] = TablePanel;


/***/ }),
/* 21 */
/***/ (function(module, exports) {

class Undo {
    constructor(sseq){
        this.sseq = sseq;
        this.undoStack = [];
        this.undoObjStack = [];
        this.redoStack = [];
        this.redoObjStack = [];
        this.undo = this.undo.bind(this);
        this.redo = this.redo.bind(this);
    };

    startMutationTracking(){
        this.mutationMap = new Map();
    }

    addMutationsToUndoStack(event_obj){
        this.add(this.mutationMap, event_obj);
        this.mutationMap = undefined;
    }

    addMutation(obj, pre, post){
        if(!this.mutationMap){
            return;
        }
        if(this.mutationMap.get(obj)){
            pre = this.mutationMap.get(obj).before;
        }
        this.mutationMap.set(obj, {obj: obj, before: pre, after : post});
    }

    add(mutations, event_obj) {
        this.undoStack.push({type:"normal",  mutations: mutations});
        this.undoObjStack.push(event_obj);
        this.redoStack = [];
        this.redoObjStack = [];
    }

    addValueChange(target, prop, before, after, callback) {
        let e = {type:"value", target: target, prop: prop, before: before, after: after, callback: callback};
        this.undoStack.push(e);
        this.undoObjStack.push(e);
        this.redoStack = [];
        this.redoObjStack = [];
    }
    addManual(e, e_obj) {
        this.undoStack.push(e);
        this.undoObjStack.push(e_obj);
        this.redoStack = [];
        this.redoObjStack = [];
    }

    clear(){
        this.undoStack = [];
        this.redoStack = [];
    };

    undo() {
        if (this.undoStack.length === 0) {
            return;
        }
        let e = this.undoStack.pop();
        this.redoStack.push(e);
        let obj = this.undoObjStack.pop();
        this.redoObjStack.push(obj);
        switch (e.type) {
            case "normal":
                this.undoNormal(e);
                break;
            case "value":
                e.target[e.prop] = e.before;
                if (e.callback) e.callback();
                break;
        }
        this.sseq.emit("update");
    };

    undoNormal(obj){
        let mutations = obj.mutations;
        for(let m of mutations.values()){
            if(m.obj.undoFromMemento){
                m.obj.undoFromMemento(m.before);
            } else {
                m.obj.restoreFromMemento(m.before);
            }
        }
    }

    redo() {
        if (this.redoStack.length === 0) {
            return;
        }
        let e = this.redoStack.pop();
        this.undoStack.push(e);
        let obj = this.redoObjStack.pop();
        this.undoObjStack.push(obj);
        switch (e.type) {
            case "normal":
                this.redoNormal(e);
                break;
            case "value":
                e.target[e.prop] = e.after;
                if (e.callback) e.callback();
                break;
        }
        this.sseq.emit("update");
    };

    redoNormal(obj){
        let mutations = obj.mutations;
        for(let m of mutations.values()){
            if(m.obj.redoFromMemento){
                m.obj.redoFromMemento(m.after);
            } else {
                m.obj.restoreFromMemento(m.after);
            }
        }
    }

    addLock(msg){
        let d = new Date();
        if(msg === undefined){
            msg = `Undo events before save at ${d.getFullYear()}-${d.getMonth()}-${d.getDay()} ${d.getHours()}:${d.getMinutes().toString().padStart(2,"0")}?`;
        }
        this.undoStack.push({
            type : "lock",
            msg : msg,
            date : d,
            undoFunction : lockFunction.bind(this)
        })
    }

    getEventObjects() {
        return this.undoObjStack;
    }

    toJSON(){
        return this.undoStack.map(function(e) {
            if(e.type === "normal"){
                return {
                    "type" : "normal",
                    "mutations" : Array.from(e.mutations.entries()).map(([k,v]) => [k.recid, v.before])
                };
            } else {
                return e;
            }
        });
    }
}

Undo.undoFunctions = {};
Undo.redoFunctions = {};
Undo.undoFunctions["lock"] = lockFunction;
Undo.redoFunctions["lock"] = function() {};


function lockFunction(obj){
    console.error("This function (lock) probably doesn't work!");
    confirm(obj.msg)
        .yes(() => {
            this.redoStack.pop();
        })
        .no(() => {
            let e = this.redoStack.pop();
            this.undoStack.push(e);
        });
}

Undo.defaultLockMessage = "Undo events before loaded page?";

exports.Undo = Undo;


/***/ }),
/* 22 */
/***/ (function(module, exports) {

exports.loadFromServer = async function(path){
    let response = await fetch(path);
    return await response.json();
};

/***/ }),
/* 23 */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
class BadMessageError extends TypeError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}
/* unused harmony export BadMessageError */


class UnknownCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}
/* unused harmony export UnknownCommandError */


class InvalidCommandError extends BadMessageError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}
/* unused harmony export InvalidCommandError */



class UnknownDisplayCommandError extends UnknownCommandError {
    constructor(...args) {
        super(...args)
        this.name = this.constructor.name;
        this.stack = this.stack.split("\n").slice(1).join("\n")
    }
}
/* unused harmony export UnknownDisplayCommandError */


class SocketListener {
    constructor(websocket) {
        this.websocket = websocket;
        this.websocket.onmessage = this.onmessage.bind(this);
        this.websocket.onopen = this.onopen.bind(this);
        this.message_dispatch = {};
        this.debug_mode = false;
    }

    add_message_handler(cmd_filter, handler) {
        this.message_dispatch[cmd_filter] = handler;
    }

    start() {
        console.log("client ready");
        this.client_ready = true;
        if(this.socket_ready) {
            this._start();
        }
    }

    onopen(event) {
        console.log("socket opened");
        this.socket_ready = true;
        if(this.client_ready){
            this._start();
        }
    }

    _start(){
        console.error("send_introduction_message");
        this.handle_message({
            "cmd" : ["start"],
            "args" : [],
            "kwargs" : {},
        }, false);
    }

    onmessage(event) {
        let msg = JSON.parse(event.data);
        this.handle_message(msg, true);
    }

    send(cmd, kwargs) { // args parameter?
        let args = [];
        console.log("send message", cmd, args, kwargs);
        if(args === undefined || kwargs === undefined) {
            throw TypeError(`Send with missing arguments.`);
        }
        if(args.constructor !== Array){
            throw TypeError(`Argument "args" expected to have type "Array" not "${args.constructor.name}"`);
        }
        if(kwargs.constructor !== Object){
            throw TypeError(`Argument "kwargs" expected to have type "Object" not "${kwargs.constructor.name}"`);
        }            
        if("cmd" in kwargs) {
            throw ValueError(`Tried to send message with top level "cmd" key`);
        }
        let obj = { "cmd" : cmd, "args" : args, "kwargs" : kwargs };
        let json_str = JSON.stringify(obj);
        this.websocket.send(json_str);
    }

    console_log_if_debug(msg) {
        if(this.debug_mode) {
            console.log(msg);
        }
    }
    
    debug(type, text, orig_msg) {
        let cmd = "debug";
        if(type !== ""){
            cmd += `.${type}`
        }            
        this.send("debug", {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg
        });
    }

    info(type, text, orig_msg) {
        let cmd = "info";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg
        });
    }

    warning(type, text, orig_msg, stack_trace) {
        let cmd = "warning";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, {
            "type" : type,
            "text" : text, 
            "orig_msg" : orig_msg,
            "stack_trace" : stack_trace
        });
    }

    error(type, msg) {
        let cmd = "error.client";
        if(type !== ""){
            cmd += `.${type}`
        }
        this.send(cmd, msg);
    }

    report_error_to_server(error, orig_msg) {
        // For some reason JSON.stringify(error) drops the "message" field by default.
        // We move it to "msg" to avoid that.
        error.msg = error.message; 
        this.error(error.name, 
            {
                "exception" : error,
                "orig_msg" : orig_msg,
            }
        );
    }

    handle_message(msg, report_error_to_server) {
        this.console_log_if_debug(msg);
        try {
            if(msg.cmd === undefined) {
                throw new UnknownCommandError(`Server sent message missing "cmd" field.`);
            }
    
            if(msg.cmd.constructor != Array){
                throw new InvalidCommandError(
                    `"msg.cmd" should have type "Array" not "${msg.cmd.constructor.name}."`
                );
            }
    
            if(msg.args === undefined) {
                throw new InvalidCommandError(
                    `Message is missing the "args" field.`
                );
            }
            
            if(msg.kwargs === undefined) {
                throw new InvalidCommandError(
                    `Message is missing the "kwargs" field.`
                );
            }
    
            let key = undefined;
            for(let partial_cmd of msg.cmd) {
                if(this.message_dispatch[partial_cmd] !== undefined){
                    key = partial_cmd; 
                    break;
                }
            }
            this.console_log_if_debug("cmd", msg.cmd, "key", key);
            this.console_log_if_debug("received message","cmd", msg.cmd, "key", key);
            if(key === undefined) {
                throw new UnknownCommandError(`Server sent unknown command "${msg.cmd[0]}".`);
            }
            this.message_dispatch[key](msg.cmd, msg.args, msg.kwargs);
        } catch(error) {
            this.console_log_if_debug(error);
            console.error(error);
            if(report_error_to_server){
                this.report_error_to_server(error, msg);
            }
        }
    }    
}
/* harmony export (immutable) */ __webpack_exports__["a"] = SocketListener;


/***/ }),
/* 24 */
/***/ (function(module, exports, __webpack_require__) {

var __WEBPACK_AMD_DEFINE_RESULT__;/*global define:false */
/**
 * Copyright 2012-2017 Craig Campbell
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Mousetrap is a simple keyboard shortcut library for Javascript with
 * no external dependencies
 *
 * @version 1.6.5
 * @url craig.is/killing/mice
 */
(function(window, document, undefined) {

    // Check if mousetrap is used inside browser, if not, return
    if (!window) {
        return;
    }

    /**
     * mapping of special keycodes to their corresponding keys
     *
     * everything in this dictionary cannot use keypress events
     * so it has to be here to map to the correct keycodes for
     * keyup/keydown events
     *
     * @type {Object}
     */
    var _MAP = {
        8: 'backspace',
        9: 'tab',
        13: 'enter',
        16: 'shift',
        17: 'ctrl',
        18: 'alt',
        20: 'capslock',
        27: 'esc',
        32: 'space',
        33: 'pageup',
        34: 'pagedown',
        35: 'end',
        36: 'home',
        37: 'left',
        38: 'up',
        39: 'right',
        40: 'down',
        45: 'ins',
        46: 'del',
        91: 'meta',
        93: 'meta',
        224: 'meta'
    };

    /**
     * mapping for special characters so they can support
     *
     * this dictionary is only used incase you want to bind a
     * keyup or keydown event to one of these keys
     *
     * @type {Object}
     */
    var _KEYCODE_MAP = {
        106: '*',
        107: '+',
        109: '-',
        110: '.',
        111 : '/',
        186: ';',
        187: '=',
        188: ',',
        189: '-',
        190: '.',
        191: '/',
        192: '`',
        219: '[',
        220: '\\',
        221: ']',
        222: '\''
    };

    /**
     * this is a mapping of keys that require shift on a US keypad
     * back to the non shift equivelents
     *
     * this is so you can use keyup events with these keys
     *
     * note that this will only work reliably on US keyboards
     *
     * @type {Object}
     */
    var _SHIFT_MAP = {
        '~': '`',
        '!': '1',
        '@': '2',
        '#': '3',
        '$': '4',
        '%': '5',
        '^': '6',
        '&': '7',
        '*': '8',
        '(': '9',
        ')': '0',
        '_': '-',
        '+': '=',
        ':': ';',
        '\"': '\'',
        '<': ',',
        '>': '.',
        '?': '/',
        '|': '\\'
    };

    /**
     * this is a list of special strings you can use to map
     * to modifier keys when you specify your keyboard shortcuts
     *
     * @type {Object}
     */
    var _SPECIAL_ALIASES = {
        'option': 'alt',
        'command': 'meta',
        'return': 'enter',
        'escape': 'esc',
        'plus': '+',
        'mod': /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'meta' : 'ctrl'
    };

    /**
     * variable to store the flipped version of _MAP from above
     * needed to check if we should use keypress or not when no action
     * is specified
     *
     * @type {Object|undefined}
     */
    var _REVERSE_MAP;

    /**
     * loop through the f keys, f1 to f19 and add them to the map
     * programatically
     */
    for (var i = 1; i < 20; ++i) {
        _MAP[111 + i] = 'f' + i;
    }

    /**
     * loop through to map numbers on the numeric keypad
     */
    for (i = 0; i <= 9; ++i) {

        // This needs to use a string cause otherwise since 0 is falsey
        // mousetrap will never fire for numpad 0 pressed as part of a keydown
        // event.
        //
        // @see https://github.com/ccampbell/mousetrap/pull/258
        _MAP[i + 96] = i.toString();
    }

    /**
     * cross browser add event method
     *
     * @param {Element|HTMLDocument} object
     * @param {string} type
     * @param {Function} callback
     * @returns void
     */
    function _addEvent(object, type, callback) {
        if (object.addEventListener) {
            object.addEventListener(type, callback, false);
            return;
        }

        object.attachEvent('on' + type, callback);
    }

    /**
     * takes the event and returns the key character
     *
     * @param {Event} e
     * @return {string}
     */
    function _characterFromEvent(e) {

        // for keypress events we should return the character as is
        if (e.type == 'keypress') {
            var character = String.fromCharCode(e.which);

            // if the shift key is not pressed then it is safe to assume
            // that we want the character to be lowercase.  this means if
            // you accidentally have caps lock on then your key bindings
            // will continue to work
            //
            // the only side effect that might not be desired is if you
            // bind something like 'A' cause you want to trigger an
            // event when capital A is pressed caps lock will no longer
            // trigger the event.  shift+a will though.
            if (!e.shiftKey) {
                character = character.toLowerCase();
            }

            return character;
        }

        // for non keypress events the special maps are needed
        if (_MAP[e.which]) {
            return _MAP[e.which];
        }

        if (_KEYCODE_MAP[e.which]) {
            return _KEYCODE_MAP[e.which];
        }

        // if it is not in the special map

        // with keydown and keyup events the character seems to always
        // come in as an uppercase character whether you are pressing shift
        // or not.  we should make sure it is always lowercase for comparisons
        return String.fromCharCode(e.which).toLowerCase();
    }

    /**
     * checks if two arrays are equal
     *
     * @param {Array} modifiers1
     * @param {Array} modifiers2
     * @returns {boolean}
     */
    function _modifiersMatch(modifiers1, modifiers2) {
        return modifiers1.sort().join(',') === modifiers2.sort().join(',');
    }

    /**
     * takes a key event and figures out what the modifiers are
     *
     * @param {Event} e
     * @returns {Array}
     */
    function _eventModifiers(e) {
        var modifiers = [];

        if (e.shiftKey) {
            modifiers.push('shift');
        }

        if (e.altKey) {
            modifiers.push('alt');
        }

        if (e.ctrlKey) {
            modifiers.push('ctrl');
        }

        if (e.metaKey) {
            modifiers.push('meta');
        }

        return modifiers;
    }

    /**
     * prevents default for this event
     *
     * @param {Event} e
     * @returns void
     */
    function _preventDefault(e) {
        if (e.preventDefault) {
            e.preventDefault();
            return;
        }

        e.returnValue = false;
    }

    /**
     * stops propogation for this event
     *
     * @param {Event} e
     * @returns void
     */
    function _stopPropagation(e) {
        if (e.stopPropagation) {
            e.stopPropagation();
            return;
        }

        e.cancelBubble = true;
    }

    /**
     * determines if the keycode specified is a modifier key or not
     *
     * @param {string} key
     * @returns {boolean}
     */
    function _isModifier(key) {
        return key == 'shift' || key == 'ctrl' || key == 'alt' || key == 'meta';
    }

    /**
     * reverses the map lookup so that we can look for specific keys
     * to see what can and can't use keypress
     *
     * @return {Object}
     */
    function _getReverseMap() {
        if (!_REVERSE_MAP) {
            _REVERSE_MAP = {};
            for (var key in _MAP) {

                // pull out the numeric keypad from here cause keypress should
                // be able to detect the keys from the character
                if (key > 95 && key < 112) {
                    continue;
                }

                if (_MAP.hasOwnProperty(key)) {
                    _REVERSE_MAP[_MAP[key]] = key;
                }
            }
        }
        return _REVERSE_MAP;
    }

    /**
     * picks the best action based on the key combination
     *
     * @param {string} key - character for key
     * @param {Array} modifiers
     * @param {string=} action passed in
     */
    function _pickBestAction(key, modifiers, action) {

        // if no action was picked in we should try to pick the one
        // that we think would work best for this key
        if (!action) {
            action = _getReverseMap()[key] ? 'keydown' : 'keypress';
        }

        // modifier keys don't work as expected with keypress,
        // switch to keydown
        if (action == 'keypress' && modifiers.length) {
            action = 'keydown';
        }

        return action;
    }

    /**
     * Converts from a string key combination to an array
     *
     * @param  {string} combination like "command+shift+l"
     * @return {Array}
     */
    function _keysFromString(combination) {
        if (combination === '+') {
            return ['+'];
        }

        combination = combination.replace(/\+{2}/g, '+plus');
        return combination.split('+');
    }

    /**
     * Gets info for a specific key combination
     *
     * @param  {string} combination key combination ("command+s" or "a" or "*")
     * @param  {string=} action
     * @returns {Object}
     */
    function _getKeyInfo(combination, action) {
        var keys;
        var key;
        var i;
        var modifiers = [];

        // take the keys from this pattern and figure out what the actual
        // pattern is all about
        keys = _keysFromString(combination);

        for (i = 0; i < keys.length; ++i) {
            key = keys[i];

            // normalize key names
            if (_SPECIAL_ALIASES[key]) {
                key = _SPECIAL_ALIASES[key];
            }

            // if this is not a keypress event then we should
            // be smart about using shift keys
            // this will only work for US keyboards however
            if (action && action != 'keypress' && _SHIFT_MAP[key]) {
                key = _SHIFT_MAP[key];
                modifiers.push('shift');
            }

            // if this key is a modifier then add it to the list of modifiers
            if (_isModifier(key)) {
                modifiers.push(key);
            }
        }

        // depending on what the key combination is
        // we will try to pick the best event for it
        action = _pickBestAction(key, modifiers, action);

        return {
            key: key,
            modifiers: modifiers,
            action: action
        };
    }

    function _belongsTo(element, ancestor) {
        if (element === null || element === document) {
            return false;
        }

        if (element === ancestor) {
            return true;
        }

        return _belongsTo(element.parentNode, ancestor);
    }

    function Mousetrap(targetElement) {
        var self = this;

        targetElement = targetElement || document;

        if (!(self instanceof Mousetrap)) {
            return new Mousetrap(targetElement);
        }

        /**
         * element to attach key events to
         *
         * @type {Element}
         */
        self.target = targetElement;

        /**
         * a list of all the callbacks setup via Mousetrap.bind()
         *
         * @type {Object}
         */
        self._callbacks = {};

        /**
         * direct map of string combinations to callbacks used for trigger()
         *
         * @type {Object}
         */
        self._directMap = {};

        /**
         * keeps track of what level each sequence is at since multiple
         * sequences can start out with the same sequence
         *
         * @type {Object}
         */
        var _sequenceLevels = {};

        /**
         * variable to store the setTimeout call
         *
         * @type {null|number}
         */
        var _resetTimer;

        /**
         * temporary state where we will ignore the next keyup
         *
         * @type {boolean|string}
         */
        var _ignoreNextKeyup = false;

        /**
         * temporary state where we will ignore the next keypress
         *
         * @type {boolean}
         */
        var _ignoreNextKeypress = false;

        /**
         * are we currently inside of a sequence?
         * type of action ("keyup" or "keydown" or "keypress") or false
         *
         * @type {boolean|string}
         */
        var _nextExpectedAction = false;

        /**
         * resets all sequence counters except for the ones passed in
         *
         * @param {Object} doNotReset
         * @returns void
         */
        function _resetSequences(doNotReset) {
            doNotReset = doNotReset || {};

            var activeSequences = false,
                key;

            for (key in _sequenceLevels) {
                if (doNotReset[key]) {
                    activeSequences = true;
                    continue;
                }
                _sequenceLevels[key] = 0;
            }

            if (!activeSequences) {
                _nextExpectedAction = false;
            }
        }

        /**
         * finds all callbacks that match based on the keycode, modifiers,
         * and action
         *
         * @param {string} character
         * @param {Array} modifiers
         * @param {Event|Object} e
         * @param {string=} sequenceName - name of the sequence we are looking for
         * @param {string=} combination
         * @param {number=} level
         * @returns {Array}
         */
        function _getMatches(character, modifiers, e, sequenceName, combination, level) {
            var i;
            var callback;
            var matches = [];
            var action = e.type;

            // if there are no events related to this keycode
            if (!self._callbacks[character]) {
                return [];
            }

            // if a modifier key is coming up on its own we should allow it
            if (action == 'keyup' && _isModifier(character)) {
                modifiers = [character];
            }

            // loop through all callbacks for the key that was pressed
            // and see if any of them match
            for (i = 0; i < self._callbacks[character].length; ++i) {
                callback = self._callbacks[character][i];

                // if a sequence name is not specified, but this is a sequence at
                // the wrong level then move onto the next match
                if (!sequenceName && callback.seq && _sequenceLevels[callback.seq] != callback.level) {
                    continue;
                }

                // if the action we are looking for doesn't match the action we got
                // then we should keep going
                if (action != callback.action) {
                    continue;
                }

                // if this is a keypress event and the meta key and control key
                // are not pressed that means that we need to only look at the
                // character, otherwise check the modifiers as well
                //
                // chrome will not fire a keypress if meta or control is down
                // safari will fire a keypress if meta or meta+shift is down
                // firefox will fire a keypress if meta or control is down
                if ((action == 'keypress' && !e.metaKey && !e.ctrlKey) || _modifiersMatch(modifiers, callback.modifiers)) {

                    // when you bind a combination or sequence a second time it
                    // should overwrite the first one.  if a sequenceName or
                    // combination is specified in this call it does just that
                    //
                    // @todo make deleting its own method?
                    var deleteCombo = !sequenceName && callback.combo == combination;
                    var deleteSequence = sequenceName && callback.seq == sequenceName && callback.level == level;
                    if (deleteCombo || deleteSequence) {
                        self._callbacks[character].splice(i, 1);
                    }

                    matches.push(callback);
                }
            }

            return matches;
        }

        /**
         * actually calls the callback function
         *
         * if your callback function returns false this will use the jquery
         * convention - prevent default and stop propogation on the event
         *
         * @param {Function} callback
         * @param {Event} e
         * @returns void
         */
        function _fireCallback(callback, e, combo, sequence) {

            // if this event should not happen stop here
            if (self.stopCallback(e, e.target || e.srcElement, combo, sequence)) {
                return;
            }

            if (callback(e, combo) === false) {
                _preventDefault(e);
                _stopPropagation(e);
            }
        }

        /**
         * handles a character key event
         *
         * @param {string} character
         * @param {Array} modifiers
         * @param {Event} e
         * @returns void
         */
        self._handleKey = function(character, modifiers, e) {
            var callbacks = _getMatches(character, modifiers, e);
            var i;
            var doNotReset = {};
            var maxLevel = 0;
            var processedSequenceCallback = false;

            // Calculate the maxLevel for sequences so we can only execute the longest callback sequence
            for (i = 0; i < callbacks.length; ++i) {
                if (callbacks[i].seq) {
                    maxLevel = Math.max(maxLevel, callbacks[i].level);
                }
            }

            // loop through matching callbacks for this key event
            for (i = 0; i < callbacks.length; ++i) {

                // fire for all sequence callbacks
                // this is because if for example you have multiple sequences
                // bound such as "g i" and "g t" they both need to fire the
                // callback for matching g cause otherwise you can only ever
                // match the first one
                if (callbacks[i].seq) {

                    // only fire callbacks for the maxLevel to prevent
                    // subsequences from also firing
                    //
                    // for example 'a option b' should not cause 'option b' to fire
                    // even though 'option b' is part of the other sequence
                    //
                    // any sequences that do not match here will be discarded
                    // below by the _resetSequences call
                    if (callbacks[i].level != maxLevel) {
                        continue;
                    }

                    processedSequenceCallback = true;

                    // keep a list of which sequences were matches for later
                    doNotReset[callbacks[i].seq] = 1;
                    _fireCallback(callbacks[i].callback, e, callbacks[i].combo, callbacks[i].seq);
                    continue;
                }

                // if there were no sequence matches but we are still here
                // that means this is a regular match so we should fire that
                if (!processedSequenceCallback) {
                    _fireCallback(callbacks[i].callback, e, callbacks[i].combo);
                }
            }

            // if the key you pressed matches the type of sequence without
            // being a modifier (ie "keyup" or "keypress") then we should
            // reset all sequences that were not matched by this event
            //
            // this is so, for example, if you have the sequence "h a t" and you
            // type "h e a r t" it does not match.  in this case the "e" will
            // cause the sequence to reset
            //
            // modifier keys are ignored because you can have a sequence
            // that contains modifiers such as "enter ctrl+space" and in most
            // cases the modifier key will be pressed before the next key
            //
            // also if you have a sequence such as "ctrl+b a" then pressing the
            // "b" key will trigger a "keypress" and a "keydown"
            //
            // the "keydown" is expected when there is a modifier, but the
            // "keypress" ends up matching the _nextExpectedAction since it occurs
            // after and that causes the sequence to reset
            //
            // we ignore keypresses in a sequence that directly follow a keydown
            // for the same character
            var ignoreThisKeypress = e.type == 'keypress' && _ignoreNextKeypress;
            if (e.type == _nextExpectedAction && !_isModifier(character) && !ignoreThisKeypress) {
                _resetSequences(doNotReset);
            }

            _ignoreNextKeypress = processedSequenceCallback && e.type == 'keydown';
        };

        /**
         * handles a keydown event
         *
         * @param {Event} e
         * @returns void
         */
        function _handleKeyEvent(e) {

            // normalize e.which for key events
            // @see http://stackoverflow.com/questions/4285627/javascript-keycode-vs-charcode-utter-confusion
            if (typeof e.which !== 'number') {
                e.which = e.keyCode;
            }

            var character = _characterFromEvent(e);

            // no character found then stop
            if (!character) {
                return;
            }

            // need to use === for the character check because the character can be 0
            if (e.type == 'keyup' && _ignoreNextKeyup === character) {
                _ignoreNextKeyup = false;
                return;
            }

            self.handleKey(character, _eventModifiers(e), e);
        }

        /**
         * called to set a 1 second timeout on the specified sequence
         *
         * this is so after each key press in the sequence you have 1 second
         * to press the next key before you have to start over
         *
         * @returns void
         */
        function _resetSequenceTimer() {
            clearTimeout(_resetTimer);
            _resetTimer = setTimeout(_resetSequences, 1000);
        }

        /**
         * binds a key sequence to an event
         *
         * @param {string} combo - combo specified in bind call
         * @param {Array} keys
         * @param {Function} callback
         * @param {string=} action
         * @returns void
         */
        function _bindSequence(combo, keys, callback, action) {

            // start off by adding a sequence level record for this combination
            // and setting the level to 0
            _sequenceLevels[combo] = 0;

            /**
             * callback to increase the sequence level for this sequence and reset
             * all other sequences that were active
             *
             * @param {string} nextAction
             * @returns {Function}
             */
            function _increaseSequence(nextAction) {
                return function() {
                    _nextExpectedAction = nextAction;
                    ++_sequenceLevels[combo];
                    _resetSequenceTimer();
                };
            }

            /**
             * wraps the specified callback inside of another function in order
             * to reset all sequence counters as soon as this sequence is done
             *
             * @param {Event} e
             * @returns void
             */
            function _callbackAndReset(e) {
                _fireCallback(callback, e, combo);

                // we should ignore the next key up if the action is key down
                // or keypress.  this is so if you finish a sequence and
                // release the key the final key will not trigger a keyup
                if (action !== 'keyup') {
                    _ignoreNextKeyup = _characterFromEvent(e);
                }

                // weird race condition if a sequence ends with the key
                // another sequence begins with
                setTimeout(_resetSequences, 10);
            }

            // loop through keys one at a time and bind the appropriate callback
            // function.  for any key leading up to the final one it should
            // increase the sequence. after the final, it should reset all sequences
            //
            // if an action is specified in the original bind call then that will
            // be used throughout.  otherwise we will pass the action that the
            // next key in the sequence should match.  this allows a sequence
            // to mix and match keypress and keydown events depending on which
            // ones are better suited to the key provided
            for (var i = 0; i < keys.length; ++i) {
                var isFinal = i + 1 === keys.length;
                var wrappedCallback = isFinal ? _callbackAndReset : _increaseSequence(action || _getKeyInfo(keys[i + 1]).action);
                _bindSingle(keys[i], wrappedCallback, action, combo, i);
            }
        }

        /**
         * binds a single keyboard combination
         *
         * @param {string} combination
         * @param {Function} callback
         * @param {string=} action
         * @param {string=} sequenceName - name of sequence if part of sequence
         * @param {number=} level - what part of the sequence the command is
         * @returns void
         */
        function _bindSingle(combination, callback, action, sequenceName, level) {

            // store a direct mapped reference for use with Mousetrap.trigger
            self._directMap[combination + ':' + action] = callback;

            // make sure multiple spaces in a row become a single space
            combination = combination.replace(/\s+/g, ' ');

            var sequence = combination.split(' ');
            var info;

            // if this pattern is a sequence of keys then run through this method
            // to reprocess each pattern one key at a time
            if (sequence.length > 1) {
                _bindSequence(combination, sequence, callback, action);
                return;
            }

            info = _getKeyInfo(combination, action);

            // make sure to initialize array if this is the first time
            // a callback is added for this key
            self._callbacks[info.key] = self._callbacks[info.key] || [];

            // remove an existing match if there is one
            _getMatches(info.key, info.modifiers, {type: info.action}, sequenceName, combination, level);

            // add this call back to the array
            // if it is a sequence put it at the beginning
            // if not put it at the end
            //
            // this is important because the way these are processed expects
            // the sequence ones to come first
            self._callbacks[info.key][sequenceName ? 'unshift' : 'push']({
                callback: callback,
                modifiers: info.modifiers,
                action: info.action,
                seq: sequenceName,
                level: level,
                combo: combination
            });
        }

        /**
         * binds multiple combinations to the same callback
         *
         * @param {Array} combinations
         * @param {Function} callback
         * @param {string|undefined} action
         * @returns void
         */
        self._bindMultiple = function(combinations, callback, action) {
            for (var i = 0; i < combinations.length; ++i) {
                _bindSingle(combinations[i], callback, action);
            }
        };

        // start!
        _addEvent(targetElement, 'keypress', _handleKeyEvent);
        _addEvent(targetElement, 'keydown', _handleKeyEvent);
        _addEvent(targetElement, 'keyup', _handleKeyEvent);
    }

    /**
     * binds an event to mousetrap
     *
     * can be a single key, a combination of keys separated with +,
     * an array of keys, or a sequence of keys separated by spaces
     *
     * be sure to list the modifier keys first to make sure that the
     * correct key ends up getting bound (the last key in the pattern)
     *
     * @param {string|Array} keys
     * @param {Function} callback
     * @param {string=} action - 'keypress', 'keydown', or 'keyup'
     * @returns void
     */
    Mousetrap.prototype.bind = function(keys, callback, action) {
        var self = this;
        keys = keys instanceof Array ? keys : [keys];
        self._bindMultiple.call(self, keys, callback, action);
        return self;
    };

    /**
     * unbinds an event to mousetrap
     *
     * the unbinding sets the callback function of the specified key combo
     * to an empty function and deletes the corresponding key in the
     * _directMap dict.
     *
     * TODO: actually remove this from the _callbacks dictionary instead
     * of binding an empty function
     *
     * the keycombo+action has to be exactly the same as
     * it was defined in the bind method
     *
     * @param {string|Array} keys
     * @param {string} action
     * @returns void
     */
    Mousetrap.prototype.unbind = function(keys, action) {
        var self = this;
        return self.bind.call(self, keys, function() {}, action);
    };

    /**
     * triggers an event that has already been bound
     *
     * @param {string} keys
     * @param {string=} action
     * @returns void
     */
    Mousetrap.prototype.trigger = function(keys, action) {
        var self = this;
        if (self._directMap[keys + ':' + action]) {
            self._directMap[keys + ':' + action]({}, keys);
        }
        return self;
    };

    /**
     * resets the library back to its initial state.  this is useful
     * if you want to clear out the current keyboard shortcuts and bind
     * new ones - for example if you switch to another page
     *
     * @returns void
     */
    Mousetrap.prototype.reset = function() {
        var self = this;
        self._callbacks = {};
        self._directMap = {};
        return self;
    };

    /**
     * should we stop this event before firing off callbacks
     *
     * @param {Event} e
     * @param {Element} element
     * @return {boolean}
     */
    Mousetrap.prototype.stopCallback = function(e, element) {
        var self = this;

        // if the element has the class "mousetrap" then no need to stop
        if ((' ' + element.className + ' ').indexOf(' mousetrap ') > -1) {
            return false;
        }

        if (_belongsTo(element, self.target)) {
            return false;
        }

        // Events originating from a shadow DOM are re-targetted and `e.target` is the shadow host,
        // not the initial event target in the shadow tree. Note that not all events cross the
        // shadow boundary.
        // For shadow trees with `mode: 'open'`, the initial event target is the first element in
        // the event’s composed path. For shadow trees with `mode: 'closed'`, the initial event
        // target cannot be obtained.
        if ('composedPath' in e && typeof e.composedPath === 'function') {
            // For open shadow trees, update `element` so that the following check works.
            var initialEventTarget = e.composedPath()[0];
            if (initialEventTarget !== e.target) {
                element = initialEventTarget;
            }
        }

        // stop for input, select, and textarea
        return element.tagName == 'INPUT' || element.tagName == 'SELECT' || element.tagName == 'TEXTAREA' || element.isContentEditable;
    };

    /**
     * exposes _handleKey publicly so it can be overwritten by extensions
     */
    Mousetrap.prototype.handleKey = function() {
        var self = this;
        return self._handleKey.apply(self, arguments);
    };

    /**
     * allow custom key mappings
     */
    Mousetrap.addKeycodes = function(object) {
        for (var key in object) {
            if (object.hasOwnProperty(key)) {
                _MAP[key] = object[key];
            }
        }
        _REVERSE_MAP = null;
    };

    /**
     * Init the global mousetrap functions
     *
     * This method is needed to allow the global mousetrap functions to work
     * now that mousetrap is a constructor function.
     */
    Mousetrap.init = function() {
        var documentMousetrap = Mousetrap(document);
        for (var method in documentMousetrap) {
            if (method.charAt(0) !== '_') {
                Mousetrap[method] = (function(method) {
                    return function() {
                        return documentMousetrap[method].apply(documentMousetrap, arguments);
                    };
                } (method));
            }
        }
    };

    Mousetrap.init();

    // expose mousetrap to the global object
    window.Mousetrap = Mousetrap;

    // expose as a common js module
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = Mousetrap;
    }

    // expose mousetrap as an AMD module
    if (true) {
        !(__WEBPACK_AMD_DEFINE_RESULT__ = function() {
            return Mousetrap;
        }.call(exports, __webpack_require__, exports, module),
				__WEBPACK_AMD_DEFINE_RESULT__ !== undefined && (module.exports = __WEBPACK_AMD_DEFINE_RESULT__));
    }
}) (typeof window !== 'undefined' ? window : null, typeof  window !== 'undefined' ? document : null);


/***/ })
/******/ ]);