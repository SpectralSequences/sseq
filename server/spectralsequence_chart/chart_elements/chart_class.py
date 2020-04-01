import threading
from uuid import uuid4

from . import ChartNode
from .. import utils

class ChartClass:
    def __init__(self, sseq, **kwargs):
        self._sseq = sseq
        self._edges = []
        self._lock = threading.Lock()
        self.uuid = uuid4()
        utils.assign_fields(self, kwargs, [
            { "type" : "mandatory", "field" : "x" },
            { "type" : "mandatory", "field" : "y" },
            { "type" : "optional", "field" : "idx" },
            { "type" : "default",   "field" : "name",             "default" : "" },
            { "type" : "default",   "field" : "transition_pages", "default" : [] },
            { "type" : "mandatory", "field" : "node_list" },
            { "type" : "default",   "field" : "visible",          "default" : True },
            { "type" : "optional",  "field" : "xoffset" },
            { "type" : "optional",  "field" : "yoffset" },
            { "type" : "optional",  "field" : "tooltip" },
        ])
        for (i, n) in enumerate(self.node_list):
            if type(self.node_list[i]) is int:
                self.node_list[i] = self._sseq.nodes[i].copy()
            elif type(self.node_list[i]) is dict:
                self.node_list[i] = ChartNode.from_json(self._sseq, self.node_list[i])
        # self.node_list = [ n.idx for n in self.node_list ]

    @staticmethod
    def from_json(sseq, json):
        return ChartClass(sseq, **json)

    def __repr__(self):
        return f"ChartClass({self.x},{self.y})"

    def to_json(self):
        return utils.public_fields(self)

    def get_page_idx(self, page):
        for i, v in self.transition_pages:
            if v > page:
                return i
        return len(self.transition_pages)

    def set_node_field_by_idx(self, idx, field, value):
        n = self.node_list[idx]
        setattr(n, field, value)


    def set_field(self, field, value):
        # with self._lock:
            for i in range(len(self.node_list)):
                self.set_node_field_by_idx(i, field, value)
            self._sseq.add_element_to_update(self)

    def set_field_on_page(self, page, field, value):
        # with self._lock:
            i = self.get_page_idx(page)
            self.set_node_field_by_idx(i, field, value)
            self._sseq.add_element_to_update(self)

    def add_page(self, page, node=None):
        # if page in self.transition_pages:
            # return False
        # with self._lock:
        if page in self.transition_pages:
            return False            
        idx = self.get_page_idx(page)
        self.transition_pages.insert(idx, page)
        self.node_list.insert(idx+1, node)
        self._sseq.add_element_to_update(self)

    def copy_previous_node(self, page):
        idx = self.get_page_idx(page)
        if idx == 0:
            raise ValueError("No previous node.")
        self.node_list[idx] = self.node_list[idx - 1].copy()
        self._sseq.add_element_to_update(self)

    def replace(self, **kwargs):
        n = self.node_list[-2].copy()
        self.node_list[-1] = n
        for [key, value] in kwargs.items():
            setattr(n, key, value)
        self._sseq.add_element_to_update(self)