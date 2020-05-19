"use strict";

import * as EventEmitter from "events";
import * as d3 from "d3";
import { INFINITY } from "../infinity.js";

const GridEnum = Object.freeze({ go : 1, chess : 2 });

function groupByArray(xs, key) { 
    return xs.reduce(
        function reducer(rv, x) { 
            let v = key instanceof Function ? key(x) : x[key]; 
            let el = rv.find((r) => r && r.key === v); 
            if(el) { 
                el.values.push(x); 
            } else { 
                rv.push({ key: v, values: [x] }); 
            } 
            return rv; 
        }, 
        []
    ); 
} 


export class Display extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({mode: 'open'});
        let slot = document.createElement("slot");
        this.shadowRoot.appendChild(slot);

        this._leftMargin = 40;
        this._rightMargin = 5;
        this._topMargin = 45;
        this._bottomMargin = 50;
        this._domainOffset = 1 / 2;

        this.gridStyle = GridEnum.go;
        this.gridColor = "#c6c6c6";
        this.background_color = "#FFFFFF";
        this.gridStrokeWidth = 0.3;
        this.TICK_STEP_LOG_BASE = 1.1; // Used for deciding when to change tick step.
        this.bidegreeDistanceThreshold = 15;

        this.hiddenStructlines = new Set();
        this.updateQueue = 0;

        this.xScaleInit = d3.scaleLinear();
        this.yScaleInit = d3.scaleLinear();

        this.canvas = document.createElement("canvas");
        this.canvas.style.padding = "0px";
        this.canvas.style.position = "absolute";
        this.canvas.style.top = "0";
        this.canvas.style.left = "0";

        this.shadowRoot.appendChild(this.canvas);

        this.eventsElement = document.createElement("div");

        this.context = this.canvas.getContext("2d");
        this.node_buffers = {};

        this.handleZoom = this.handleZoom.bind(this);
        this.nextPage = this.nextPage.bind(this);
        this.previousPage = this.previousPage.bind(this);
        this._emitMouseover = this._emitMouseover.bind(this);
        this._emitClick = this._emitClick.bind(this);
        this.zoom = d3.zoom().scaleExtent([0, 4]);
        this.zoom.on("zoom", this.handleZoom);
        this.zoomD3Element = d3.select(this.canvas);
        this.zoomD3Element.call(this.zoom).on("dblclick.zoom", null);
        this.zoom.on("start", () => {
            this.updatingZoom = true;
        });
        this.zoom.on("end", () => {
            this.updatingZoom = false;
            this._emitMouseover();
        });

        this.canvas.addEventListener("mousemove", this._emitMouseover);
        this.canvas.addEventListener("click", this._emitClick);

        // TODO: improve window resize handling. Currently the way that the domain changes is suboptimal.
        // I think the best would be to maintain the x and y range by scaling.
        this._resizeObserver = new ResizeObserver(entries => {
            for(let e of entries){
                requestAnimationFrame(() => e.target.resize());
            }
        });
        this._resizeObserver.observe(this);
    }

    setBackgroundColor(color) {
        this.background_color = color;
        this.shadowRoot.style.background = color;
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
        this.zoom.on("zoom", this.handleZoom);
        this.update(); // Make sure this is update(), updateBatch() causes screen flicker.
    }

    /**
     * Initialization method called in constructor.
     * @private
     */
    _initializeCanvas(width, height){
        let computedStyle = getComputedStyle(this);
        // computed_width will look like "####px", need to get rid of "px".
        let computedWidth = Number.parseFloat(computedStyle.width.slice(0,-2)); 
        let computedHeight = Number.parseFloat(computedStyle.height.slice(0,-2)); 
        const canvasWidth = width || 0.99*computedWidth;
        const canvasHeight = height || 0.97*computedHeight;

        this._canvasWidth = canvasWidth;
        this._canvasHeight = canvasHeight;

        this.canvas.width = canvasWidth;
        this.canvas.height = canvasHeight;

        this._clipWidth = this._canvasWidth - this._rightMargin;
        this._clipHeight = this._canvasHeight - this._bottomMargin;

        this._plotWidth = this._canvasWidth - this._leftMargin - this._rightMargin;
        this._plotHeight = this._canvasHeight - this._bottomMargin - this._topMargin;

        this.xScaleInit = this.xScaleInit.range([this._leftMargin, this._clipWidth]);
        this.yScaleInit = this.yScaleInit.range([this._clipHeight, this._topMargin]);
    }

    emit(event, ...args){
        let myEvent = new CustomEvent(event, { 
            detail: args,
            bubbles: true, 
            composed: true 
        });
        this.dispatchEvent(myEvent);
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

        this.sseq.on('update', this.updateBatch);
        this.update();
    }

    _initializeScale(){
        this.xScaleInit.domain([this.sseq.initial_x_range[0] - this._domainOffset, this.sseq.initial_x_range[1] + this._domainOffset]);
        this.yScaleInit.domain([this.sseq.initial_y_range[0] - this._domainOffset, this.sseq.initial_y_range[1] + this._domainOffset]);
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

    handleZoom(){
        this.updateMousePosition(d3.event.sourceEvent);
        this.updateBatch();
    }

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
            this._emitMouseover();
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
        ctx.rect(this._leftMargin, this._topMargin + y_clip_offset, this._plotWidth, this._plotHeight - y_clip_offset);
        ctx.stroke();
        ctx.clip();
        ctx.globalAlpha = 1;
    }

    _drawSseq(ctx = this.context) {
        if (!this.sseq) return;
        this.total_draws = this.total_draws + 1 || 0;
        let startTime = performance.now();

        this._updateScale();
        this._updateGridAndTickStep();

        let [classes, edges] = this.sseq.getElementsToDraw(
            this.pageRange, 
            this.xmin - 1, this.xmax + 1, this.ymin - 1, this.ymax + 1
        );
        this._updateClassPositions(classes);

        ctx.clearRect(0, 0, this._canvasWidth, this._canvasHeight);

        this._drawTicks(ctx);
        this._drawAxes(ctx);

        ctx.save();

        this.clipContext(ctx);
        this._drawGrid(ctx);

        this.emit("draw_background");
        this._highlightClasses(ctx);
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
                this._canvasWidth / (this.xmaxFloat - this.xminFloat) * (this.sseq.x_range[1] - this.sseq.x_range[0] + 1);
            let default_height = 
                this._canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.sseq.y_range[1] - this.sseq.y_range[0] + 1);
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
        let dt = performance.now() - startTime;
        // console.log("elapsed", dt/1000, "fps:", 1000/dt);
    }

    /**
     * @private
     */
    _updateScale(){
        let zoomD3Element = this.zoomD3Element;
        let transform = d3.zoomTransform(zoomD3Element.node());
        let originalTransform = transform;
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
        let autoTranslated = false;
        // Prevent user from panning off the side.
        if (this.sseq.x_range) {
            if (xScale(this.sseq.x_range[1] - this.sseq.x_range[0] + 2 * this._domainOffset) - xScale(0) < this._plotWidth) {
                // We simply record the scale was maxed and handle this later
                // by modifying xScale directly.
                xScaleMaxed = true;
            } else if (xScale(this.sseq.x_range[0] - this._domainOffset) > this._leftMargin) {
                this.zoom.translateBy(zoomD3Element, (this._leftMargin - xScale(this.sseq.x_range[0] - this._domainOffset)) / scale, 0);
                autoTranslated = true;
            } else if (xScale(this.sseq.x_range[1] + this._domainOffset) < this._clipWidth) {
                this.zoom.translateBy(zoomD3Element, (this._clipWidth - xScale(this.sseq.x_range[1] + this._domainOffset)) / scale, 0);
                autoTranslated = true;
            }
        }

        if (this.sseq.y_range) {
            if (yScale(0) -yScale(this.sseq.y_range[1] - this.sseq.y_range[0] + 2 * this._domainOffset) < this._plotHeight) {
                yScaleMaxed = true;
            } else if (yScale(this.sseq.y_range[0] - this._domainOffset) < this._clipHeight) {
                this.zoom.translateBy(zoomD3Element, 0, (this._clipHeight - yScale(this.sseq.y_range[0] - this._domainOffset)) / scale);
                autoTranslated = true;
            } else if (yScale(this.sseq.y_range[1] + this._domainOffset) > this._topMargin) {
                this.zoom.translateBy(zoomD3Element, 0, this._topMargin - yScale(this.sseq.y_range[1] + this._domainOffset) / scale);
                autoTranslated = true;
            }
        }

        let oldXScaleMaxed = this.xScaleMaxed;
        let oldYScaleMaxed = this.yScaleMaxed;
        this.xScaleMaxed = xScaleMaxed;
        this.yScaleMaxed = yScaleMaxed;
        let scalesMaxed = (xScaleMaxed && yScaleMaxed);
        let oldScalesMaxed = (oldXScaleMaxed && oldYScaleMaxed);

        // If both scales are maxed, and the user attempts to zoom out further,
        // d3 registers a zoom, but nothing in the interface changes since we
        // manually override xScale and yScale instead of doing something at
        // the level of the transform (see below). We do *not* want to keep
        // zooming out, or else when the user wants to zoom back in, they will
        // have to zoom in for a while before the interface actually zooms in.
        // Thus, We restore the previous zoom state.
        if (scalesMaxed && oldScalesMaxed && scale < this.scale) {
            this.zoom.transform(zoomD3Element, this.transform);
            this.zoom.on("zoom", this.handleZoom);
            this.disableMouseoverUpdates = true;
            return;
        }

        // Get new transform and scale objects after possible translation above
        transform = d3.zoomTransform(zoomD3Element.node());
        let old_transform = this.transform;
        this.transform = transform;
        this.scale = this.transform.k;
        this.xScale = this.transform.rescaleX(this.xScaleInit);
        this.yScale = this.transform.rescaleY(this.yScaleInit);

        // If x or y scale is maxed, we directly override xScale/yScale instead
        // of messing with zoom, since we want to continue allow zooming in the
        // other direction
        if (xScaleMaxed) {
            this.xScale.domain([
                this.sseq.x_range[0] - this._domainOffset,
                this.sseq.x_range[1] + this._domainOffset
            ]);
            this.transform.x = old_transform.x;
        }
        if (yScaleMaxed) {
            this.yScale.domain([
                this.sseq.y_range[0] - this._domainOffset,
                this.sseq.y_range[1] + this._domainOffset
            ]);
            this.transform.y = old_transform.y;
        }
        this.zoom.transform(zoomD3Element, this.transform);
        if(old_transform){
            let updatedScale = old_transform.k !== transform.k;
            let updatedTranslation =  old_transform.x != transform.x || old_transform.y != transform.y;
            let previousUpdatedTranslation = this.updatedTranslation;
            this.updatedTranslation = updatedTranslation;
            let revertedDistance = Math.abs(originalTransform.x - transform.x) + Math.abs(originalTransform.y - transform.y);
            this.disableMouseoverUpdates = 
                ((updatedScale && autoTranslated) || updatedTranslation || previousUpdatedTranslation )
                && (oldXScaleMaxed == xScaleMaxed)
                && (oldYScaleMaxed == yScaleMaxed)
                && revertedDistance < 5;
        }

        this.xminFloat = this.xScale.invert(this._leftMargin);
        this.xmaxFloat = this.xScale.invert(this._clipWidth);
        this.yminFloat = this.yScale.invert(this._clipHeight);
        this.ymaxFloat = this.yScale.invert(this._topMargin);
        this.xmin = Math.ceil(this.xminFloat);
        this.xmax = Math.floor(this.xmaxFloat);
        this.ymin = Math.ceil(this.yminFloat);
        this.ymax = Math.floor(this.ymaxFloat);

        this.zoom.on("zoom", this.handleZoom);
    }

    dxScale(x){
        return this.xScale(x) - this.xScale(0);
    }

    dyScale(x){
        return this.yScale(x) - this.yScale(0);
    }

    _updateGridAndTickStep(){
        // TODO: This 70 is a magic number. Maybe I should give it a name?
        this.xTicks = this.xScale.ticks(this._canvasWidth / 70);
        this.yTicks = this.yScale.ticks(this._canvasHeight / 70);

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
            context.fillText(i, this.xScale(i), this._clipHeight + 20);
        }

        context.textAlign = "right";
        for (let i = Math.floor(this.yTicks[0]); i <= this.yTicks[this.yTicks.length - 1]; i += this.yTickStep) {
            context.fillText(i, this._leftMargin - 10, this.yScale(i));
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
            context.lineTo(this.xScale(col), this._clipHeight);
        }
        context.stroke();

        context.beginPath();
        for (let row = Math.floor(this.ymin / this.yGridStep) * this.yGridStep - yoffset; row <= this.ymax; row += this.yGridStep) {
            context.moveTo(this._leftMargin, this.yScale(row));
            context.lineTo(this._canvasWidth - this._rightMargin, this.yScale(row));
        }
        context.stroke();
    }

    _drawAxes(context){
        context.save();

        // This makes the white square in the bottom left and top right corners which prevents axes labels from appearing to the left
        // or below the axes intercept.
        context.fillStyle = this.background_color;
        context.rect(0, this._clipHeight, this._leftMargin, this._bottomMargin);
        context.rect(0, 0, this._leftMargin, this._topMargin);
        context.fill();
        context.fillStyle = "#000";

        // Draw the axes.
        context.beginPath();
        context.moveTo(this._leftMargin, this._topMargin);
        context.lineTo(this._leftMargin, this._clipHeight);
        context.lineTo(this._canvasWidth - this._rightMargin, this._clipHeight);
        context.stroke();

        context.restore();
    }

    _updateClassPositions(classes){
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

    _highlightClasses(context) {
        for (let c of this.classes_to_draw) {
            if(c._highlight){
                c.drawHighlight(context);
            }
        }
    }

    _drawClasses(context) {
        const BUFFER_WIDTH = 64;
        const BUFFER_HEIGHT = 64;
        let groupedClasses = groupByArray(this.classes_to_draw, c => JSON.stringify(c._getStyleForCanvasContext()))

        for(let classGroup of groupedClasses){
            let buffer;
            if(classGroup.key in this.node_buffers){
                buffer = this.node_buffers[classGroup.key];
                let bufferCtx = buffer.getContext('2d');
                bufferCtx.clearRect(0, 0, BUFFER_WIDTH, BUFFER_HEIGHT);
            } else {
                buffer = document.createElement('canvas');
                buffer.width = BUFFER_WIDTH;
                buffer.height = BUFFER_HEIGHT;
                this.node_buffers[classGroup.key] = buffer;
            }
            let bufferCtx = buffer.getContext('2d');
            let firstClass = classGroup.values[0];
            firstClass.draw(bufferCtx, BUFFER_WIDTH/2, BUFFER_HEIGHT/2);
            let path = firstClass.getMouseoverPath(0, 0);
                   
            for(let c of classGroup.values) {
                c._path = path;
                context.drawImage(buffer, c._canvas_x - BUFFER_WIDTH/2, c._canvas_y - BUFFER_HEIGHT/2);
            }
        }
    }

    _drawEdges(context, edges){
        let grouped_edges = groupByArray(edges, (e) => JSON.stringify([e.color, e.lineWidth, e.opacity, e.dash]));
        for(let edge_group of grouped_edges){
            context.save();
            let first_edge = edge_group.values[0];
            context.strokeStyle = first_edge.color || "black";
            if(first_edge.lineWidth){
                context.lineWidth = first_edge.lineWidth;
            }
            if(first_edge.opacity){
                context.globalAlpha = first_edge.opacity;
            }
            if(first_edge.dash){
                context.setLineDash(first_edge.dash);
            }            
            context.beginPath();
            for (let e of edge_group.values) {
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

                let sourceX = source_node._canvas_x + e._sourceOffset.x;
                let sourceY = source_node._canvas_y + e._sourceOffset.y;
                let targetX = target_node._canvas_x + e._targetOffset.x;
                let targetY = target_node._canvas_y + e._targetOffset.y;
                
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
            }
            context.stroke();
            context.restore();
        }
    }

    getMouseState(){
        let o = {};
        o.screen_x = this.mousex;
        o.screen_y = this.mousey;
        o.real_x = this.xScale.invert(this.mousex);
        o.real_y = this.yScale.invert(this.mousey);
        o.x = Math.round(o.real_x);
        o.y = Math.round(o.real_y);
        o.screen_lattice_x = this.xScale(o.x);
        o.screen_lattice_y = this.yScale(o.y);
        let dx = o.x - o.real_x;
        let dy = o.y - o.real_y;
        o.distance = Math.sqrt(dx*dx + dy*dy);
        o.mouseover_class = this.mouseover_class;
        o.mouseover_bidegree = this.mouseover_bidegree;
        return o;
    }

    updateMousePosition(e){
        this.mousex = e.layerX;
        this.mousey = e.layerY;
        this.mouseState = this.getMouseState();
    }

    _emitClick(e) {
        e.stopPropagation();
        let o = this.getMouseState();
        o.event = e;
        this.emit("click", o);
    }

    _emitMouseover(e, redraw) {
        // If not yet set up, updateMousePosition will throw an error.
        if(!this.classes_to_draw){
            return;
        }

        // We cannot query for mouse position. We must remember it from
        // previous events. If update() is called, we call _onMousemove without
        // an event.
        if(e) {
            this.updateMousePosition(e);
        }

        // Don't emit mouseover or mouseout 
        if(this.updatingZoom && this.disableMouseoverUpdates) {
            return;
        }

        redraw = redraw | false;
        redraw |= this._emitMouseoverClass();
        redraw |= this._emitMouseoverBidegree();

        if (redraw) {
            this._drawSseq(this.context);  
        } 
    }

    getMouseoverClass(x, y){
        return this.classes_to_draw.find((c) => 
            this.context.isPointInPath(c._path, this.mousex - c._canvas_x, this.mousey - c._canvas_y)
        );
    }

    _emitMouseoverClass(){
        let new_mouseover_class = this.getMouseoverClass(this.mousex, this.mousey);
        let redraw = false;
        if (this.mouseover_class) {
            if(new_mouseover_class === this.mouseover_class) {
                return false;
            } else {
                this.emit("mouseout-class", this.mouseover_class, this.mouseState);
                this.mouseover_class = null;
                redraw = true;
            }
        }
        if(new_mouseover_class) {
            redraw = true;
            this.mouseover_class = new_mouseover_class;
            this.emit("mouseover-class", new_mouseover_class, this.mouseState);
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
                this.emit("mouseout-bidegree", this.mouseover_bidegree, this.mouseState);
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
            this.emit("mouseover-bidegree", bidegree, this.mouseState);
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
            this._canvasWidth  / (this.xmaxFloat - this.xminFloat) * (this.sseq.x_range[1] - this.sseq.x_range[0] + 1),
            this._canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.sseq.y_range[1] - this.sseq.y_range[0] + 1)
        );
    }

    toSVG(){
        let ctx = new C2S(this._canvasWidth, this._canvasHeight);
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
            let t = d3.interval(() => {
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
        this.zoom.on("zoom", this.handleZoom);
    }

    getPageDescriptor(pageRange) {
        if (!this.sseq) {
            return;  
        }

        let basePage = 2;
        if(this.sseq.page_list.includes(1)){
            basePage = 1;
        }
        if (pageRange[0] === INFINITY) {
            return "Page ∞";
        }
        if (pageRange === 0) {
            return `Page ${basePage} with all differentials`;
        }
        if (pageRange === 1 && basePage === 2) {
            return `Page ${basePage} with no differentials`;
        }
        if (pageRange.length) {
            if(pageRange[1] === INFINITY){
                return `Page ${pageRange[0]} with all differentials`;
            }
            if(pageRange[1] === -1){
                return `Page ${pageRange[0]} with no differentials`;
            }

            if(pageRange[0] === pageRange[1]){
                return `Page ${pageRange[0]}`;
            }

            return `Pages ${pageRange[0]} – ${pageRange[1]}`.replace(INFINITY, "∞");
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


customElements.define('sseq-display', Display);