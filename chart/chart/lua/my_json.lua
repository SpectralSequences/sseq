json = require "json"

module = {}

function shallowcopy(orig)
    local orig_type = type(orig)
    local copy
    if orig_type == 'table' then
        copy = {}
        for orig_key, orig_value in pairs(orig) do
            copy[orig_key] = orig_value
        end
    else -- number, string, boolean, etc
        copy = orig
    end
    return copy
end

function module:encode_helper(obj)
    local o
    if obj.tojson then
        o = obj.tojson()
    else:
        o = shallowcopy(obj)
    end
    for k, v in pairs(o):
        o[k] = module.encode_helper(v)
    end
    return o
end

function module:encode(obj)
    return json.encode(module.encode_helper(obj))
end

function module:decode(type_map, str)
    obj = json.decode(str)
    module.decode_helper(type_map, obj)
end

function module:decode_helper(type_map, str)
    for k, v in pairs(obj) do
        if v["type"] ~= nil then
        end
    end
end

module.decode = json.decode

return module
