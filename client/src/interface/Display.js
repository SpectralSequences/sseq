"use strict";

let EventEmitter = require("events");
let d3 = Object.assign({}, 
    require("d3-selection"), 
    require("d3-zoom"), 
    require("d3-scale"), 
    require("d3-timer")
);

let INFINITY = require("../infinity.js").INFINITY

const gridGo = "go";
const gridChess = "chess";

class Display extends EventEmitter {
    // container is either an id (e.g. "#main") or a DOM object
    constructor(container, sseq) {
        super();

        this.leftMargin = 40;
        this.rightMargin = 5;
        this.topMargin = 45;
        this.bottomMargin = 50;
        this.domainOffset = 1 / 2;

        this.gridStyle = gridGo;
        this.gridColor = "#c6c6c6";
        this.background_color = "#FFFFFF";
        this.gridStrokeWidth = 0.3;
        this.TICK_STEP_LOG_BASE = 1.1; // Used for deciding when to change tick step.

        this.hiddenStructlines = new Set();
        this.updateQueue = 0;

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

        this.updateBatch = this.updateBatch.bind(this);
        this.nextPage = this.nextPage.bind(this);
        this.previousPage = this.previousPage.bind(this);
        this._onMousemove = this._onMousemove.bind(this);
        this._onClick = this._onClick.bind(this);

        this.zoom = d3.zoom().scaleExtent([0, 4]);
        this.zoom.on("zoom", this.updateBatch);
        this.zoomD3Element = d3.select(this.canvas);
        this.zoomD3Element.call(this.zoom).on("dblclick.zoom", null);

        this.canvas.addEventListener("mousemove", this._onMousemove);
        this.canvas.addEventListener("click", this._onClick);

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
            console.log(`Warning: min_page_idx ${this.sseq.min_page_idx} greater than page list length ${this.sseq.page_list.length}. Using 0 for min_page_idx instead.`);
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
            if (d3.event) {
                // d3 zoom doesn't allow the events it handles to bubble, so we
                // fails to track pointer position.
                this._onMousemove(d3.event);
            } else {
                this._onMousemove();
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

        let [nodes, edges] = this.sseq.getElementsToDraw(
            this.pageRange, 
            this.xmin - 1, this.xmax + 1, this.ymin - 1, this.ymax + 1
        );

        this._drawGrid(ctx);
        this.emit("draw_background");
        this._updateNodes(nodes);
        this._drawEdges(ctx, edges);
        this._drawNodes(ctx);

        if (this.sseq.edgeLayerSVG)
            this.drawSVG(ctx, this.sseq.edgeLayerSVG);

        ctx.restore();

        this.emit("draw");
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
        this.transform = d3.zoomTransform(zoomD3Element.node());
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
                this._drawGoGrid(context);
                break;
            case gridChess:
                this._drawChessGrid(context);
                break;
            default:
                // TODO: an error here?
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

    _drawNodes(context) {
        for (let c of this.classes_to_draw) {
            c.draw(context);
        }
    }

    _drawEdges(context, edges){        
        for (let e of edges) {
            if(!e || e.invalid || !e.visible){ // TODO: should probably log some of the cases where we skip an edge...
                continue;
            }
            if (e.type == "Structline" && this.hiddenStructlines.has(e.mult)) {
                continue;
            }

            let source_node = e._source;
            let target_node = e._target;
            if(!source_node || ! target_node){
                continue;
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
                context.setLineDash(e.dash)
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

    _onClick(e) {
        this.emit("click", this.mouseover_node, e);
    }

    _onMousemove(e) {
        // If not yet set up 
        if (!this.classes_to_draw) return;

        let redraw = false;

        // We cannot query for mouse position. We must remember it from
        // previous events. If update() is called, we call _onMousemove without
        // an event.
        let rect = this.canvas.getBoundingClientRect();
        if (e) {
            this.x = e.clientX - rect.x;
            this.y = e.clientY - rect.y;
        }

        if (this.mouseover_node) {
            if (this.classes_to_draw.includes(this.mouseover_class) && this.context.isPointInPath(this.mouseover_class._path, this.x, this.y)) {
                return;
            } else {
                this.mouseover_node.highlight = false;
                this.mouseover_node = null;
                this.mouseover_class = null;
                redraw = true;
                this.emit("mouseout");
            }
        }
        let node = this.classes_to_draw.find(n => this.context.isPointInPath(n._path, this.x, this.y));
        if (node) {
            redraw = true;
            node.highlight = true;
            this.mouseover_node = node;
            this.mouseover_class = node.c;
            this.emit("mouseover", node);
        }

        if (redraw) this._drawSseq(this.context);
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
            this.xScale(this.sseq.x_range[0]) - this.xMinOffset,
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
        this.zoom.on("zoom", this.updateBatch);
    }

    getPageDescriptor(pageRange) {
        if (!this.sseq) return;

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

exports.Display = Display;
