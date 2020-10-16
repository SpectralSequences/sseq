import { ChartShape } from "./ChartShape";
import { PageProperty, PagePropertyOrValueToPageProperty, initialPagePropertyValue } from "./PageProperty";
import { v4 as uuidv4 } from 'uuid';
import { INFINITY } from "../infinity";
function arrayEqual(array1, array2) {
    return array1.length === array2.length &&
        array1.every((array1_i, i) => array1_i === array2[i]);
}
export class ChartClass {
    constructor(kwargs) {
        this.visible = undefined;
        this._valid = true;
        this._x_offset = 0;
        this._y_offset = 0;
        if (!kwargs.degree) {
            throw new TypeError(`Mandatory constructor argument "degree" is missing.`);
        }
        this.degree = kwargs.degree;
        if (kwargs.type && kwargs.type !== this.constructor.name) {
            throw Error(`Internal error: bad value for parameter "type"`);
        }
        this.idx = kwargs.idx;
        this.uuid = kwargs.uuid || uuidv4();
        this.max_page = kwargs.max_page || INFINITY;
        let errorContext = " in constructor for ChartClass.";
        this.name = initialPagePropertyValue(kwargs.name, "", "name", errorContext);
        this.shape = initialPagePropertyValue(kwargs.shape, "default", "shape", errorContext);
        this.scale = initialPagePropertyValue(kwargs.scale, 1, "scale", errorContext);
        this.color = initialPagePropertyValue(kwargs.color, "default", "color", errorContext);
        this.stroke = initialPagePropertyValue(kwargs.stroke, "default", "stroke", errorContext);
        this.fill = initialPagePropertyValue(kwargs.fill, "default", "fill", errorContext);
        this.opacity = initialPagePropertyValue(kwargs.opacity, 1, "opacity", errorContext);
        this.visible = initialPagePropertyValue(kwargs.visible, true, "visible", errorContext);
        this.x_nudge = initialPagePropertyValue(kwargs.x_nudge, 0, "x_nudge", errorContext);
        this.y_nudge = initialPagePropertyValue(kwargs.y_nudge, 0, "y_nudge", errorContext);
        this.dom_content = kwargs.dom_content || new Map();
        this.user_data = kwargs.user_data || new Map();
        this._displayed = false;
    }
    _updateProjection() {
        if (!this._sseq) {
            throw Error("Undefined _sseq.");
        }
        let x = 0;
        let y = 0;
        for (let i = 0; i < this._sseq.num_gradings; i++) {
            x += this._sseq.x_projection[i] * this.degree[i];
            y += this._sseq.y_projection[i] * this.degree[i];
        }
        this.x = x;
        this.y = y;
    }
    update(kwargs) {
        // TODO: new utils function that ensures no "_" fields present, raises error "bad serialized class".
        if (kwargs.degree) {
            if (!arrayEqual(this.degree, kwargs.degree)) {
                throw TypeError(`Inconsistent values for "degree".`);
            }
        }
        if (kwargs.type) {
            if (kwargs.type !== this.constructor.name) {
                throw TypeError(`Invalid value for "type"`);
            }
        }
        if (kwargs.uuid) {
            if (this.uuid !== kwargs.uuid) {
                throw TypeError(`Inconsistent values for "uuid".`);
            }
        }
        if (kwargs.idx) {
            this.idx = kwargs.idx;
        }
        if (kwargs.name) {
            this.name = PagePropertyOrValueToPageProperty(kwargs.name);
        }
        if (kwargs.max_page) {
            this.max_page = kwargs.max_page;
        }
        if (kwargs.shape) {
            this.shape = PagePropertyOrValueToPageProperty(kwargs.shape);
        }
        if (kwargs.color) {
            this.color = PagePropertyOrValueToPageProperty(kwargs.color);
        }
        if (kwargs.fill) {
            this.fill = PagePropertyOrValueToPageProperty(kwargs.fill);
        }
        if (kwargs.stroke) {
            this.stroke = PagePropertyOrValueToPageProperty(kwargs.stroke);
        }
        if (kwargs.scale) {
            this.scale = PagePropertyOrValueToPageProperty(kwargs.scale);
        }
        if (kwargs.opacity) {
            this.opacity = PagePropertyOrValueToPageProperty(kwargs.opacity);
        }
        if (kwargs.visible) {
            this.visible = PagePropertyOrValueToPageProperty(kwargs.visible);
        }
        if (kwargs.x_nudge) {
            this.x_nudge = PagePropertyOrValueToPageProperty(kwargs.x_nudge);
        }
        if (kwargs.y_nudge) {
            this.y_nudge = PagePropertyOrValueToPageProperty(kwargs.y_nudge);
        }
        if (kwargs.dom_content) {
            this.dom_content = kwargs.dom_content;
        }
    }
    toJSON() {
        return {
            type: this.constructor.name,
            degree: this.degree,
            idx: this.idx,
            uuid: this.uuid,
            name: this.name,
            max_page: this.max_page,
            shape: this.shape,
            color: this.color,
            fill: this.fill,
            stroke: this.stroke,
            scale: this.scale,
            opacity: this.opacity,
            visible: this.visible,
            x_nudge: this.x_nudge,
            y_nudge: this.y_nudge,
            dom_content: this.dom_content,
            user_data: this.user_data
        };
    }
    static fromJSON(obj) {
        return new ChartClass(obj);
    }
    setPosition(x, y, size) {
        if (isNaN(x) || isNaN(y) || isNaN(size)) {
            console.error(this, x, y, size);
            throw new TypeError("class.setPosition called with bad argument.");
        }
        this._canvas_x = x;
        this._canvas_y = y;
        this._size = size;
    }
    getDrawParams(x, y) {
        let node = this._node;
        if (node === undefined) {
            throw TypeError("Undefined node.");
        }
        // TODO: This guard probably shouldn't need to be here
        if (this._size === undefined || this.x === undefined || this.y === undefined) {
            throw TypeError("Undefined field x, y, or _size");
        }
        return {
            shape: node.shape,
            size: this._size * node.scale,
            x: x,
            y: y,
            fillQ: node.fill !== false,
            strokeQ: node.stroke !== false,
            node: node
        };
    }
    _getStyleForCanvasContext() {
        let result = {};
        let node = this._node;
        if (node === undefined) {
            throw TypeError("Undefined node.");
        }
        result["shape"] = node.shape;
        if (node.opacity) {
            result["opacity"] = node.opacity;
        }
        if (node.color) {
            if (node.fill !== false) {
                result["fillStyle"] = node.color;
            }
            if (node.stroke !== false) {
                result["strokeStyle"] = node.color;
            }
        }
        if (node.stroke && node.stroke !== true && node.stroke !== "default") {
            result["strokeStyle"] = node.stroke;
        }
        if (node.fill && node.fill !== true && node.fill !== "default") {
            result["fillStyle"] = node.fill;
        }
        // TODO: should remove this?
        if (this._size === undefined) {
            throw TypeError("Undefined field _size");
        }
        result["lineWidth"] = Math.min(3, this._size * node.scale / 20); // Magic number
        return result;
    }
    _drawPrepareCanvasContext(context) {
        Object.assign(context, this._getStyleForCanvasContext());
    }
    // drawHighlight(context : CanvasRenderingContext2D) {
    //     context.save();
    //     context.beginPath();
    //     context.fillStyle = this._sseq.highlightColor;
    //     let params = this.getDrawParams(this._canvas_x, this._canvas_y);
    //     params.size *= this._sseq.highlightScale;
    //     ChartShape.outline(context, params);
    //     context.fill();
    //     context.restore();
    // }
    isDisplayed() {
        return this._displayed;
    }
    draw(context, x, y) {
        if (x === undefined) {
            x = this._canvas_x;
        }
        if (y === undefined) {
            y = this._canvas_y;
        }
        context.save();
        this._drawPrepareCanvasContext(context);
        let params = this.getDrawParams(x, y);
        context.beginPath();
        ChartShape.draw(context, params);
        context.restore();
    }
    getMouseoverPath(x, y) {
        let path = new Path2D();
        let params = this.getDrawParams(x, y);
        params.size *= this._sseq.mouseoverScale;
        ChartShape.outline(path, params);
        return path;
    }
    /**
     * Gets the node to be drawn for the class on the given page. Used primarily by display.
     * @param c
     * @param page
     * @returns {*}
     */
    getNode(page) {
        return {
            shape: this.shape[page],
            scale: this.scale[page],
            color: this.color[page],
            stroke: this.stroke[page],
            fill: this.fill[page],
            opacity: this.opacity[page]
        };
    }
    _drawOnPageQ(page) {
        return page <= this.max_page; // && this.visible;
    }
    _inRangeQ(xmin, xmax, ymin, ymax) {
        // TODO: maybe remove the need for this guard?
        if (this.x === undefined || this.y === undefined) {
            throw TypeError("Undefined field x or y");
        }
        return xmin <= this.x && this.x <= xmax && ymin <= this.y && this.y <= ymax;
    }
    getNameCoord(page) {
        let tooltip = "";
        let name = this.name.constructor === PageProperty ? this.name[page] : this.name;
        if (name !== "") {
            tooltip = `\\(\\large ${name}\\)&nbsp;&mdash;&nbsp;`;
        }
        tooltip += `(${this.x}, ${this.y})`;
        return tooltip;
    }
    getXOffset(page) {
        let x_offset;
        let classes = this._sseq.classes_in_degree(...this.degree);
        let num_classes = classes.length;
        if (this.idx === undefined) {
            throw TypeError("Class has undefined index.");
        }
        let idx = this.idx;
        let out = (idx - (num_classes - 1) / 2) * this._sseq.offset_size;
        if (isNaN(out)) {
            console.error("Invalid offset for class:", this);
            x_offset = 0;
        }
        else {
            x_offset = out;
        }
        let x_nudge = this.x_nudge[page] ? this.x_nudge[page] : 0;
        return x_offset + x_nudge;
    }
    getYOffset(page) {
        let y_offset = 0;
        let y_nudge = this.y_nudge[page] ? this.y_nudge[page] : 0;
        return y_offset + y_nudge;
    }
}
