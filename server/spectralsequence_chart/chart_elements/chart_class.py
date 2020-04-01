import threading
from uuid import uuid4

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
        # self.node_list = [ n.idx for n in self.node_list ]

    def __repr__(self):
        return f"ChartClass({self.x},{self.y})"

    def to_json(self):
        return utils.public_fields(self)

    def get_page_idx(self, page):
        for i, v in self.transition_pages:
            if v > page:
                return i
        return len(self.transition_pages)

    async def set_node_field_by_idx_a(self, idx, field, value):
        n = self._sseq.nodes[self.node_list[idx]].copy()
        setattr(n, field, value)
        n = await self._sseq.get_node_a(n)
        self.node_list[idx] = n.idx


    async def set_field_a(self, field, value):
        with self._lock:
            for i in range(len(self.node_list)):
                await self.set_node_field_by_idx_a(i, field, value)
                return True

    async def set_field_on_page_a(self, page, field, value):
        with self._lock:
            i = self.get_page_idx(page)
            await self.set_node_field_by_idx_a(i, field, value)
            return True

    async def add_page_a(self, page, node=None):
        if page in self.transition_pages:
            return False
        with self._lock:
            if page in self.transition_pages:
                return False            
            idx = self.get_page_idx(page)
            self.transition_pages.insert(idx, page)
            self.node_list.insert(idx+1, node)
            return True