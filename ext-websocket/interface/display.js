'use strict';

import { Panel, GeneralPanel, ClassPanel } from "./panels.js";
import { download } from "./utils.js";
import { MIN_PAGE } from "./sseq.js";
import { Tooltip } from "./tooltip.js";

export const STATE_ADD_DIFFERENTIAL = 1;
export const STATE_QUERY_TABLE = 2;
export const STATE_QUERY_COCYCLE_STRING = 3;
const OFFSET_SIZE = 0.3;

const DIFFERENTIAL_COLORS = ["cyan", "red", "green"];
const DEFAULT_DIFFERENTIAL_COLOR = "blue";

const NODE_COLOR = {
    "InProgress": "black",
    "Error": "#a6001a",
    "Done": "gray"
};

const gridGo = "go";
const gridChess = "chess";

const DEFAULT_EDGE_STYLE = {
    "color": "black",
    "bend": 0,
    "line-dash": []
}

const MIN_CLASS_SIZE = 20;
const MAX_CLASS_SIZE = 60;

class Display extends EventEmitter {
    // container is either an id (e.g. "#main") or a DOM object
    constructor(container, sseq) {
        super();

        this.leftMargin = 40;
        this.rightMargin = 5;
        this.topMargin = 30;
        this.bottomMargin = 30;
        this.domainOffset = 1 / 2;

        this.specialClasses = new Set();
        this.highlighted = new Set();
        this.tooltip = new Tooltip(this);

        this.gridStyle = gridGo;
        this.gridColor = "#c6c6c6";
        this.gridStrokeWidth = 0.3;
        this.TICK_STEP_LOG_BASE = 1.1; // Used for deciding when to change tick step.

        this.visibleStructlines = new Set(["h_0", "a_0", "h_1", "h_2"]);
        this.structlineStyles = new Map();
        this.updating = false;

        this.container = d3.select(container);
        this.container_DOM = this.container.node();

        this.container.selectAll().remove();

        this.xScaleInit = d3.scaleLinear();
        this.yScaleInit = d3.scaleLinear();

        this.canvas = document.createElement("canvas");
        this.canvas.style.padding = "0px";
        this.canvas.style.position = "absolute";
        this.canvas.style.top = "0";
        this.canvas.style.left = "0";

        this.container_DOM.appendChild(this.canvas);

        this.context = this.canvas.getContext("2d");

        this.update = this.update.bind(this);
        this.nextPage = this.nextPage.bind(this);
        this.previousPage = this.previousPage.bind(this);
        this._onMousemove = this._onMousemove.bind(this);

        this.zoom = d3.zoom().scaleExtent([0, 4]);
        this.zoom.on("zoom", this.update);
        this.zoomD3Element = d3.select(this.canvas);
        this.zoomD3Element.call(this.zoom).on("dblclick.zoom", null);

        this.canvas.addEventListener("mousemove", this._onMousemove);
        this.canvas.addEventListener("click", () => this._onClick(this.mouseCoord));

        this.classScale = 1;

        // TODO: improve window resize handling. Currently the way that the domain changes is suboptimal.
        // I think the best would be to maintain the x and y range by scaling.
        window.addEventListener("resize",  () => this.resize());

        if (sseq) this.setSseq(sseq);
    }

    /**
     *
     * @param width Optional width. Default to 97% of width of bounding element.
     * @param height Optional height. Default to 97% of height of bounding element.
     */
    resize(width, height){
        if (!this.sseq) return;

        let oldxmin = this.xminFloat;
        let oldymin = this.yminFloat;
        // This fixes the scale, but leaves a
        this._initializeCanvas(width, height);
        this._updateScale();
        let dx = this.xminFloat - oldxmin;
        let dy = this.yminFloat - oldymin;
        this.zoom.on("zoom", null);
        this.zoom.translateBy(this.zoomD3Element, this.dxScale(dx), this.dyScale(dy));
        this.zoom.on("zoom", this.update);
        this.update();
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
            this.sseq.removeListener("update", this.update);
        }
        this.sseq = sseq;
        this.sseq.display = this;
        this.pageIdx = 0;
        this.setPage();

        this._initializeScale();
        this._initializeCanvas();

        this.sseq.on('update', this.update);

