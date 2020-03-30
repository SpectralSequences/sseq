let utils = require("./utils.js");

class ChartNode {
    constructor(kwargs) {
        utils.assign_fields(this, kwargs, [
            { "type" : "mandatory", "field" : "shape"},
            { "type" : "default", "field" : "scale", "default" : 1},
            { "type" : "optional", "field" : "fill"},
            { "type" : "optional", "field" : "stroke"},
            { "type" : "optional", "field" : "color"},
            { "type" : "optional", "field" : "opacity"},            
        ]);
    }

    toJSON() {
        return utils.public_fields(this);
    }
}

exports.ChartNode = ChartNode;