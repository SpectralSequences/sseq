import * as utils from "./utils.js";
import { ChartClass } from "./ChartClass.js";
import { INFINITY } from "../infinity.js";

export class ChartEdge {
    constructor(type, kwargs) {
        this.type = type;
        if(!("source" in kwargs)){
            throw Error(`Edge is missing argument "source".`);
        }
        if(!("target" in kwargs)){
            throw Error(`Edge is missing argument "target".`);
        }
        if(!("visible" in kwargs)) {
            this.visible = true;
        }
        Object.assign(this, kwargs);
        // if(typeof(this.source) !== ChartClass) {
        //     throw Error(`Invalid source of edge : ${JSON.stringify(this.source)}`);
        // }
        // if(typeof(this.target) !== ChartClass) {
        //     throw Error(`Invalid target of edge: ${JSON.stringify(this.target)}`);
        // }
        this._source = this.source;
        this._target = this.target;
        this.source = this.source.uuid;
        this.target = this.target.uuid;
        // utils.assign_fields(this, kwargs, [
        //     { "type" : "mandatory", "field" : "source"},
        //     { "type" : "mandatory", "field" : "target"},
        //     { "type" : "default", "field" : "visible", "default" : true},
        //     { "type" : "optional", "field" : "color"},
        //     { "type" : "optional", "field" : "opacity"},
        //     { "type" : "optional", "field" : "bend"},
        //     { "type" : "optional", "field" : "control_points"},
        //     { "type" : "optional", "field" : "arrow_type"},
        // ])
    }

    _drawOnPageQ(pageRange){
        let max_page = this.max_page || INFINITY;
        let min_page = this.min_page || 0;
        return pageRange[0] <= max_page && min_page <= pageRange[0];
    }

    toJSON() {
        return utils.public_fields(this);
    }
}

export class ChartDifferential extends ChartEdge {
    constructor(kwargs){
        super("differential", kwargs);
        utils.assign_kwarg_mandatory(this, kwargs, "page");
    }

    _drawOnPageQ(pageRange){
        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);
    }    
}

export class ChartStructline extends ChartEdge {
    constructor(kwargs){
        super("structline", kwargs);
        if(this.max_page === undefined) {
            this.max_page = INFINITY;
        }
        if(this.min_page === undefined) {
            this.min_page = 0;
        }        
    }

    _drawOnPageQ(pageRange){
        return pageRange[0] <= this.max_page && this.min_page <= pageRange[0];
    }
}

export class ChartExtension extends ChartEdge {
    constructor(kwargs){
        super("extension", kwargs);
    }

    _drawOnPageQ(pageRange){
        return pageRange[0] === INFINITY;
    }

}