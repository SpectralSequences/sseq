import utils

class ChartClass:
    def __init__(self, sseq, **kwargs):
        self._sseq = sseq
        self._edges = []
        utils.assign_fields(self, kwargs, [
            { "type" : "mandatory", "field" : "x" },
            { "type" : "mandatory", "field" : "y" },
            { "type" : "optional", "field" : "idx" },
            { "type" : "default",   "field" : "name",             "default" : "" },
            { "type" : "default",   "field" : "transition_pages", "default" : [] },
            { "type" : "mandatory", "field" : "node_list" },
            { "type" : "default",   "field" : "transition_pages", "default" : [] },
            { "type" : "default",   "field" : "visible",          "default" : True },
            { "type" : "optional",  "field" : "xoffset" },
            { "type" : "optional",  "field" : "yoffset" },
            { "type" : "optional",  "field" : "tooltip" },
        ])

    def __repr__(self):
        return f"ChartClass({self.x},{self.y})"

    def to_json(self):
        return utils.public_fields(self)

class ChartEdge:
    def __init__(self, sseq, edge_type, **kwargs):
        self._sseq = sseq
        self.type = edge_type
        utils.assign_fields(self, kwargs, [
            { "type" : "mandatory", "field" : "source"},
            { "type" : "mandatory", "field" : "target"},
            { "type" : "default", "field" : "visible", "default" : True},
            { "type" : "optional", "field" : "color"},
            { "type" : "optional", "field" : "opacity"},
            { "type" : "optional", "field" : "bend"},
            { "type" : "optional", "field" : "control_points"},
            { "type" : "optional", "field" : "arrow_type"},
        ])
        self._source = self.source
        self._target = self.target
        self.source = self.source.id
        self.target = self.target.id

    def get_source(self):
        return self._sseq.classes[self.source]

    def get_target(self):
        return self._sseq.classes[self.target]

    def to_json(self):
        return utils.public_fields(self)

class BasicSpectralSequenceChart:
    def __init__(self, name):
        self.name = name
        self.page_list = [2, 3]
        self.initialxRange = [0, 10]
        self.initialyRange = [0, 10]
        self.nodes = [{"shape" : "circle"}]
        self.classes = []
        self.edges = []
        self._classes_by_bidegree = {}
        self.min_page_idx = 0

    def __repr__(self):
        return f"{self.name}a"

    def add_class(self, **kwargs):
        c = ChartClass(self, **kwargs)
        c.id = len(self.classes)
        self.classes.append(c)
        pos = (c.x, c.y)
        if pos not in self._classes_by_bidegree:
            self._classes_by_bidegree[pos] = []
        self._classes_by_bidegree[pos].append(c)
        return c

    def add_edge(self, edge_type, **kwargs):
        e = ChartEdge(self, edge_type, **kwargs)
        e.id = len(self.edges)
        self.edges.append(e)
        e.get_source()._edges.append(e)
        e.get_target()._edges.append(e)
        return e

    def to_json(self):
        return utils.public_fields(self)
