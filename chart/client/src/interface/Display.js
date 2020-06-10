"use strict";

import * as EventEmitter from "events";
import * as d3 from "d3";
import { INFINITY } from "../infinity.js";
import { sleep, animationFrame } from "./utils.js";

export class Display extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({mode: 'open'});
        this.shadowRoot.innerHTML = `<slot></slot>`;

        this.xRange = [];
        this.yRange = [];
        this.initialXRange = [];
        this.initialYRange = [];

        this._leftMargin = 40;
        this._rightMargin = 5;
        this._topMargin = 45;
        this._bottomMargin = 50;
        this._domainOffset = 1 / 2;
        
        this.bidegreeDistanceThreshold = 15;
        this.background_color = "#FFFFFF";
        
        this.xScaleInit = d3.scaleLinear();
        this.yScaleInit = d3.scaleLinear();
        
        this.canvas = document.createElement("canvas");
        this.canvas.style.padding = "0px";
        this.canvas.style.position = "absolute";
        this.canvas.style.top = "0";
        this.canvas.style.left = "0";
        this.shadowRoot.appendChild(this.canvas);
        this.context = this.canvas.getContext("2d");
        
        this.updateQueue = 0;


        this.node_buffers = {};
        this.hiddenStructlines = new Set();

        this.handleZoom = this.handleZoom.bind(this);
        this._emitMouseover = this._emitMouseover.bind(this);
        this._emitClick = this._emitClick.bind(this);
        this.updateBatch = this.updateBatch.bind(this);

        this.canvas.addEventListener("click", this._emitClick);

        this._resizeObserver = new ResizeObserver(entries => {
            for(let e of entries){
                requestAnimationFrame(() => e.target.resize());
            }
        });
        this._resizeObserver.observe(this);
    }
    
    start(){
        this.zoom = d3.zoom().scaleExtent([0, 4]);
        this.zoomD3Element = d3.select(this.canvas);
        this.zoomD3Element.call(this.zoom).on("dblclick.zoom", null);
        this._initializeScale(...this.initialXRange, ...this.initialYRange);
        
        this.zoom.on("zoom", this.handleZoom);
        this.zoom.on("start", () => {
            this.updatingZoom = true;
        });
        this.zoom.on("end", () => {
            this.updatingZoom = false;
            this._emitMouseover();
        });
        
        window.addEventListener("mousemove", this._emitMouseover);
        
        this._initializeCanvas();
        this._updateScale();
        this.emit("scale-update", {type : "zoom"});
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
        if(!this.zoom){
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
        this.zoom.translateBy(this.zoomD3Element, this.dxScale(dx)/this.transform.k, this.dyScale(dy)/this.transform.k);
        this.zoom.on("zoom", this.handleZoom);
        this.emit("scale-update", {type : "zoom"});
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
        const canvasWidth = width || 1*computedWidth;
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
        this.emit("canvas-initialize");
    }

    emit(event, ...args){
        let myEvent = new CustomEvent(event, { detail: args });
        this.dispatchEvent(myEvent);
    }

    setXRange(xmin, xmax){
        this.xRange = [0, 0];
        this.xRange[0] = xmin;
        this.xRange[1] = xmax;
    }

    setYRange(ymin, ymax){
        this.yRange = [0, 0];
        this.yRange[0] = ymin;
        this.yRange[1] = ymax;
    }

    setInitialXRange(xmin, xmax){
        this.initialXRange[0] = xmin;
        this.initialXRange[1] = xmax;
    }

    setInitialYRange(ymin, ymax){
        this.initialYRange[0] = ymin;
        this.initialYRange[1] = ymax;
    }

    _initializeScale(xmin, xmax, ymin, ymax){
        this.xminFloat = xmin - this._domainOffset;
        this.xmaxFloat = xmax + this._domainOffset;
        this.yminFloat = ymin - this._domainOffset;
        this.ymaxFloat = ymax - this._domainOffset;
        this.xScaleInit.domain([this.xminFloat, this.xmaxFloat]);
        this.yScaleInit.domain([this.yminFloat, this.ymaxFloat]);
    }

    handleZoom(){
        if(d3.event && d3.event.sourceEvent){
            this.updateMousePosition(d3.event.sourceEvent);
        }
        this.updateBatch();
    }

    updateBatch(){
        this.update(true);
    }

    update(batch = false) {
        if(!this.zoom){
            return;
        }

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

    // This allows us to put various decorations as DOM elements and have them properly clipped
    // as long as they are behind the canvas.
    paintComplementOfClippedRegionWhite(ctx){
        // TODO: get rid of y_clip_offset or make it less ad-hoc
        let y_clip_offset = this.y_clip_offset || 0;
        ctx.save();
        ctx.beginPath();
        ctx.fillStyle = "white";
        ctx.rect(0, 0, this._canvasWidth, this._canvasHeight);
        ctx.moveTo(this._leftMargin, this._topMargin + y_clip_offset);
        ctx.rect(this._leftMargin, this._topMargin + y_clip_offset, this._plotWidth, this._plotHeight - y_clip_offset);
        ctx.fill("evenodd");
        ctx.restore();
    }

    clipContext(ctx) {
        // TODO: get rid of y_clip_offset or make it less ad-hoc
        let y_clip_offset = this.y_clip_offset || 0;
        ctx.beginPath();
        ctx.globalAlpha = 0; // C2S does not correctly clip unless the clip is stroked.
        ctx.rect(this._leftMargin, this._topMargin + y_clip_offset, this._plotWidth, this._plotHeight - y_clip_offset);
        ctx.stroke();
        ctx.clip();
        ctx.globalAlpha = 1;
    }

    _drawSseq(ctx = this.context, drawAxes = false) {
        if(!this.zoom){
            return;
        }        
        this.total_draws = this.total_draws + 1 || 0;
        let startTime = performance.now();

        this._updateScale();
        
        ctx.clearRect(0, 0, this._canvasWidth, this._canvasHeight);
        this.paintComplementOfClippedRegionWhite(ctx);

        for(let elt of this.children){
            if(!elt.paint){
                continue;
            }
            ctx.save();
            if(elt.clipped || elt.hasAttribute("clipped")){
                this.clipContext(ctx);
            }
            elt.paint(this, ctx);
            ctx.restore();
        }

        // if (this.sseq.edgeLayerSVG)
        //     this.drawSVG(ctx, this.sseq.edgeLayerSVG);

        // if(this.svg) {
        //     if(this.svg_unclipped){
        //         ctx.restore();
        //         ctx.save();
        //     }
        //     let x_scale = this.svg_x_scale || this.svg_scale || 1;
        //     let y_scale = this.svg_y_scale || this.svg_scale || 1;
        //     let x_offset = this.svg_x_offset || 0;
        //     let y_offset = this.svg_y_offset || 0;
        //     let default_width = 
        //         this._canvasWidth / (this.xmaxFloat - this.xminFloat) * (this.xRange[1] - this.xRange[0] + 1);
        //     let default_height = 
        //         this._canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.yRange[1] - this.yRange[0] + 1);
        //     let width = default_width * x_scale;
        //     let height = default_height * y_scale;
        //     this.context.drawImage(this.svg,
        //         this.xScale(this.xRange[0] + x_offset), //- display.xMinOffset,
        //         this.yScale(this.yRange[1] + 1 + y_offset) ,
        //         width, height
        //     );
        // }
        ctx.restore();
        this.emit("draw");
        let dt = performance.now() - startTime;
        // console.log("elapsed", dt/1000, "fps:", 1000/dt);
    }

    /**
     * @private
     * TODO: CLEAN ME UP!!
     */
    _updateScale(){
        let zoomD3Element = this.zoomD3Element;
        let transform = d3.zoomTransform(zoomD3Element.node());
        if(isNaN(transform.x) || isNaN(transform.y) || isNaN(transform.k) ){
            this.zoom.transform(zoomD3Element, this.old_transform);
            throw Error("Bad scale?");
        }
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
        if (this.xRange) {
            if (xScale(this.xRange[1] - this.xRange[0] + 2 * this._domainOffset) - xScale(0) < this._plotWidth) {
                // We simply record the scale was maxed and handle this later
                // by modifying xScale directly.
                xScaleMaxed = true;
            } else if (xScale(this.xRange[0] - this._domainOffset) > this._leftMargin) {
                this.zoom.translateBy(zoomD3Element, (this._leftMargin - xScale(this.xRange[0] - this._domainOffset)) / scale, 0);
                autoTranslated = true;
            } else if (xScale(this.xRange[1] + this._domainOffset) < this._clipWidth) {
                this.zoom.translateBy(zoomD3Element, (this._clipWidth - xScale(this.xRange[1] + this._domainOffset)) / scale, 0);
                autoTranslated = true;
            }
        }

        if (this.yRange) {
            if (yScale(0) -yScale(this.yRange[1] - this.yRange[0] + 2 * this._domainOffset) < this._plotHeight) {
                yScaleMaxed = true;
            } else if (yScale(this.yRange[0] - this._domainOffset) < this._clipHeight) {
                this.zoom.translateBy(zoomD3Element, 0, (this._clipHeight - yScale(this.yRange[0] - this._domainOffset)) / scale);
                autoTranslated = true;
            } else if (yScale(this.yRange[1] + this._domainOffset) > this._topMargin) {
                this.zoom.translateBy(zoomD3Element, 0, this._topMargin - yScale(this.yRange[1] + this._domainOffset) / scale);
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
                this.xRange[0] - this._domainOffset,
                this.xRange[1] + this._domainOffset
            ]);
            this.transform.x = old_transform.x;
        }
        if (yScaleMaxed) {
            this.yScale.domain([
                this.yRange[0] - this._domainOffset,
                this.yRange[1] + this._domainOffset
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
        old_transform = this.old_transform;
        this.old_transform = transform;
        let scaleChanged = 
            old_transform === undefined 
            || Math.abs(old_transform.x - transform.x) > 1e-8 
            || Math.abs(old_transform.y - transform.y) > 1e-8 
            || old_transform.k != transform.k;
        let zoomChanged = 
            old_transform === undefined || old_transform.k != transform.k;

        this.old_transform = old_transform;
        if(scaleChanged){
            let type = zoomChanged ? "zoom" : "pan";
            this.emit("scale-update", {type : type});
        }
    }

    dxScale(x){
        return this.xScale(x) - this.xScale(0);
    }

    dyScale(x){
        return this.yScale(x) - this.yScale(0);
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
        if(e.srcElement === this){
            this.mousex = e.layerX;
            this.mousey = e.layerY;
        } else {
            // If mouse is not over display, make sure mouse is not over a class.
            this.mousex = -100000; 
            this.mousey = -100000;
        }
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
        // if(!this.classes_to_draw){
        //     return;
        // }

        // We cannot query for mouse position. We must remember it from
        // previous events. If update() is called, we call _onMousemove without
        // an event.
        if(e) {
            this.updateMousePosition(e);
            this.emit("mouse-state-update", this.mouseState);
        }

        // Don't emit mouseover or mouseout 
        if(this.updatingZoom && this.disableMouseoverUpdates) {
            return;
        }


        redraw = redraw | false;
        // redraw |= this._emitMouseoverClass();
        redraw |= this._emitMouseoverBidegree();

        if (redraw) {
            this._drawSseq(this.context);  
        } 
    }

    _emitMouseoverBidegree(){
        let x = this.mousex;
        let y = this.mousey;
        let nearest_x = Math.round(this.xScale.invert(x));
        let nearest_y = Math.round(this.yScale.invert(y));
        let redraw = false;
        let threshold = this.bidegreeDistanceThreshold * 1;//(this.sseq.bidegreeDistanceScale | 1)
        let xscale = 1;
        let yscale = 1;
        let x_max_threshold = Math.abs(this.dxScale(1)) * 0.4;
        let y_max_threshold = Math.abs(this.dyScale(1)) * 0.4;
        if(threshold > x_max_threshold) {
            xscale = threshold / x_max_threshold;
        }
        if(threshold > y_max_threshold) {
            yscale = threshold / y_max_threshold;
        }
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
            this.xScale(this.xRange[0]),// - this.xMinOffset,
            this.yScale(this.yRange[1] + 1),
            this._canvasWidth  / (this.xmaxFloat - this.xminFloat) * (this.xRange[1] - this.xRange[0] + 1),
            this._canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.yRange[1] - this.yRange[0] + 1)
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
    async seek(x, y){
        this._seekTarget = [x, y];
        if(!this._seekActive){
            this._seekActive = this._seek();
        }
        await this._seekActive;
        delete this._seekActive;
    }

    async _seek(){
        let t = performance.now();
        let tprev = performance.now();
        let lastdx = 0;
        let lastdy = 0;
        while(true){
            // console.log("loop elapsed =",t - tprev);
            tprev = t;
            t = performance.now();
            let dx = 0;
            let dy = 0;
            let [x, y] = this._seekTarget;
            if (x > this.xmaxFloat - 1) {
                dx = this.xmaxFloat - 1 - x;
            } else if (x < this.xminFloat + 1) {
                dx = this.xminFloat + 1 - x;
            }
            if (y > this.ymaxFloat - 1) {
                dy = this.ymaxFloat - 1 - y;
            } else if (y < this.yminFloat + 1) {
                dy = this.yminFloat + 1 - y;
            }
            if (Math.abs(dx - lastdx) < 1e-8 && Math.abs(dy - lastdy) < 1e-8) {
                return;
            }
            lastdx = dx;
            lastdy = dy;
            
            let dxActual = this.dxScale(dx);
            let dyActual = this.dyScale(dy);
            let dist = Math.sqrt(dxActual * dxActual + dyActual * dyActual);
            if(dist < 1e-2){
                return;
            }
            // console.log("seek:", dxActual, dyActual, dist);
            // steps controls the speed -- doubling steps halves the speed.
            // Of course we could maybe set up some fancy algorithm that zooms and pans.
            let steps = Math.ceil(dist / 15);
            let xstep = dxActual / steps;
            let ystep = dyActual / steps;
            this.translateBy(xstep, ystep);
            await animationFrame();
        }
    }

    translateBy(xstep, ystep){
        this.zoom.on("zoom", null);
        this.zoom.translateBy(this.zoomD3Element, xstep / this.scale, ystep / this.scale );
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }

    zoomBy(step, target){
        let factor = Math.exp(Math.log(1.1) * step);
        this.zoom.on("zoom", null);
        this.zoom.scaleBy(this.zoomD3Element, factor, target );
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }
}


customElements.define('sseq-display', Display);