import { Canvas, EdgeOptions, Glyph, GlyphBuilder, GlyphInstance, JsPoint as Vec2, Vec4 as RustColor, Vec4 } from "./display_backend/pkg/sseq_display_backend";
import { Shape, Node, ChartClass, SpectralSequenceChart, INFINITY } from "./chart/lib";
import { assert } from "console";
import { Color } from "./chart/Color";
import { throttle } from "./utils";

(<any>window).EdgeOptions = EdgeOptions;
(<any>window).Vec2 = Vec2;

interface Touch {
    centerX : number; 
    centerY : number; 
    averageDistance : number; 
    touchCount : number;
    time : number;
}

interface GlyphAndColors {
    glyph : Glyph;
    fill : RustColor;
    stroke : RustColor;
}

function getTouchesInfo(touchEvent : TouchEvent) : Touch {
    let touches = Array.from(touchEvent.touches);
    let touchCount = touches.length;
    let centerX = 0;
    let centerY = 0;
    let averageDistance = 0;
    for(let touch of touches){
        let {screenX, screenY} = touch;
        centerX += screenX;
        centerY += screenY;
    }
    centerX /= touchCount;
    centerY /= touchCount;
    for(let touch of touches){
        let {screenX, screenY} = touch;
        let dx = screenX - centerX;
        let dy = screenY - centerY;
        averageDistance += Math.sqrt(dx * dx + dy * dy);
    }
    averageDistance /= touchCount;
    let time = getTime();
    return { centerX, centerY, averageDistance, touchCount, time };
}

function getTime(){
    return new Date().getTime();
}

function buildShapeGlyph(shape : Shape) : GlyphBuilder {
    switch (shape.ty) {
        case "character":
            if(shape.font === "stix"){
                return GlyphBuilder.from_stix(shape.char);
            } else {
                throw Error("Not implemented");
            }
        case "empty":
            return GlyphBuilder.empty();
        case "composed":
            let builder = buildShapeGlyph(shape.innerShape || { ty : "empty" });
            switch(shape.operation){
                case "circled":
                    builder.circled(shape.padding);
                    break;
                case "boxed":
                    builder.boxed(shape.padding);
                    break;
                default:
                    throw Error("Unknown composition operation.");
            }
            return builder;
        case "diacritic":
            throw Error("Not implemented.")
    }
}


const WEBGL_OPTIONS =  {"stencil" : true, "alpha" : true , "preserveDrawingBuffer" : true, antialias : true };

export class ChartElement extends HTMLElement {
    _oldTouches : Touch[] = [];
    _previousMouseX : number = 0;
    _previousMouseY : number = 0;
    _canvas? : Canvas;
    _needsRedraw : boolean = true;
    _idleFrames : number = 0;
    requestAnimationFrameId : number = 0;
    _resizeObserver? : ResizeObserver;
    _mouseDown : boolean = false;
    _glyph_scale : number = 0;
    shape_to_glyph : Map<Shape, Glyph> = new Map();
    class_to_glyph_instance : Map<ChartClass, GlyphInstance> = new Map();
    glyph_instance_index_to_class : ChartClass[] = [];
    mouseover_class? : ChartClass;
    sseq? : SpectralSequenceChart;
    pageRange : [number, number] = [0, INFINITY];
    page : number = 0;
    page_idx : number = 0;
    defaultGlyphAndColors : GlyphAndColors;
    scale_update_zoom : boolean = false;
    scale_update_pan : boolean = false;

    topMargin : number = 40;
    bottomMargin : number = 60;
    leftMargin : number = 40;
    rightMargin : number = 20;

    constructor(){
        super();
        this.attachShadow({ mode : "open" });
        let gb = GlyphBuilder.empty();
        gb.circled(15);
        let glyph = gb.build();
        this.defaultGlyphAndColors = {
            glyph,
            fill : new RustColor(0, 0, 0, 1),
            stroke : new RustColor(0, 0, 0, 1)
        };
    }

