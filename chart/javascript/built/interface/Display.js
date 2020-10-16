"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import * as d3 from "d3";
import { animationFrame } from "./utils";
export class DisplayElement extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({ mode: 'open' });
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
            for (let e of entries) {
                requestAnimationFrame(() => e.target.resize());
            }
        });
        this._resizeObserver.observe(this);
    }
    start() {
        this.zoom = d3.xyzoom();
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
        this.emit("scale-update", { type: "zoom" });
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
    resize(width, height) {
        if (!this.zoom) {
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
        this.zoom.translateBy(this.zoomD3Element, this.dxScale(dx) / this.transform.kx, this.dyScale(dy) / this.transform.ky);
        this.zoom.on("zoom", this.handleZoom);
        this.emit("scale-update", { type: "zoom" });
        this.update(); // Make sure this is update(), updateBatch() causes screen flicker.
    }
    /**
     * Initialization method called in constructor.
     * @private
     */
    _initializeCanvas(width, height) {
        let computedStyle = getComputedStyle(this);
        // computed_width will look like "####px", need to get rid of "px".
        let computedWidth = Number.parseFloat(computedStyle.width.slice(0, -2));
        let computedHeight = Number.parseFloat(computedStyle.height.slice(0, -2));
        const canvasWidth = width || 1 * computedWidth;
        const canvasHeight = height || 0.97 * computedHeight;
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
        let [xRangeMin, xRangeMax] = this.xScaleInit.range();
        let [yRangeMin, yRangeMax] = this.yScaleInit.range();
        this.zoom.extent([[xRangeMin, yRangeMax], [xRangeMax, yRangeMin]])
            .scaleExtent([[0, 4], [0, 4]]);
        this.updateZoomTranslateExtent();
        this.emit("canvas-initialize");
    }
    updateZoomTranslateExtent() {
        let [xMin, xMax] = this.xRange;
        let [yMin, yMax] = this.yRange;
        let [xRangeMin, xRangeMax] = [xMin - this._domainOffset, xMax + this._domainOffset].map(this.xScaleInit);
        let [yRangeMin, yRangeMax] = [yMin - this._domainOffset, yMax + this._domainOffset].map(this.yScaleInit);
        this.zoom.translateExtent([
            [xRangeMin, yRangeMax],
            [xRangeMax, yRangeMin]
        ]);
    }
    emit(event, ...args) {
        let myEvent = new CustomEvent(event, { detail: args });
        this.dispatchEvent(myEvent);
    }
    setXRange(xmin, xmax) {
        this.xRange = [0, 0];
        this.xRange[0] = xmin;
        this.xRange[1] = xmax;
        if (this.zoom) {
            this.updateZoomTranslateExtent();
        }
    }
    setYRange(ymin, ymax) {
        this.yRange = [0, 0];
        this.yRange[0] = ymin;
        this.yRange[1] = ymax;
        if (this.zoom) {
            this.updateZoomTranslateExtent();
        }
    }
    setInitialXRange(xmin, xmax) {
        this.initialXRange[0] = xmin;
        this.initialXRange[1] = xmax;
    }
    setInitialYRange(ymin, ymax) {
        this.initialYRange[0] = ymin;
        this.initialYRange[1] = ymax;
    }
    _initializeScale(xmin, xmax, ymin, ymax) {
        this.xminFloat = xmin - this._domainOffset;
        this.xmaxFloat = xmax + this._domainOffset;
        this.yminFloat = ymin - this._domainOffset;
        this.ymaxFloat = ymax - this._domainOffset;
        this.xScaleInit.domain([this.xminFloat, this.xmaxFloat]);
        this.yScaleInit.domain([this.yminFloat, this.ymaxFloat]);
    }
    handleZoom() {
        if (d3.event && d3.event.sourceEvent) {
            this.updateMousePosition(d3.event.sourceEvent);
        }
        this.updateBatch();
    }
    updateBatch() {
        this.update(true);
    }
    update(batch = false) {
        if (!this.zoom) {
            return;
        }
        this.updateQueue++;
        // this._drawSseq(this.context);
        let drawFunc = () => {
            this.updateQueue--;
            if (this.updateQueue != 0)
                return;
            this._drawSseq(this.context);
            this._emitMouseover();
        };
        if (batch) {
            requestAnimationFrame(drawFunc);
        }
        else {
            drawFunc();
        }
    }
    // This allows us to put various decorations as DOM elements and have them properly clipped
    // as long as they are behind the canvas.
    paintComplementOfClippedRegionWhite(ctx) {
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
        if (!this.zoom) {
            return;
        }
        this.total_draws = this.total_draws + 1 || 0;
        let startTime = performance.now();
        this._updateScale();
        ctx.clearRect(0, 0, this._canvasWidth, this._canvasHeight);
        this.paintComplementOfClippedRegionWhite(ctx);
        for (let elt of this.children) {
            if (!elt.paint) {
                continue;
            }
            ctx.save();
            if (elt.clipped || elt.hasAttribute("clipped")) {
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
    _updateScale() {
        let zoomD3Element = this.zoomD3Element;
        let transform = d3.xyzoomTransform(zoomD3Element.node());
        if (isNaN(transform.x) || isNaN(transform.y) || isNaN(transform.kx) || isNaN(transform.ky)) {
            this.zoom.transform(zoomD3Element, this.old_transform);
            throw Error("Bad scale?");
        }
        this.zoom.on("zoom", null);
        transform = d3.xyzoomTransform(zoomD3Element.node());
        let old_transform = this.transform;
        this.transform = transform;
        this.scale = this.transform.kx;
        this.xScale = this.transform.rescaleX(this.xScaleInit);
        this.yScale = this.transform.rescaleY(this.yScaleInit);
        this.zoom.transform(zoomD3Element, this.transform);
        this.xminFloat = this.xScale.invert(this._leftMargin);
        this.xmaxFloat = this.xScale.invert(this._clipWidth);
        this.yminFloat = this.yScale.invert(this._clipHeight);
        this.ymaxFloat = this.yScale.invert(this._topMargin);
        this.xmin = Math.ceil(this.xminFloat);
        this.xmax = Math.floor(this.xmaxFloat);
        this.ymin = Math.ceil(this.yminFloat);
        this.ymax = Math.floor(this.ymaxFloat);
        let scaleChanged = old_transform === undefined
            || Math.abs(old_transform.x - transform.x) > 1e-8
            || Math.abs(old_transform.y - transform.y) > 1e-8
            || old_transform.k != transform.k;
        let zoomChanged = old_transform === undefined || old_transform.kx !== transform.kx;
        this.old_transform = old_transform;
        if (scaleChanged) {
            let type = zoomChanged ? "zoom" : "pan";
            this.emit("scale-update", { type: type });
        }
        this.zoom.on("zoom", this.handleZoom);
    }
    dxScale(x) {
        return this.xScale(x) - this.xScale(0);
    }
    dyScale(x) {
        return this.yScale(x) - this.yScale(0);
    }
    getMouseState() {
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
        o.distance = Math.sqrt(dx * dx + dy * dy);
        o.mouseover_class = this.mouseover_class;
        o.mouseover_bidegree = this.mouseover_bidegree;
        return o;
    }
    updateMousePosition(e) {
        if (e.srcElement === this) {
            this.mousex = e.layerX;
            this.mousey = e.layerY;
        }
        else {
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
        if (e) {
            this.updateMousePosition(e);
            this.emit("mouse-state-update", this.mouseState);
        }
        // Don't emit mouseover or mouseout 
        if (this.updatingZoom && this.disableMouseoverUpdates) {
            return;
        }
        redraw = redraw | false;
        // redraw |= this._emitMouseoverClass();
        redraw |= this._emitMouseoverBidegree();
        if (redraw) {
            this._drawSseq(this.context);
        }
    }
    _emitMouseoverBidegree() {
        let x = this.mousex;
        let y = this.mousey;
        let nearest_x = Math.round(this.xScale.invert(x));
        let nearest_y = Math.round(this.yScale.invert(y));
        let redraw = false;
        let threshold = this.bidegreeDistanceThreshold * 1; //(this.sseq.bidegreeDistanceScale | 1)
        let xscale = 1;
        let yscale = 1;
        let x_max_threshold = Math.abs(this.dxScale(1)) * 0.4;
        let y_max_threshold = Math.abs(this.dyScale(1)) * 0.4;
        if (threshold > x_max_threshold) {
            xscale = threshold / x_max_threshold;
        }
        if (threshold > y_max_threshold) {
            yscale = threshold / y_max_threshold;
        }
        if (this.mouseover_bidegree) {
            let bidegree = this.mouseover_bidegree;
            let dx = (x - this.xScale(bidegree[0])) * xscale;
            let dy = (y - this.yScale(bidegree[1])) * yscale;
            let distance = Math.sqrt(dx * dx + dy * dy);
            if (distance < threshold) {
                return false;
            }
            else {
                this.emit("mouseout-bidegree", this.mouseover_bidegree, this.mouseState);
                this.mouseover_bidegree = null;
                redraw = true;
            }
        }
        let bidegree = [nearest_x, nearest_y];
        let dx = (x - this.xScale(bidegree[0])) * xscale;
        let dy = (y - this.yScale(bidegree[1])) * yscale;
        let distance = Math.sqrt(dx * dx + dy * dy);
        if (distance < threshold) {
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
    drawSVG(context, xml) {
        // make it base64
        let svg64 = btoa(xml);
        let b64Start = 'data:image/svg+xml;base64,';
        // prepend a "header"
        let image64 = b64Start + svg64;
        // set it as the source of the img element
        let img = new Image();
        img.src = image64;
        context.drawImage(img, this.xScale(this.xRange[0]), // - this.xMinOffset,
        this.yScale(this.yRange[1] + 1), this._canvasWidth / (this.xmaxFloat - this.xminFloat) * (this.xRange[1] - this.xRange[0] + 1), this._canvasHeight / (this.ymaxFloat - this.yminFloat) * (this.yRange[1] - this.yRange[0] + 1));
    }
    toSVG() {
        let ctx = new C2S(this._canvasWidth, this._canvasHeight);
        this._drawSseq(ctx);
        return ctx.getSerializedSvg(true);
    }
    downloadSVG(filename) {
        if (filename === undefined) {
            filename = `${this.sseq.name}_x-${this.xmin}-${this.xmax}_y-${this.ymin}-${this.ymax}.svg`;
        }
        IO.download(filename, this.toSVG(), "image/svg+xml");
    }
    /**
     * Move the canvas to contain (x,y)
     * TODO: control speed, control acceptable range of target positions, maybe zoom out if display is super zoomed in?
     * @param x
     * @param y
     */
    seek(x, y) {
        return __awaiter(this, void 0, void 0, function* () {
            this._seekTarget = [x, y];
            if (!this._seekActive) {
                this._seekActive = this._seek();
            }
            yield this._seekActive;
            delete this._seekActive;
        });
    }
    _seek() {
        return __awaiter(this, void 0, void 0, function* () {
            let t = performance.now();
            let tprev = performance.now();
            let lastdx = 0;
            let lastdy = 0;
            while (true) {
                // console.log("loop elapsed =",t - tprev);
                tprev = t;
                t = performance.now();
                let dx = 0;
                let dy = 0;
                let [x, y] = this._seekTarget;
                if (x > this.xmaxFloat - 1) {
                    dx = this.xmaxFloat - 1 - x;
                }
                else if (x < this.xminFloat + 1) {
                    dx = this.xminFloat + 1 - x;
                }
                if (y > this.ymaxFloat - 1) {
                    dy = this.ymaxFloat - 1 - y;
                }
                else if (y < this.yminFloat + 1) {
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
                if (dist < 1e-2) {
                    return;
                }
                // console.log("seek:", dxActual, dyActual, dist);
                // steps controls the speed -- doubling steps halves the speed.
                // Of course we could maybe set up some fancy algorithm that zooms and pans.
                let steps = Math.ceil(dist / 15);
                let xstep = dxActual / steps;
                let ystep = dyActual / steps;
                this.translateBy(xstep, ystep);
                yield animationFrame();
            }
        });
    }
    translateBy(xstep, ystep) {
        this.zoom.on("zoom", null);
        this.zoom.translateBy(this.zoomD3Element, xstep / this.scale, ystep / this.scale);
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }
    zoomScaleTo([kx, ky], p) {
        let zoom = this.zoom;
        let transform = this.transform;
        function centroid(extent) {
            return [(+extent[0][0] + +extent[1][0]) / 2, (+extent[0][1] + +extent[1][1]) / 2];
        }
        function constrain(transform, extent) {
            let [[x0, y0], [x1, y1]] = zoom.translateExtent();
            var dx0 = transform.invertX(extent[0][0]) - x0, dx1 = transform.invertX(extent[1][0]) - x1, dy0 = transform.invertY(extent[0][1]) - y0, dy1 = transform.invertY(extent[1][1]) - y1;
            return transform.translate(dx1 > dx0 ? (dx0 + dx1) / 2 : Math.min(0, dx0) || Math.max(0, dx1), dy1 > dy0 ? (dy0 + dy1) / 2 : Math.min(0, dy0) || Math.max(0, dy1));
        }
        function constrainScaleExtent() {
            let [[x0, y0], [x1, y1]] = zoom.translateExtent();
            let [[kx0, kx1], [ky0, ky1]] = zoom.scaleExtent();
            let [[rx0, ry0], [rx1, ry1]] = zoom.extent()();
            kx0 = x1 !== x0 ? Math.max(kx0, (rx1 - rx0) / (x1 - x0)) : Infinity;
            ky0 = y1 !== y0 ? Math.max(ky0, (ry1 - ry0) / (y1 - y0)) : Infinity;
            zoom.scaleExtent([[kx0, kx1], [ky0, ky1]]);
        }
        function translate(transform, p0, p1) {
            var x = p0[0] - p1[0] * transform.kx, y = p0[1] - p1[1] * transform.ky;
            return x === transform.x && y === transform.y ? transform : new transform.constructor(x, y, transform.kx, transform.ky);
        }
        function scale(transform, kx, ky) {
            let [[kx0, kx1], [ky0, ky1]] = zoom.scaleExtent();
            kx = Math.max(kx0, Math.min(kx1, kx));
            ky = Math.max(ky0, Math.min(ky1, ky));
            return (kx === transform.kx && ky === transform.ky) ? transform : new transform.constructor(transform.x, transform.y, kx, ky);
        }
        this.zoom.transform(this.zoomD3Element, function () {
            let e = zoom.extent().apply(this, arguments), t0 = transform, p0 = p == null ? centroid(e) : typeof p === "function" ? p.apply(this, arguments) : p, p1 = t0.invert(p0), kx1 = typeof kx === "function" ? kx.apply(this, arguments) : kx, ky1 = typeof ky === "function" ? ky.apply(this, arguments) : ky;
            let result = constrain(translate(scale(t0, kx1, ky1), p0, p1), e);
            return result;
        });
        constrainScaleExtent();
    }
    zoomScaleBy([kx0, ky0], p) {
        let { kx, ky } = this.transform;
        this.zoomScaleTo([
            function () {
                kx0 = typeof kx0 === "function" ? kx0.apply(this, arguments) : kx0;
                return kx * kx0;
            },
            function () {
                ky0 = typeof ky0 === "function" ? ky0.apply(this, arguments) : ky0;
                return ky * ky0;
            }
        ], p);
    }
    scaleBy(step, zoom_center) {
        // zoom_center = zoom_center || [0,0];
        // let [zcX, zcY] = zoom_center;
        let factor = Math.exp(Math.log(1.1) * step);
        this.zoom.on("zoom", null);
        this.zoomScaleBy([factor, factor], zoom_center);
        // this.zoom.transform(this.zoomD3Element, () => new this.transform.constructor(this.transform.x - zoom_center);
        // this.zoom.scaleBy(this.zoomD3Element, factor, factor);
        // this.zoom.translateBy(this.zoomD3Element, ...zoom_center);
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }
    scaleXBy(step, zoom_center) {
        let factor = Math.exp(Math.log(1.1) * step);
        this.zoom.on("zoom", null);
        this.zoomScaleBy([factor, 1], zoom_center);
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }
    scaleYBy(step, zoom_center) {
        let factor = Math.exp(Math.log(1.1) * step);
        this.zoom.on("zoom", null);
        this.zoomScaleBy([1, factor], zoom_center);
        this.update();
        this.zoom.on("zoom", this.handleZoom);
    }
}
customElements.define('sseq-display', DisplayElement);