        for (let name of sseq.products.keys()) {
            this.structlineStyles.set(name, Object.assign({}, DEFAULT_EDGE_STYLE));
        }
        this.sseq.on("new-structline", (name) => {
            this.structlineStyles.set(name, Object.assign({}, DEFAULT_EDGE_STYLE));
        });
        this.update();
    }

    _initializeScale(){
        this.xScaleInit.domain([this.sseq.minDegree - this.domainOffset, this.sseq.maxX + this.domainOffset]);
        this.yScaleInit.domain([0 - this.domainOffset, this.sseq.maxY + this.domainOffset]);
    }

    nextPage(){
        if (this.pageIdx < this.sseq.pageList.length - 1) {
            this.setPage(this.pageIdx + 1);
            this.update();
        }
    }

    previousPage(){
        if (this.pageIdx > 0) {
            this.setPage(this.pageIdx - 1);
            this.update();
        }
    }

    setSpecialClasses(classes) {
        this.specialClasses.clear();
        for (let c of classes) {
            this.specialClasses.add(c.join(" "));
        }
    }

    clearHighlight() {
        this.highlighted.clear();
        if (this.selected) {
            this.highlightClass(this.selected);
        }
    }

    removeHighlight(coord) {
        this.highlighted.delete(coord.join(" "));

        if (this.selected) {
            this.highlightClass(this.selected);
        }
    }

    highlightClass(coord) {
        this.highlighted.add(coord.join(" "));
    }

    /**
     * Update this.page to reflect the value of pageIdx.
     * Eventually I should make a display that indicates the current page again, then this can also say what that is.
     */
    setPage(idx){
        if (!this.sseq) return;

        if(idx !== undefined){
            this.pageIdx = idx;
        }
        this.page = this.sseq.pageList[this.pageIdx];
        this._onClick(this.selected);
    }

    update() {
        if (!this.sseq) return;
        if (this.updating) return;

        this.updating = true;

        requestAnimationFrame(() => {
            this.updating = false;

            this._drawSseq(this.context);
            if (d3.event) {
                // d3 zoom doesn't allow the events it handles to bubble, so we
                // fails to track pointer position.
                this._onMousemove(d3.event);
            } else {
                this._onMousemove();
            }
        });
    }

    clipContext(ctx) {
        ctx.beginPath();
        ctx.globalAlpha = 0; // C2S does not correctly clip unless the clip is stroked.
        ctx.rect(this.leftMargin, this.topMargin, this.plotWidth, this.plotHeight);
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

        this._drawGrid(ctx);
        this._drawStructlines(ctx);
        this._drawDifferentials(ctx);
        this._drawNodes(ctx);

        ctx.restore();
    }

    /**
     * @private
     */
    _updateScale(){
        let zoomD3Element = this.zoomD3Element;
        let transform = d3.zoomTransform(zoomD3Element.node());
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
        if (xScale(this.sseq.maxX - this.sseq.minDegree + 2 * this.domainOffset) - xScale(0) < this.plotWidth) {
            // We simply record the scale was maxed and handle this later
            // by modifying xScale directly.
            xScaleMaxed = true;
        } else if (xScale(this.sseq.minDegree - this.domainOffset) > this.leftMargin) {
            this.zoom.translateBy(zoomD3Element, (this.leftMargin - xScale(this.sseq.minDegree - this.domainOffset)) / scale, 0);
        } else if (xScale(this.sseq.maxX + this.domainOffset) < this.clipWidth) {
            this.zoom.translateBy(zoomD3Element, (this.clipWidth - xScale(this.sseq.maxX + this.domainOffset)) / scale, 0);
        }

        if (yScale(0) -yScale(this.sseq.maxY + 2 * this.domainOffset) < this.plotHeight) {
            yScaleMaxed = true;
        } else if (yScale(-this.domainOffset) < this.clipHeight) {
            this.zoom.translateBy(zoomD3Element, 0, (this.clipHeight - yScale(-this.domainOffset)) / scale);
        } else if (yScale(this.sseq.maxY + this.domainOffset) > this.topMargin) {
            this.zoom.translateBy(zoomD3Element, 0, this.topMargin - yScale(this.sseq.maxY + this.domainOffset) / scale);
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
                this.zoom.on("zoom", this.update);
                return;
            } else {
                this.oldScalesMaxed = true;
            }
        } else {
            this.oldScalesMaxed = false;
        }

        // Get new transform and scale objects after possible translation above
        this.transform = d3.zoomTransform(zoomD3Element.node());
        this.scale = this.transform.k;
        this.xScale = this.transform.rescaleX(this.xScaleInit);
        this.yScale = this.transform.rescaleY(this.yScaleInit);

        // If x or y scale is maxed, we directly override xScale/yScale instead
        // of messing with zoom, since we want to continue allow zooming in the
        // other direction
        if (xScaleMaxed) {
            this.xScale.domain([
                this.sseq.minDegree - this.domainOffset,
                this.sseq.maxX + this.domainOffset
            ]);
        }
        if (yScaleMaxed) {
            this.yScale.domain([
                -this.domainOffset,
                this.sseq.maxY + this.domainOffset
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

        this.zoom.on("zoom", this.update);
    }

    dxScale(x){
        return this.xScale(x) - this.xScale(0);
    }

    dyScale(x){
        return this.yScale(x) - this.yScale(0);
    }

    // Computes the canvas coordinates of the ith class of (x, y) when there
    // are num classes in total. If i and num are not specified, this returns the coordinates of the point (x, y).
    sseqToCanvas(x, y, i, num) {
        if (i === undefined && num === undefined) {
            i = 0;
            num = 1;
        }

        return [this.xScale(x + (i - (num - 1)/2) * OFFSET_SIZE), this.yScale(y)];
    }

    canvasToSseq(x, y) {
        return [Math.round(this.xScale.invert(x)), Math.round(this.yScale.invert(y))];
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

        this.xGridStep = (Math.floor(this.xTickStep / 5) === 0) ? 1 : Math.floor(this.xTickStep / 5);
        this.yGridStep = (Math.floor(this.yTickStep / 5) === 0) ? 1 : Math.floor(this.yTickStep / 5);

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
            case gridGo:
                this._drawGridWithOffset(context, 0, 0);
                break;
            case gridChess:
                this._drawGridWithOffset(context, 0.5, 0.5);
                break;
            default:
                // TODO: an error here?
                break;
        }

        context.restore();
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
        // This prevents axes labels from appearing to the left or below the
        // axes intercept.
        context.clearRect(0, 0, this.leftMargin, this.topMargin);
        context.clearRect(0, this.clipHeight, this.leftMargin, this.bottomMargin);

        context.save();

        // Draw the axes.
        context.beginPath();
        context.moveTo(this.leftMargin, this.topMargin);
        context.lineTo(this.leftMargin, this.clipHeight);
        context.lineTo(this.canvasWidth - this.rightMargin, this.clipHeight);
        context.stroke();

        context.restore();
    }

    _drawNodes(context) {
        context.save();

        let size = Math.max(Math.min(this.dxScale(1), -this.dyScale(1), MAX_CLASS_SIZE), MIN_CLASS_SIZE) * this.classScale;

        for (let x = this.xmin - 1; x < this.xmax + 1; x++) {
            for (let y = this.ymin - 1; y < this.ymax + 1; y++) {

                let classes = this.sseq.getClasses(x, y, this.page);
                if (classes === undefined) {
                    continue;
                }

                if (this.highlighted.has(`${x} ${y}`)) {
                    context.fillStyle = "red";
                } else if (this.specialClasses.has(`${x} ${y}`)) {
                    context.fillStyle = "#ff7f00";
                } else {
                    context.fillStyle = NODE_COLOR[this.sseq.classState.get(x,y)];
                }

                let num = classes.length;
                for (let i = 0; i < num; i++) {
                    let [x_, y_] = this.sseqToCanvas(x, y, i, num);

                    context.beginPath();
                    context.arc(x_, y_, size * 0.1, 0, 2 * Math.PI);
                    context.fill();
                }
            }
        }
        context.restore();
    }

    _drawStructlines(context) {
        for (let [name, mult] of this.sseq.products) {
            if (!this.visibleStructlines.has(name))
                continue;

            context.save();
            let style = this.structlineStyles.get(name);
            context.strokeStyle = style.color;
            context.setLineDash(style["line-dash"]);

            for (let x = this.xmin - 1 - mult.x; x < this.xmax + 1; x++) {
                for (let y = this.ymin - 1 - mult.y; y < this.ymax + 1; y++) {
                    let matrices = mult.matrices.get(x, y);
                    if (matrices === undefined || matrices === null)
                        continue;

                    let pageIdx = Math.min(matrices.length - 1, this.page - MIN_PAGE);
                    let matrix = matrices[pageIdx];
                    if (matrix === undefined)
                        continue;

                    let sourceDim = matrix.length;
                    if (sourceDim == 0) continue;

                    let targetDim = matrix[0].length;
                    if (targetDim == 0) continue;

                    for (let i = 0; i < sourceDim; i++) {
                        for (let j = 0; j < targetDim; j++) {
                            if (matrix[i][j] != 0) {
                                let [sourceX, sourceY] = this.sseqToCanvas(x, y, i, sourceDim);
                                let [targetX, targetY] = this.sseqToCanvas(x + mult.x, y + mult.y, j, targetDim);

                                context.beginPath();
                                if (style.bend != 0) {
                                    let distance = Math.sqrt((targetX - sourceX)*(targetX - sourceX) + (targetY - sourceY)*(targetY - sourceY));
                                    let looseness = 0.4;
                                    let angle = Math.atan((targetY - sourceY)/(targetX - sourceX));
                                    let bendAngle = - style.bend * Math.PI/180;
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
                            }
                        }
                    }
                }
            }
            context.restore();
        }
    }

    _drawDifferentials(context) {
        context.save();
        if (DIFFERENTIAL_COLORS[this.page - MIN_PAGE]) {
            context.strokeStyle = DIFFERENTIAL_COLORS[this.page - MIN_PAGE];
        } else {
            context.strokeStyle = DEFAULT_DIFFERENTIAL_COLOR;
        }

        for (let x = this.xmin - 1; x < this.xmax + 2; x++) {
            for (let y = this.ymin - 1 - this.page; y < this.ymax + 1 + this.page; y++) {
                let matrix = this.sseq.getDifferentials(x, y, this.page);
                if (matrix === undefined) continue;

                let sourceDim = matrix.length;
                if (sourceDim == 0) continue;

                let targetDim = matrix[0].length;
                if (targetDim == 0) continue;

                for (let i = 0; i < sourceDim; i++) {
                    for (let j = 0; j < targetDim; j++) {
                        if (matrix[i][j] != 0) {
                            let [sourceX, sourceY] = this.sseqToCanvas(x, y, i, sourceDim);
                            let [targetX, targetY] = this.sseqToCanvas(x - 1, y + this.page, j, targetDim);

                            context.beginPath();
                            context.moveTo(sourceX, sourceY);
                            context.lineTo(targetX, targetY);
                            context.stroke();
                        }
                    }
                }
            }
        }
        context.restore();
    }

    _onClick(coord) {
        if (!coord)
            return;

        let oldSelected = this.selected;
        this.selected = null;

        if (this.sseq.hasClasses(...coord, this.page)) {
            this.selected = Array.from(coord); // Copy
            this.highlightClass(this.selected);
        }

        // removeHighlight doesn't remove oldSelected if oldSelected = this.selected.
        if (oldSelected)
            this.removeHighlight(oldSelected);

        this.emit("click", oldSelected);

        this.update();
    }

    _onMousemove(e) {
        // Avoid some mysterious race condition.
        if (!this.xScale)
            return;

        // We cannot query for mouse position. We must remember it from
        // previous events. If update() is called, we call _onMousemove without
        // an event.
        let rect = this.canvas.getBoundingClientRect();
        let mouseCoord;
        if (e) {
            mouseCoord = this.canvasToSseq(e.clientX - rect.x, e.clientY - rect.y);
        } else {
            mouseCoord = this.mouseCoord;
        }

        if (mouseCoord === undefined) return;

        // We changed position!
        if (this.mouseCoord === undefined ||
             ((mouseCoord[0] != this.mouseCoord[0] || mouseCoord[1] != this.mouseCoord[1]) &&
                 (this.mouseCoord === undefined || this.sseq.hasClasses(...mouseCoord, this.page) || this.sseq.hasClasses(...this.mouseCoord, this.page)))) {
            if (this.mouseCoord) {
                this.removeHighlight(this.mouseCoord);
            }
            this.mouseCoord = mouseCoord;
            this.highlightClass(this.mouseCoord);

            if (!this.sseq.hasClasses(mouseCoord[0], mouseCoord[1], this.page)) {
                this.tooltip.hide();
            } else {
                this.tooltip.setHTML(`(${mouseCoord[0]}, ${mouseCoord[1]})`);
                this.tooltip.show(...this.sseqToCanvas(mouseCoord[0], mouseCoord[1]));
            }

            this.update();
        } else {
            this.mouseCoord = mouseCoord;
        }
    }

    toSVG(){
        let ctx = new C2S(this.canvasWidth, this.canvasHeight);
        this._drawSseq(ctx);

        return ctx.getSerializedSvg(true);
    }

    downloadSVG() {
        let filename = prompt("File name");
        if (filename === null) {
            return;
        }
        filename = filename.trim();
        if (!filename.endsWith(".svg"))
            filename += ".svg";
        download(filename, this.toSVG(), "image/svg+xml")
    }
}

class Sidebar {
    constructor(parentContainer) {
        this.adjuster = document.createElement("div");
//        this.adjuster.style.backgroundColor = "rgba(0,0,0,0.125)";
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
        this.sidebar.style.width = "250px";
        this.sidebar.style.display = "flex";
        this.sidebar.style.flexDirection = "column";
        this.sidebar.className = "sidebar";

        parentContainer.appendChild(this.sidebar);

        this.mainDiv = document.createElement("div");
        this.mainDiv.style.overflow = "auto";
        this.mainDiv.style.flexGrow = "1";
        this.sidebar.appendChild(this.mainDiv);

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
        this.footer = new Panel(this.footer_div, display);
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

export class SidebarDisplay extends Display {
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

        Mousetrap.bind('left',  this.previousPage);
        Mousetrap.bind('right', this.nextPage);
    }
}

export class MainDisplay extends SidebarDisplay {
    constructor(container, sseq, isUnit) {
        super(container, sseq);

        this.isUnit = isUnit;
        this.selected = null;
        this.on("mouseout", this._onMouseout.bind(this));
        this.on("click", this.__onClick.bind(this));

        this.generalPanel = new GeneralPanel(this.sidebar.mainDiv, this);
        this.sidebar.addPanel(this.generalPanel);
        this.sidebar.currentPanel = this.generalPanel;

        this.classPanel = new ClassPanel(this.sidebar.mainDiv, this);
        this.sidebar.addPanel(this.classPanel);

        this.sidebar.footer.newGroup();
        this.sidebar.footer.currentGroup.style.textAlign = "center";
        this.runningSign = document.createElement("p");
        this.runningSign.innerHTML = "Running...";
        this.sidebar.footer.addObject(this.runningSign);

        this.sidebar.footer.addButtonRow([
            ["Undo", () => this.sseq.undo()],
            ["Redo", () => this.sseq.redo()]
        ]);

        this.sidebar.footer.addButton("Download SVG", () => this.downloadSVG());
        this.sidebar.footer.addButton("Download Snapshots", () => this.sseq.downloadHistoryList());
        this.sidebar.footer.addButtonRow([
            ["Save", () => window.save()],
            ["Link", () => alert("Link to calculation:\n\n" + window.getHistoryLink())],
        ]);

        Mousetrap.bind("J", () => this.sidebar.currentPanel.prevTab());
        Mousetrap.bind("K", () => this.sidebar.currentPanel.nextTab());
        Mousetrap.bind("d", () => this.state = STATE_ADD_DIFFERENTIAL);
        Mousetrap.bind("p", () => {
            if (this.selected)
                this.sseq.addPermanentClassInteractive(...this.selected);
        });
        Mousetrap.bind("y", () => this.state = STATE_QUERY_TABLE);
        Mousetrap.bind("x", () => this.state = STATE_QUERY_COCYCLE_STRING);

        Mousetrap.bind("n", () => {
            if (!this.selected) return;
            let [x, y] = this.selected;
            let num = this.sseq.getClasses(x, y, MIN_PAGE).length;

            let idx = 0;
            if (num != 1) {
                while(true) {
                    idx = prompt("Class index");
                    if (idx === null)
                        return;

                    idx = parseInt(idx);
                    if (Number.isNaN(idx) || idx >= num || idx < 0) {
                        alert(`Invalid index. Enter integer between 0 and ${num} (inclusive)`);
                    } else {
                        break;
                    }
                }
            }

            let name = prompt("New class name");
            if (name !== null) {
                sseq.setClassName(x, y, idx, name);
            }
        });


        sseq.on("update", (x, y) => { if (this.selected && this.selected[0] == x && this.selected[1] == y) this.sidebar.showPanel() });
    }

    _onMouseover(node) {
        this.tooltip.setHTML(`(${node.x}, ${node.y})`);
        this.tooltip.show(node.canvas_x, node.canvas_y);
    }

    __onClick(oldSelected) {
        if (this.state == STATE_QUERY_TABLE) {
            this.sseq.queryTable(...this.mouseCoord);
            this.state = null;
            return;
        }
        if (this.state == STATE_QUERY_COCYCLE_STRING) {
            this.sseq.queryCocycleString(...this.mouseCoord);
            this.state = null;
            return;
        }


        if (!this.selected) {
            this._unselect();
            return;
        }

        switch (this.state) {
            case STATE_ADD_DIFFERENTIAL:
                if (oldSelected && oldSelected[0] == this.selected[0] + 1 && this.selected[1] - oldSelected[1] >= MIN_PAGE) {
                    this.sseq.addDifferentialInteractive(oldSelected, this.selected);
                    this.state = null;
                    this._onClick(oldSelected);
                    break;
                }
        }

        this.sidebar.showPanel(this.classPanel);
    }

    _onMouseout() {
        if (this.selected)
            this.highlightClass(this.selected.x, this.selected.y);
        this.tooltip.hide();
    }


    _unselect() {
        this.selected = null;
        this.state = null;
        this.clearHighlight();

        this.sidebar.showPanel(this.generalPanel);
        this.update();
    }

    setSseq(sseq) {
        super.setSseq(sseq);

        sseq.on("new-structline", () => {
            this.sidebar.showPanel()
        });
    }
}

export class UnitDisplay extends Display {
    constructor(container, sseq) {
        super(container, sseq);

        document.querySelectorAll(".close-modal").forEach((c) => {
            c.addEventListener("click", this.closeModal.bind(this));
        });

        document.querySelector("#modal-diff").addEventListener("click", () => {
            document.querySelector("#modal-title").innerHTML = "Select target element";
            this.state = STATE_ADD_DIFFERENTIAL;
        });

        document.querySelector("#modal-ok").addEventListener("click", () => {
            let [x, y] = this.selected;
            let num = this.sseq.getClasses(x, y, MIN_PAGE).length;
            window.mainSseq.addProductInteractive(x, y, num);
            this.closeModal();
        });

        document.querySelector("#modal-more").addEventListener("click", () => this.sseq.resolveFurther());
        document.querySelector("#modal-more").addEventListener("mouseup", () => document.querySelector("#modal-more").blur());

        this.on("click", this.__onClick.bind(this));
    }

    openModal() {
        this._unselect();
        this.sseq.resolveFurther(10);
        document.querySelector("#overlay").style.removeProperty("display");
        document.querySelector("#modal-ok").disabled = true;
        document.querySelector("#modal-diff").disabled = true;
        let dialog = document.querySelector("#unitsseq-dialog");
        dialog.classList.add("modal-shown");
    }

    closeModal() {
        document.querySelector("#overlay").style.display = "none";
        let dialog = document.querySelector("#unitsseq-dialog");
        dialog.classList.remove("modal-shown");
        this.selected = null;
        this._unselect();
        this.tooltip.hide();
    }

    __onClick(oldSelected) {
        if (!this.selected) {
            this._unselect();
            return;
        }

        if (this.state == STATE_ADD_DIFFERENTIAL) {
            if (this.selected[0] == oldSelected[0] - 1 && this.selected[1] - oldSelected[1] >= MIN_PAGE) {
                let check = confirm(`Add differential from (${oldSelected[0]}, ${oldSelected[1]}) to (${this.selected[0]}, ${this.selected[1]})?`);
                if (check) {
                    this.sseq.addProductDifferentialInteractive(oldSelected[0], oldSelected[1], this.selected[1] - oldSelected[1]);
                    this.state = null;
                    this.closeModal();
                }
            } else {
                alert("Invalid target for differential");
            }
        } else {
            this.state = null;
        }
        document.querySelector("#modal-ok").disabled = false;
        document.querySelector("#modal-diff").disabled = false;
    }

    _unselect() {
        this.state = null;

        this.clearHighlight();
        this.update();
        document.querySelector("#modal-title").innerHTML = "Select element to multiply with";
        document.querySelector("#modal-ok").disabled = true;
        document.querySelector("#modal-diff").disabled = true;
    }
}