    connectedCallback(){
        let canvasElement = document.createElement("canvas");
        this.shadowRoot!.appendChild(canvasElement);
        let slot = document.createElement("slot");
        slot.style.position = "relative";
        this.shadowRoot!.appendChild(slot);
        this.style.overflow = "hidden";

        let canvasContext = canvasElement.getContext("webgl2", WEBGL_OPTIONS) as WebGL2RenderingContext;
        this._canvas = new Canvas(canvasContext);
        this._canvas.set_margins(this.leftMargin, this.rightMargin, this.bottomMargin, this.topMargin);
        this.addEventListener("mousedown", this.handleMouseDown.bind(this));
        this.addEventListener("mouseup", this.handleMouseUp.bind(this));
        this.addEventListener("mousemove", this.handleMouseMove.bind(this));
        this.addEventListener("touchstart", this.handleTouchStart.bind(this));
        this.addEventListener("touchmove", this.handleTouchMove.bind(this));
        this.addEventListener("touchend", this.handleTouchEnd.bind(this));

        this.addEventListener("wheel", this.handleScroll.bind(this));
        this._resizeObserver = new ResizeObserver(_entries => {
            this._resize();
        });
        this._resizeObserver.observe(this);
        
        this._canvas.resize(this.offsetWidth, this.offsetHeight, window.devicePixelRatio);
        this._canvas.set_current_xrange(-10, 10);
        this._canvas.set_current_yrange(-10, 10);
        this._requestRedraw();
        this._requestFrame();
        this.dispatchCustomEvent("canvas-initialize", {});
        this.dispatchCustomEvent("margin-update", {});
    }

    _resize(){
        this._canvas!.resize(this.offsetWidth, this.offsetHeight, window.devicePixelRatio);
        this._requestRedraw();
        this._requestFrame();
        this.dispatchCustomEvent("canvas-resize", {});
    }

    _requestFrame(){
        this.requestAnimationFrameId = requestAnimationFrame(() => this.handleFrame());
    }

    _requestRedraw(){
        this._needsRedraw = true;
    }
    
    _stopAnimation(){

    }

    current_xrange() : [number, number] {
        return Array.from(this._canvas!.current_xrange()) as [number, number];
    }

    current_yrange() : [number, number] {
        return Array.from(this._canvas!.current_yrange()) as [number, number];
    }

    dxScale(x : number) : number {
        return this._canvas!.scale_x(x);
    }

    dyScale(y : number) : number {
        return -this._canvas!.scale_y(y);
    }

    xScale(x : number) : number {
        return this._canvas!.transform_x(x);
    }

    yScale(y : number) : number {
        return this._canvas!.transform_y(y);
    }

    dispatchCustomEvent(type : string, detail : Object){
        this.dispatchEvent(new CustomEvent(type, { detail }));
    }

    _requestScaleUpdateZoom() {
        this.scale_update_zoom = true;
        this._requestRedraw();
    }

    _requestScaleUpdatePan() {
        this.scale_update_pan = true;
        this._requestRedraw();
    }

    handleScroll(event : WheelEvent) {
        event.preventDefault();
        this._stopAnimation();
        let mousePoint = new Vec2(event.offsetX, event.offsetY);
        // If we are close to a grid point (currently within 10px) lock on to it.
        let [nearestX, nearestY, distance] = this._canvas!.nearest_gridpoint(mousePoint);
        if(distance < 10){
            this._canvas!.translate(new Vec2(-nearestX + mousePoint.x, -nearestY + mousePoint.y));
        }
        let direction = Math.sign(event.deltaY);
        this._canvas!.scale_around(Math.pow(0.6, direction), mousePoint);
        this._requestScaleUpdateZoom();
        this.updateMouseoverClass([event.offsetX, event.offsetY]);
    }
    
    handlePinch(x : number, y : number, delta : number) {
        this._stopAnimation();
        this._canvas!.scale_around(Math.pow(0.98, delta), new Vec2(x, y));
        this._requestScaleUpdateZoom();
    }

