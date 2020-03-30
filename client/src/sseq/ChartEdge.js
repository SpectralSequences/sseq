let utils = require("./utils.js");
let INFINITY = require("../infinity.js").INFINITY;

class ChartEdge {
    constructor(type, kwargs) {
        this.type = type;
        utils.assign_fields(this, kwargs, [
            { "type" : "mandatory", "field" : "source"},
            { "type" : "mandatory", "field" : "target"},
            { "type" : "default", "field" : "visible", "default" : true},
            { "type" : "optional", "field" : "color"},
            { "type" : "optional", "field" : "opacity"},
            { "type" : "optional", "field" : "bend"},
            { "type" : "optional", "field" : "control_points"},
            { "type" : "optional", "field" : "arrow_type"},
        ])
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

class ChartDifferential extends ChartEdge {
    constructor(kwargs){
        super("differential", kwargs);
        utils.assign_kwarg_mandatory(this, kwargs, "page");
    }

    _drawOnPageQ(pageRange){
        return pageRange[0] === 0 || (pageRange[0] <= this.page && this.page <= pageRange[1]);
    }    
}

class ChartStructline extends ChartEdge {
    constructor(kwargs){
        super("structline", kwargs);
        this.max_page = utils.assign_kwarg_default(kwargs, "max_page", INFINITY);
        this.min_page = utils.assign_kwarg_default(kwargs, "min_page", 0);
    }

    static _drawOnPageQ(pageRange){
        return pageRange[0] <= this.max_page && this.min_page <= pageRange[0];
    }
}

class ChartExtension extends ChartEdge {
    constructor(kwargs){
        super("extension", kwargs);
    }

    static _drawOnPageQ(pageRange){
        return pageRange[0] === infinity;
    }
}

exports.ChartEdge = ChartEdge;
exports.ChartDifferential = ChartDifferential;
exports.ChartStructline = ChartStructline;
exports.ChartExtension = ChartExtension;