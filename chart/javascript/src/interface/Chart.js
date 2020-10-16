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


export class ChartElement extends HTMLElement {
    get clipped(){
        return true
    }

    constructor(){
        super();
        this.nextPage = this.nextPage.bind(this);
        this.previousPage = this.previousPage.bind(this);
        this.handleScaleUpdate = this.handleScaleUpdate.bind(this);
        this.node_buffers = {};
    }

    connectedCallback(){
        this.handleMouseStateUpdate = this.handleMouseStateUpdate.bind(this);
        let elt = this;
        while(elt !== undefined && elt.nodeName !== "SSEQ-DISPLAY"){
            elt = elt.parentElement;
        }
        if(elt === undefined){
            throw Error("sseq-class-highlighter must be a descendant of sseq-display.");
        }
        this.disp = elt;
        this.disp.addEventListener("scale-update", this.handleScaleUpdate);
        this.disp.addEventListener("mouse-state-update", this.handleMouseStateUpdate);
    }

    nextPage(){
        this.changePage(1);
    }

    previousPage(){
        this.changePage(-1);
    }

    changePage(delta){
        let min_idx = 0;
        let max_idx = this.sseq.page_list.length - 1;
        let new_idx = Math.min(Math.max(this.page_idx + delta, min_idx), max_idx)
        if (new_idx !== this.page_idx) {
            this.setPage(new_idx);
            this.update();
        }
    }

    /**
     * Set the spectral sequence to display.
     * @param ss
     */
    setSseq(sseq){
        if(this.sseq) {
            this.sseq.removeListener("update", this.disp.updateBatch);
        }
        this.sseq = sseq;
        // The sseq object contains the list of valid pages. Always includes at least 0 and infinity.
        if(this.sseq.initial_page_idx){
            this.page_idx = this.sseq.initial_page_idx;
        } else {
            this.page_idx = 0;
        }
        // if(this.page_idx >= this.sseq.page_list.length){
        //     console.warn(`Warning: min_page_idx ${this.sseq.min_page_idx} greater than page list length ${this.sseq.page_list.length}. Using 0 for min_page_idx instead.`);
        //     this.page_idx = 0;
        //     this.min_page_idx = 0;
        // }
        this.setPage();
        this.disp.setInitialXRange(...sseq.initial_x_range);
        this.disp.setInitialYRange(...sseq.initial_y_range);
        this.disp.setXRange(...sseq.x_range);
        this.disp.setYRange(...sseq.y_range);
        this.disp.resize();

        // if(sseq.gridStyle){
        //     this.gridStyle = sseq.gridStyle;
        // }

        this.sseq.on('update', this.disp.updateBatch);
        this.disp.update();
    }

    /**
     * Update this.page and this.pageRange to reflect the value of page_idx.
     */
    setPage(idx){
        if (!this.sseq){
            return;  
        } 

        if(idx !== undefined){
            this.page_idx = idx;
        }
        this.pageRange = this.sseq.page_list[this.page_idx];

        if(Array.isArray(this.pageRange)){
            this.page = this.pageRange[0];
        } else {
            this.page = this.pageRange;
        }
        this.disp.emit("page-change", this.pageRange, this.page_idx);
    }

    getClassAtScreenCoordinates(x, y){
        return this.classesToDraw.find((c) => 
            this.disp.context.isPointInPath(c._path, x - c._canvas_x, y - c._canvas_y)
        );
    }

    handleMouseStateUpdate(evt){
        if(!this.sseq){
            return;
        }
        let mouseState = evt.detail[0];
        let newMouseoverClass = this.getClassAtScreenCoordinates(mouseState.screen_x, mouseState.screen_y);
        mouseState.mouseoverClass = newMouseoverClass;
        let c = this.classesToDraw[0];
        if (this.mouseoverClass) {
            if(newMouseoverClass === this.mouseoverClass) {
                return;
            } else {
                this.disp.emit("mouseout-class", this.mouseoverClass, mouseState);
                this.mouseoverClass = null;
                this.disp.updateBatch();
            }
        }
        if(newMouseoverClass) {
            this.mouseoverClass = newMouseoverClass;
            this.disp.emit("mouseover-class", newMouseoverClass, this.mouseState);
            this.disp.updateBatch();
        }
    }

    update(){
        this.updateElements();
        this.disp.update();
    }

    handleScaleUpdate(){
        this.updateElements();
    }

    updateElements(disp){
        if(!disp){
            disp = this.disp;
        }
        let [classes, edges] = this.sseq.getElementsToDraw(
            this.pageRange, 
            disp.xmin - 1, disp.xmax + 1, disp.ymin - 1, disp.ymax + 1
        );
        let size = Math.max(Math.min(disp.dxScale(1), -disp.dyScale(1), this.sseq.max_class_size), this.sseq.min_class_size) * this.sseq.class_scale;
        this.classesToDraw = classes;
        this.edgesToDraw = edges;
        for(let c of classes) {
            c.setPosition( 
                disp.xScale(c.x) + c.getXOffset(), 
                disp.yScale(c.y) + c.getYOffset(), 
                size
            );
        }
    }

    _highlightClasses(disp, context) {
        for (let c of this.classesToDraw) {
            if(c._highlight){
                c.drawHighlight(context);
            }
        }
    }

    _drawClasses(disp, context, classes) {
        const BUFFER_WIDTH = 64;
        const BUFFER_HEIGHT = 64;
        let groupedClasses = groupByArray(classes, c => JSON.stringify(c._getStyleForCanvasContext()))

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

    _drawEdges(disp, context, edges){
        let grouped_edges = groupByArray(edges, (e) => JSON.stringify([e.color[this.page]]));//, e.lineWidth[this.page], e.opacity[this.page], e.dash[this.page]]));
        for(let edge_group of grouped_edges){
            context.save();
            let first_edge = edge_group.values[0];
            let color = first_edge.color[this.page] || "black";
            if(color === "default"){
                color = "black";
            }
            context.strokeStyle = color;
            if(first_edge.lineWidth){
                let lineWidth = first_edge.lineWidth[this.page];
                if(lineWidth !== "default"){
                    context.lineWidth = lineWidth;
                }
            }
            if(first_edge.opacity){
                context.globalAlpha = first_edge.opacity[this.page];
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

                let source = e.source;
                let target = e.target;
                if(!source || ! target){
                    throw ValueError(`Edge ${e} has undefined source or target node`);
                }
                e._sourceOffset = e.sourceOffset || {x: 0, y: 0};
                e._targetOffset = e.targetOffset || {x: 0, y: 0};

                let sourceX = source._canvas_x + e._sourceOffset.x;
                let sourceY = source._canvas_y + e._sourceOffset.y;
                let targetX = target._canvas_x + e._targetOffset.x;
                let targetY = target._canvas_y + e._targetOffset.y;
                let bend = e.bend[this.page];
                if(bend){//&& e.bend !== 0
                    let distance = Math.sqrt((targetX - sourceX)*(targetX - sourceX) + (targetY - sourceY)*(targetY - sourceY));
                    let looseness = 0.4;
                    if(e.looseness){
                        looseness = e.looseness;
                    }
                    let angle = Math.atan((targetY - sourceY)/(targetX - sourceX));
                    let bendAngle = - bend * Math.PI/180;
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

    paint(disp, ctx) {
        if(!this.sseq){
            return;
        }
        this._highlightClasses(disp, ctx);
        this._drawEdges(disp, ctx, this.edgesToDraw);
        this._drawClasses(disp, ctx, this.classesToDraw);
    }

    toJSON(){
        return this;
    }
}
customElements.define('sseq-chart', ChartElement);