    handleTouchStart(event : TouchEvent) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } = getTouchesInfo(event);
        let time = getTime();
        this._stopAnimation();
        this._oldTouches.push({centerX, centerY, averageDistance, touchCount, time});
    }
    
    handleTouchMove(event : TouchEvent) {
        event.preventDefault();
        let touch = getTouchesInfo(event);
        let { centerX, centerY, averageDistance, touchCount } = touch;
        let previous = this._oldTouches[this._oldTouches.length - 1];
        if(previous.touchCount === touchCount) {
            let type;
            if(averageDistance !== 0 && previous.averageDistance !== 0) {
                this._canvas!.scale_around(averageDistance / previous.averageDistance, new Vec2(previous.centerX, previous.centerY));
                this._requestScaleUpdateZoom();
            } else {
                this._canvas!.translate(new Vec2(centerX - previous.centerX, centerY - previous.centerY));
                this._requestScaleUpdatePan();
            }
        }
        this._oldTouches.push(touch);
    }
    
    handleTouchEnd(event : TouchEvent) {
        event.preventDefault();
        let touch = getTouchesInfo(event);
        if(touch.touchCount !== 0) {
            this._oldTouches.push(touch);
            return;
        }
        let time = touch.time;

        let oldTouches = this._oldTouches;
        this._oldTouches = [];

        // Search for an old touch that was long enough ago that the velocity should be stable
        for(let i = oldTouches.length - 2; i >= 0; i--) {
            // Ignore touches due to a pinch gesture
            if(oldTouches[i].touchCount > 1) {
                return;
            }

            // If we find an old enough touch, maybe do a fling
            if(time - oldTouches[i].time > 0.1 * 1000) {
                this._maybeFling(oldTouches[i], oldTouches[i + 1]);
                return;
            }
        }
    }

    _maybeFling(beforeTouch : Touch, afterTouch : Touch){
        // let scale = 1 / (afterTouch.time - beforeTouch.time);
        // let vx = (afterTouch.centerX - beforeTouch.centerX) * scale;
        // let vy = (afterTouch.centerY - beforeTouch.centerY) * scale;
        // let speed = Math.sqrt(vx * vx + vy * vy);
        // let duration = Math.log(1 + speed) / 5;
        // let flingDistance = speed * duration / 5; // Divide by 5 since a quintic decay function has an initial slope of 5

        // // Only fling if the speed is fast enough
        // if(speed > 50) {
        //     _startAnimation(.DECAY, duration);
        //     _endOrigin += velocity * (flingDistance / speed);
        // }
    }
    
    handleMouseDown(event : MouseEvent) {
        let { offsetX : x, offsetY : y } = event;
        // this.setCursor(.MOVE);
        this._mouseDown = true;
        this._previousMouseX = x;
        this._previousMouseY = y;
        if(this.mouseover_class){
            (<any>document.querySelector("sseq-class-highlighter")).fire([this.mouseover_class]);
        }
    }
    
    handleMouseMove(event : MouseEvent) {
        let { offsetX : x, offsetY : y, buttons } = event;
        if(buttons > 0){ 
            this._canvas!.translate(new Vec2(x - this._previousMouseX, y - this._previousMouseY));
            this._requestRedraw();
            this._requestScaleUpdatePan();

            // this.setCursor(.MOVE);
        }
        this._previousMouseX = x;
        this._previousMouseY = y;
        if(!this._mouseDown){
            this.updateMouseoverClass([x,y]);
        }
    }

    updateMouseoverClass(coord : [number, number]){
        let idx = this._canvas!.object_underneath_pixel(new Vec2(...coord));
        let new_mouseover_class : ChartClass | undefined;
        if(idx === undefined || this.glyph_instance_index_to_class[idx] === undefined){
            new_mouseover_class = undefined;
        } else {
            new_mouseover_class = this.glyph_instance_index_to_class[idx];
        }
        if(this.mouseover_class === new_mouseover_class){
            return;
        }
        if(this.mouseover_class){
            this.dispatchCustomEvent("mouseout-class", { cls : this.mouseover_class });
        }
        this.mouseover_class = new_mouseover_class;
        if(new_mouseover_class){
            this.dispatchCustomEvent("mouseover-class", { cls : new_mouseover_class });
        }
    }
    
    handleMouseUp(event : MouseEvent) {
        let { offsetX : x, offsetY : y, buttons } = event;
        if(buttons === 0) {
            this._mouseDown = false;
            // this._mouseAction = .NONE
            // this.setCursor(.DEFAULT);
        }
    
        this._previousMouseX = x;
        this._previousMouseY = y;
    }


    handleFrame() {
        this._requestFrame();
        
		// let time = getTime();

		// if _animation != .NONE {
		// 	var t = (time - _startTime) / (_endTime - _startTime)

		// 	# Stop the animation once it's done
		// 	if t > 1 {
		// 		_canvas.setOriginAndScale(_endOrigin.x, _endOrigin.y, _endScale)
		// 		_animation = .NONE
		// 	}

		// 	else {
		// 		# Bend the animation curve for a more pleasant animation
		// 		if _animation == .EASE_IN_OUT {
		// 			t *= t * t * (t * (t * 6 - 15) + 10)
		// 		} else {
		// 			assert(_animation == .DECAY)
		// 			t = 1 - t
		// 			t = 1 - t * t * t * t * t
		// 		}

		// 		# Animate both origin and scale
		// 		_canvas.setOriginAndScale(
		// 			_startOrigin.x + (_endOrigin.x - _startOrigin.x) * t,
		// 			_startOrigin.y + (_endOrigin.y - _startOrigin.y) * t,
		// 			1 / (1 / _startScale + (1 / _endScale - 1 / _startScale) * t))
		// 	}

		// 	_requestRedraw
		// }

		if(this._needsRedraw) {
            this._idleFrames = 0;
			this._needsRedraw = false;
            this._draw();
            return;
        }
		// Render occasionally even when idle. Chrome must render at least 10fps to
		// avoid stutter when starting to render at 60fps again.
        // this._idleFrames ++;
        // if(this._idleFrames % 6 == 0 && this._idleFrames < 60 * 2) {
		// 	this._draw();
		// }
    }

    nextPage(){
        this.changePage(1);
    }

    previousPage(){
        this.changePage(-1);
    }

    changePage(delta : number){
        if(!this.sseq){
            return;
        }
        let min_idx = 0;
        let max_idx = this.sseq.page_list.length - 1;
        let new_idx = Math.min(Math.max(this.page_idx + delta, min_idx), max_idx)
        if (new_idx !== this.page_idx) {
            this.setPage(new_idx);
        }
    }

    setPage(idx? : number){
        if (!this.sseq){
            return;  
        } 

        if(idx !== undefined){
            this.page_idx = idx;
        }
        this.pageRange = this.sseq.page_list[this.page_idx];
        this.page = this.pageRange[0];

        this._updateChart();
        this.dispatchCustomEvent("page-change", {page : this.page, range : this.pageRange, idx : this.page_idx });
    }

    setMargins(leftMargin : number, rightMargin : number, topMargin : number, bottomMargin : number){
        this.leftMargin = leftMargin;
        this.rightMargin = rightMargin;
        this.topMargin = topMargin;
        this.bottomMargin = bottomMargin;
        this._canvas!.set_margins(this.leftMargin, this.rightMargin, this.bottomMargin, this.topMargin);
        this.dispatchCustomEvent("margin-update", {});
    }

    setSseq(sseq : SpectralSequenceChart){
        this.sseq = sseq;
        this.setPage(0);
    }

    getShapeGlyph(shape : Shape) : Glyph {
        let cached = this.shape_to_glyph.get(shape);
        if(cached){
            return cached;
        }
        let glyph = buildShapeGlyph(shape).build();
        this.shape_to_glyph.set(shape, glyph);
        return glyph;
    }
    
    getNodeGlyphAndColors(node : Node) : GlyphAndColors {
        if(node === "DefaultNode"){
            return this.defaultGlyphAndColors;
        } else {
            let glyph = this.getShapeGlyph(node.shape);
            return {
                glyph,
                stroke : new RustColor(...node.colors[0]),
                fill : new RustColor(...node.colors[1])
            };
        }
    }

    getClassPosition(c : ChartClass) : [number, number] {
        let position = new Vec2(c.x!, c.y!);
        let offset = new Vec2(c.getXOffset(this.page), c.getYOffset(this.page));
        // For some reason this line really confuses the type checker...
        let { x, y } = (<any>this._canvas!).glyph_position(position, offset).toJSON();
        return [x, y];
    }

    isClassInRange(c : ChartClass) : boolean {
        let [xmin, xmax ] = this.current_xrange();
        let [ymin, ymax ] = this.current_yrange();
        return xmin <= c.x! && c.x! <= xmax && ymin <= c.y! && c.y! <= ymax;
    }

    isClassNearlyInRange(c : ChartClass) : boolean {
        let [xmin, xmax ] = this.current_xrange();
        let [ymin, ymax ] = this.current_yrange();
        return xmin - 1.0 <= c.x! && c.x! <= xmax + 1.0 && ymin - 1.0 <= c.y! && c.y! <= ymax + 1.0;
    }

    isClassVisible(c : ChartClass) : boolean {
        return this.isClassNearlyInRange(c) && c.visible[this.page];
    }

    classOuterRadius(c : ChartClass) : number | undefined {
        let glyph_instance = this.class_to_glyph_instance.get(c);
        if(!glyph_instance){
            return;
        }
        return glyph_instance.outer_radius() * this._glyph_scale * 100.0;
    }

    _updateChart(){
        if(!this.sseq){
            return;
        }
        this._canvas!.clear();
        let idx = -1;
        for(let c of this.sseq.classes.values()){
            if(!c.visible[this.page] || c.max_page < this.page){
                continue;
            }
            idx ++;
            let position = new Vec2(c.x!, c.y!);
            let offset = new Vec2(c.getXOffset(this.page), c.getYOffset(this.page));
            let {glyph, stroke, fill} = this.getNodeGlyphAndColors(c.node[this.page]);
            let glyph_instance = this._canvas!.add_glyph(position, offset, glyph, c.scale[this.page] * 100, stroke, fill);
            this.class_to_glyph_instance.set(c, glyph_instance);
            this.glyph_instance_index_to_class[idx] = c;
        }
        for(let e of this.sseq.edges.values()){
            if(!e.drawOnPageQ(this.pageRange)){
                continue;
            }
            let start_glyph = this.class_to_glyph_instance.get(e.source!)!;
            let end_glyph = this.class_to_glyph_instance.get(e.target!)!;
            let {start_tip, end_tip, bend, color, dash_pattern, line_width} = e.getEdgeStyle(this.page);
            let options = EdgeOptions.new();
            options.set_bend_degrees(bend);
            options.set_thickness(line_width);
            options.set_dash_pattern(new Uint8Array(dash_pattern));
            options.set_color(new Vec4(...color));
            this._canvas!.add_edge(start_glyph, end_glyph, options);
        }
        this._requestRedraw();
    }

    _updateClassScale(){
        const max_glyph_scale = 1.0;
        const min_glyph_scale = 0.1;
        let [xmin, xmax] = this.current_xrange();
        let [ymin, ymax] = this.current_yrange();
        let xstep = (this.clientWidth - this.leftMargin - this.rightMargin) / (xmax - xmin);
        let ystep = (this.clientHeight - this.topMargin - this.bottomMargin) / (ymax - ymin);
        let min_step = Math.min(xstep, ystep);
        this._glyph_scale = Math.max(Math.min(min_step / 100, max_glyph_scale), min_glyph_scale);
        this._canvas!.set_glyph_scale(this._glyph_scale);
    }

    _draw(){
        this._updateClassScale();
        if(this.scale_update_zoom){
            this.dispatchCustomEvent("scale-update", { type : "zoom" });
        } else if(this.scale_update_pan){
            this.dispatchCustomEvent("scale-update", { type : "pan" });
        }
        this.scale_update_zoom = false;
        this.scale_update_pan = false;
        this._canvas!.render();
        this.dispatchCustomEvent("draw", {});
        this.updateMouseoverClass([this._previousMouseX, this._previousMouseY]);
    }
}
customElements.define('sseq-chart', ChartElement);
