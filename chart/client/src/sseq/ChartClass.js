let utils = require("./utils.js");
let ChartShape = require("./ChartShape.js").ChartShape;

class ChartClass {
    constructor(sseq, kwargs) {
        this._sseq = sseq;
        this._valid = true;
        this._x_offset = 0;
        this._y_offset = 0;
        
        // utils.assign_fields(this, kwargs, [
        //     { "type" : "mandatory", "field" : "x" },
        //     { "type" : "mandatory", "field" : "y" },
        //     { "type" : "optional", "field" : "idx" },
        //     { "type" : "default",   "field" : "name",             "default" : "" },
        //     { "type" : "default",   "field" : "transition_pages", "default" : [] },
        //     { "type" : "mandatory", "field" : "node_list" },
        //     { "type" : "default",   "field" : "transition_pages", "default" : [] },
        //     { "type" : "default",   "field" : "visible",          "default" : true },
        //     { "type" : "optional",  "field" : "xoffset" },
        //     { "type" : "optional",  "field" : "yoffset" },
        //     { "type" : "optional",  "field" : "tooltip" },
        //     { "type" : "optional",  "field" : "uuid" },
        // ]);
        
        // TODO: new utils function that ensures no "_" fields present, raises error "bad serialized class".
        Object.assign(this, kwargs);
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

    draw(context) {
        let node = this._node;
        context.save();

        if(node.opacity) {
            context.opacity = node.opacity;
        }

        if(node.color) {
            context.fillStyle = node.color;
            context.strokeStyle = node.color;
        }

        if(node.stroke && node.stroke !== true) {
            context.strokeStyle = node.stroke;
        }

        if(node.fill && node.fill !== true) {
            context.fillStyle = node.fill;
        }

        if(node.highlight) {
            if(node.hcolor) {
                context.fillStyle = node.hcolor;
                context.strokeStyle = node.hcolor;
            }

            if(node.hstroke) {
                context.strokeStyle = node.hstroke;
            }

            if(node.hfill) {
                context.fillStyle = node.hfill;
            }
        }
        context.lineWidth = Math.min(3, node.size * node.scale / 20); // Magic number
        this._path = ChartShape.draw(node.shape, context, this._canvas_x, this._canvas_y, this._size * node.scale, node);
        context.restore();
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

exports.ChartClass = ChartClass;