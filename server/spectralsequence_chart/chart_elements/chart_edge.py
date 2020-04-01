from uuid import uuid4

from .. import utils

class ChartEdge:
    def __init__(self, sseq, edge_type, **kwargs):
        self._sseq = sseq
        self.type = edge_type
        self.uuid = uuid4()
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

    def replace_source(self, **kwargs):
        self._source.replace(**kwargs)
    
    def replace_target(self, **kwargs):
        self._target.replace(**kwargs)


    def to_json(self):
        return utils.public_fields(self)

class ChartDifferential(ChartEdge):
    def __init__(self, sseq, page, **kwargs):
        super().__init__(sseq, "differential", **kwargs)
        self.page = page

    def replace_source():
        pass

class ChartStructline(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, "structline", **kwargs)


class ChartExtension(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, "extension", **kwargs)