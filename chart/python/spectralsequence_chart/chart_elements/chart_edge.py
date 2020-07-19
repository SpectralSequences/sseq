from uuid import uuid4
from .. import utils

class ChartEdge:
    def __init__(self, sseq, edge_type, *, source, target, **kwargs):
        self._sseq = sseq
        self.type = edge_type
        if "uuid" in kwargs:
            self.uuid = kwargs["uuid"]
        else:
            self.uuid = str(uuid4())

        self.source = source
        self.target = target

        setattr(self, utils.PROPERTY_PREFIX + "source",  self.source.uuid)
        setattr(self, utils.PROPERTY_PREFIX + "target",  self.target.uuid)

        [message_args, message_kwargs] = utils.arguments(
            type = self.type,
            uuid = self.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        )
        if "page" in kwargs:
            message_kwargs["page"] = kwargs["page"]
        self._sseq.add_batched_message(self.uuid + ".new", "chart.edge.add", message_args, message_kwargs)
        utils.copy_fields_from_kwargs(self, kwargs)

        if "visible" not in kwargs:
            self.visible = True

        self.source._edges.append(self)
        self.target._edges.append(self)

    def needs_update(self):
        self._sseq.add_edge_to_update(self)

    @utils.sseq_property
    def color(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def bend(self, storage_name):
        self.needs_update()

    @property
    def line_width(self):
        return getattr(self, utils.PROPERTY_PREFIX + "lineWidth", None)
    
    @line_width.setter
    def line_width(self, v):
        setattr(self, utils.PROPERTY_PREFIX + "lineWidth",  v)
        self.needs_update()

    def replace_source(self, **kwargs):
        self.source.replace(**kwargs)
    
    def replace_target(self, **kwargs):
        self.target.replace(**kwargs)

    def delete(self):
        self._sseq.add_edge_to_delete(self)
        del self._sseq._edges[self.uuid]
        del self.source._edges[self.source._edges.index(self)]
        del self.target._edges[self.target._edges.index(self)]

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
        result = utils.public_fields(self)
        result["type"] = type(self).__name__
        return result

class ChartDifferential(ChartEdge):
    def __init__(self, sseq, page, **kwargs):
        super().__init__(sseq, ChartDifferential.__name__, **kwargs, page = page)
        self.page = page

class ChartStructline(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, ChartStructline.__name__, **kwargs)

class ChartExtension(ChartEdge):
    def __init__(self, sseq, **kwargs):
        super().__init__(sseq, ChartExtension.__name__, **kwargs)