"use strict";
function KeyError(message) {
    this.message = message;
    this.name = 'KeyError';
    console.error(message);
}
exports.assign_fields = function assign_fields(obj, kwargs, fields) {
    for (let field of fields) {
        switch (field["type"]) {
            case "mandatory":
                exports.assign_kwarg_mandatory(obj, kwargs, field["field"]);
                break;
            case "optional":
                exports.assign_kwarg_optional(obj, kwargs, field["field"]);
                break;
            case "default":
                exports.assign_kwarg_default(obj, kwargs, field["field"], field["default"]);
                break;
            default:
                throw new KeyError(`Unknown field type ${field["type"]}`);
        }
    }
};
exports.assign_kwarg_mandatory = function assign_kwarg_mandatory(obj, kwargs, field) {
    if (field === undefined) {
        throw new TypeError("field is undefined.");
    }
    if (kwargs === undefined) {
        throw new TypeError("kwargs is undefined.");
    }
    if (kwargs[field] === undefined) {
        throw new KeyError(`Argument kwargs is missing mandatory field ${field}. kwargs is : ${JSON.stringify(kwargs)}`);
    }
    else {
        obj[field] = kwargs[field];
    }
};
exports.assign_kwarg_optional = function assign_kwarg_optional(obj, kwargs, field) {
    if (field === undefined) {
        throw new TypeError("field is undefined.");
    }
    if (kwargs === undefined) {
        throw new KeyError("kwargs is undefined!");
    }
    if (kwargs[field] === undefined) {
    }
    else {
        obj[field] = kwargs[field];
    }
};
exports.assign_kwarg_default = function assign_kwarg_default(obj, kwargs, field, default_value) {
    if (field === undefined) {
        throw new TypeError("field is undefined.");
    }
    if (kwargs === undefined) {
        throw new KeyError("kwargs is undefined!");
    }
    if (kwargs[field] === undefined) {
        obj[field] = default_value;
    }
    else {
        obj[field] = kwargs[field];
    }
};
exports.public_fields = function public_fields(obj) {
    result = {};
    for (let [key, value] of Object.entries(obj)) {
        if (!key.startsWith("_")) {
            result[key] = value;
        }
    }
    return result;
};
