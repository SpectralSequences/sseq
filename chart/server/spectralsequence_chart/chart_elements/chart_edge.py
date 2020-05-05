from uuid import uuid4
from .. import utils

class ChartEdge:
    def __init__(self, sseq, edge_type, **kwargs):
        self._sseq = sseq
        self.type = edge_type
        utils.copy_fields_from_kwargs(self, kwargs)

        if "uuid" not in kwargs:
            self.uuid = str(uuid4())

        if "source" not in kwargs:
            raise ValueError("""Edge is missing mandatory argument "source".""")

        if "target" not in kwargs:
            raise ValueError("""Edge is missing mandatory argument "target".""")

        if "visible" not in kwargs:
            self.visible = True

        if self.source is not str:
            self.source = self.source.uuid
        if self.target is not str:
            self.target = self.target.uuid
        self._source = self._sseq.classes[self.source]
        self._target = self._sseq.classes[self.target]
        self._source._edges.append(self)
        self._target._edges.append(self)

    def get_source(self):
        return self._source

    def get_target(self):
        return self._target

    def replace_source(self, **kwargs):
        self._source.replace(**kwargs)
    
    def replace_target(self, **kwargs):
        self._target.replace(**kwargs)

    def delete(self):
        del self._sseq.edges[self.uuid]
        # del e._source.edges[e]
        # del e._target.edges[e]

    def set_bend(self, bend):
        self.bend = bend
        

    @staticmethod
    def from_json(sseq, json):
        edge_type = json["type"]
        json["source"] = sseq.classes[json["source"]]
        json["target"] = sseq.classes[json["target"]]
        if edge_type == ChartDifferential.__name__:
            return ChartDifferential(sseq, json.pop("page"), **json)
        if edge_type == ChartStructline.__name__:
            return ChartStructline(sseq, **json)
        if edge_type == ChartExtension.__name__:
            return ChartExtension(sseq, **json)

    def to_json(self):
        return utils.public_fields(self)

class ChartDifferential(ChartEdge):
    def __init__(self, sseq, page, **kwargs):
        super().__init__(sseq, ChartDifferential.__name__, **kwargs)
        self.page = page

class ChartStructline(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, ChartStructline.__name__, **kwargs)


class ChartExtension(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, ChartExtension.__name__, **kwargs)