json = require "json"

PageProperty = {type = "PageProperty"}
PageProperty.__index = function indexPageProperty(table, index)
    if type(index) ==
end


Chart = {type = "SseqChart"}

function Chart:new()
    o = {}
    setmetatable(o, self)
    self.__index = self
    self.classes =  {}
    self.edges = {}
    return o
end

ChartClass = {type = "ChartClass"}
function ChartClass:new()

end
