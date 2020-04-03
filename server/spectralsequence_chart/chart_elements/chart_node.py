from uuid import uuid4

from .. import utils

class ChartNode:
    def __init__(self, sseq, **kwargs):
        self._sseq = sseq
        self.uuid = str(uuid4())
        utils.assign_fields(self, kwargs, [
            { "type" : "mandatory", "field" : "shape"},
            { "type" : "default", "field" : "scale", "default" : 1},
            { "type" : "optional", "field" : "fill"},
            { "type" : "optional", "field" : "stroke"},
            { "type" : "optional", "field" : "color"},
            { "type" : "optional", "field" : "opacity"},            
        ])

    @staticmethod
    def from_json(sseq, json):
        try:
            result = ChartNode(sseq, **json)
        except:
            print(json)
            raise
        return result

    def copy(self):
        d = {}
        for key in utils.public_keys(self):
            d[key] = getattr(self, key)
        return ChartNode(self._sseq, **d)

    # def update_fields(self, kwargs):
        # self._sseq

    def __hash__(self):
        return hash(tuple(getattr(self, key) for key in utils.public_keys(self)))


    def to_json(self):
        return utils.public_fields(self) 