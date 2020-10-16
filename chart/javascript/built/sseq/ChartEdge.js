import { initialPagePropertyValue, PagePropertyOrValueToPageProperty } from "./PageProperty";
import { INFINITY } from "../infinity";
import { v4 as uuidv4 } from 'uuid';
export class ChartEdge {
    // arrow_type : ArrowSpecifier; // TODO??
    constructor(kwargs) {
        if (!kwargs.source_uuid) {
            throw TypeError(`Missing mandatory argument "source_uuid"`);
        }
        if (!kwargs.target_uuid) {
            throw TypeError(`Missing mandatory argument "target_uuid"`);
        }
        let errorContext = "";
        this.uuid = kwargs.uuid || uuidv4();
        this._source_uuid = kwargs.source_uuid;
        this._target_uuid = kwargs.target_uuid;
        this.bend = initialPagePropertyValue(kwargs.bend, 0, "bend", errorContext);
        this.color = initialPagePropertyValue(kwargs.color, "black", "color", errorContext);
        this.dash_pattern = initialPagePropertyValue(kwargs.dash_pattern, 0, "dash_pattern", errorContext);
        this.line_width = initialPagePropertyValue(kwargs.line_width, 3, "line_width", errorContext);
        this.user_data = kwargs.user_data || new Map();
    }
    update(kwargs) {
        if (kwargs.source_uuid) {
            if (this._source_uuid !== kwargs.source_uuid) {
                throw TypeError(`Inconsistent values for "source_uuid".`);
            }
        }
        if (kwargs.target_uuid) {
            if (this._target_uuid !== kwargs.target_uuid) {
                throw TypeError(`Inconsistent values for "target_uuid".`);
            }
        }
        if (kwargs.uuid) {
            if (this.uuid !== kwargs.uuid) {
                throw TypeError(`Inconsistent values for "uuid".`);
            }
        }
        if (kwargs.type) {
            if (kwargs.type !== this.constructor.name) {
                throw TypeError(`Invalid value for "type"`);
            }
        }
        if (kwargs.color) {
            this.color = PagePropertyOrValueToPageProperty(kwargs.color);
        }
        if (kwargs.dash_pattern) {
            this.dash_pattern = PagePropertyOrValueToPageProperty(kwargs.dash_pattern);
        }
        if (kwargs.line_width) {
            this.line_width = PagePropertyOrValueToPageProperty(kwargs.line_width);
        }
        if (kwargs.bend) {
            this.bend = PagePropertyOrValueToPageProperty(kwargs.bend);
        }
        this.user_data = this.user_data || new Map();
    }
    _drawOnPageQ(pageRange) {
        throw new Error("This should be overridden...");
    }
    toJSON() {
        return {
            type: this.constructor.name,
            uuid: this.uuid,
            source_uuid: this._source_uuid,
            target_uuid: this._target_uuid,
            color: this.color,
            dash_pattern: this.dash_pattern,
            line_width: this.line_width,
            bend: this.bend,
            user_data: this.user_data
        };
    }
}
export class ChartDifferential extends ChartEdge {
    constructor(kwargs) {
        super(kwargs);
        this.page = kwargs.page;
    }
    _drawOnPageQ(pageRange) {
        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);
    }
    toJSON() {
        let result = super.toJSON();
        result.page = this.page;
        return result;
    }
    static fromJSON(obj) {
        return new ChartDifferential(obj);
    }
}
export class ChartStructline extends ChartEdge {
    constructor(kwargs) {
        super(kwargs);
        let errorContext = "";
        this.visible = initialPagePropertyValue(kwargs.visible, true, "visible", errorContext);
    }
    update(kwargs) {
        super.update(kwargs);
        if (kwargs.visible) {
            this.visible = PagePropertyOrValueToPageProperty(kwargs.visible);
        }
    }
    _drawOnPageQ(pageRange) {
        return this.visible[pageRange[0]];
    }
    toJSON() {
        let result = super.toJSON();
        result.visible = this.visible;
        return result;
    }
    static fromJSON(obj) {
        return new ChartStructline(obj);
    }
}
export class ChartExtension extends ChartEdge {
    constructor(kwargs) {
        super(kwargs);
    }
    _drawOnPageQ(pageRange) {
        return pageRange[0] === INFINITY;
    }
    static fromJSON(obj) {
        return new ChartExtension(obj);
    }
}
