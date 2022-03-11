INFINITY = require "infinity"
table = require("table")
json = require "json"

PageProperty = {type = "PageProperty"}
function PageProperty:new(value) 
    o = {}
    setmetatable(o, self)
    o.type = "PageProperty"
    o.values = {{-65535, value}}
    return o
end

function PageProperty:find_index(target_page)
    local result_idx
    for idx, page_value in ipairs(self.values) do
        if page_value[1] > target_page then
            break
        end
        result_idx = idx 
    end
    return result_idx, self.values[result_idx][1] == target_page
end

function PageProperty:__index(index)
    if type(index) == "string" then
        result = rawget(self, index)
        if result ~= nil then
            return result
        end
        return PageProperty[index]
    end
    assert(type(index) ~= "table", "Type Error: Can only assign to slice index, cannot retreive.")
    assert(type(index) == "number", string.format("Type Error: expected number, got %s.", type(index)))
    idx, _ = self:find_index(index)
    return self.values[idx][2]
end

function PageProperty:__newindex(index, value)
    if type(index) == "number" then
        self:setitem_single(index, v)
        self:merge_redundant()
        return
    end
    assert(type(index) == "string", string.format("Excepted number or string, not %s.", type(index)))
    idx = index:find(":")
    if idx == nil then
        rawset(self, index, value)
        return
    end
    assert(#index:match("[0-9]*:[0-9]*") == #index, "Invalid slice.")
    start = tonumber(index:sub(1,idx-1)) or -INFINITY
    stop = tonumber(index:sub(idx+1)) or INFINITY
    orig_value = self[stop]
    start_idx, hit_start = self:setitem_single(start, value)
    end_idx, hit_end = self:find_index(stop)
    if not hit_end and stop < INFINITY then
        end_idx = self:setitem_single(stop, orig_value)
    end
    if stop == INFINITY then
        end_idx = end_idx + 1
    end
    self:merge_redundant(start_idx, end_idx)
end

function PageProperty:setitem_single(p, v)
    idx, hit = self:find_index(p)
    if hit then
        self.values[idx][2] = v
    else
        idx = idx + 1
        table.insert(self.values, idx, {p, v})
    end
    return idx, hit
end

function PageProperty:merge_redundant(start_delete, end_delete)
    start_delete = start_delete or INFINITY
    end_delete = end_delete or -INFINITY
    local t = self.values
    local j = 2
    local n = #t
    for i=2,n do
        if 
            t[i][2] == t[j-1][2] 
            or ( start_delete < i and end_delete > i ) 
        then
            t[i] = nil
        else
            if i ~= j then
                t[j] = t[i]
                t[i] = nil
            end
            j = j + 1
        end
    end   
end

function PageProperty:__tostring() 
    return string.format("PageProperty(%s)", json.encode(self.values))
end


return PageProperty