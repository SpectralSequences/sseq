import * as utils from "./utils.js";
import { ChartShape } from "./ChartShape.js";
import { ChartNode } from "./ChartNode.js";

export class ChartClass {
    constructor(sseq, kwargs) {
        this._sseq = sseq;
        this._valid = true;
        this._x_offset = 0;
        this._y_offset = 0;
        
        // TODO: new utils function that ensures no "_" fields present, raises error "bad serialized class".
        Object.assign(this, kwargs);
        this.name = this.name || "";
        this.transition_pages = this.transition_pages || [];
        this.node_list = this.node_list || [
            new ChartNode({
                "shape" : "circle"
            })
        ];
        if(this.visible === undefined) {
            this.visible = true;
        }
    }

    setPosition(x, y, size) {
        if(isNaN(x) || isNaN(y) || isNaN(size)){
            console.error(this, x, y, size);
            throw "class.setPosition called with bad argument.";
        }
        this._canvas_x = x;
        this._canvas_y = y;
        this._size = size;
    }

    getDrawParams(x, y) {
        let node = this._node;
        return {
            shape : node.shape,
            size : this._size * node.scale,
            x : x,
            y : y,
            fillQ : node.fill !== false,
            strokeQ : node.stroke !== false,
            node : node
        };
    }

    _getStyleForCanvasContext(){
        let result = {};
        let node = this._node;
        if(node.opacity) {
            result.opacity = node.opacity;
        }

        if(node.color) {
            if(node.fill !== false){
                result.fillStyle = node.color;
            }
            if(node.stroke !== false){
                result.strokeStyle = node.color;
            }
        }

        if(node.stroke && node.stroke !== true) {
            result.strokeStyle = node.stroke;
        }

        if(node.fill && node.fill !== true) {
            result.fillStyle = node.fill;
        }
        result.lineWidth = Math.min(3, this._size * node.scale / 20); // Magic number
        return result;
    }

    _drawPrepareCanvasContext(context){
        Object.assign(context, this._getStyleForCanvasContext());
    }

    drawHighlight(context) {
        context.save();
        context.beginPath();
        context.fillStyle = this._sseq.highlightColor;
        let params = this.getDrawParams(this._canvas_x, this._canvas_y);
        params.size *= this._sseq.highlightScale;
        ChartShape.outline(context, params);
        context.fill();
        context.restore();
    }

    isDisplayed(){
        return this._displayed;
    }

    draw(context, x, y) {
        if(x === undefined){
            x = this._canvas_x;
        }
        if(y === undefined){
            y = this._canvas_y;
        }
        context.save();
        this._drawPrepareCanvasContext(context)
        let params = this.getDrawParams(x, y);
        context.beginPath();
        ChartShape.draw(context, params);
        context.restore();
    }

    getMouseoverPath(x, y){
        let path = new Path2D();
        let params = this.getDrawParams(x, y);
        params.size *= this._sseq.mouseoverScale;
        ChartShape.outline(path, params);
        return path;
    }

    _drawOnPageQ(page){
        let idx = this._getPageIndex(page);
        return this.node_list[idx] != null && this.visible;
    }

    _inRangeQ(xmin, xmax, ymin, ymax){
        return xmin <= this.x && this.x <= xmax && ymin <= this.y && this.y <= ymax;
    }

    _getPageIndex(page){
        if( page === undefined ) {
            return this.node_list.length - 1;
        } else if( page === this._last_page ) {
            return this._last_page_idx;
        }
        let page_idx = this.transition_pages.length;
        for(let i = 0; i < this.transition_pages.length; i++){
            if(this.transition_pages[i] >= page){
                page_idx = i;
                break;
            }
        }
        this._last_page = page;
        this._last_page_idx = page_idx;
        return page_idx;
    }

    getNameCoord(){
        let tooltip = "";
        if (this.name !== "") {
            tooltip = `\\(\\large ${this.name}\\)&nbsp;&mdash;&nbsp;`;
        }
        tooltip += `(${this.x}, ${this.y})`;
        return tooltip;
    }

    getXOffset() {
        let x_offset;
        let classes = this._sseq.classes_by_degree.get([this.x, this.y]);
        let num_classes = classes.length;
        let idx = this.idx;
        let out = (idx - (num_classes - 1) / 2) * this._sseq.offset_size;
        if (isNaN(out)) {
            console.error("Invalid offset for class:", this);
            x_offset = 0;
        } else {
            x_offset = out; 
        }

        let x_nudge = this.x_nudge ? this.x_nudge : 0;
        return x_offset + x_nudge;
    }

    getYOffset() {
        let y_offset = 0;
        let y_nudge = this.y_nudge ? this.y_nudge : 0;
        return y_offset + y_nudge;
    }

    toJSON() {
        return utils.public_fields(this);
    }
}