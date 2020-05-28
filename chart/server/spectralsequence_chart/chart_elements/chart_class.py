from message_passing_tree.prelude import arguments
import threading
from uuid import uuid4
from . import ChartNode
from .. import utils

class ChartClass:
    def __init__(self, sseq, **kwargs):
        if "uuid" in kwargs:
            self.uuid = kwargs["uuid"]
        else:
            self.uuid = str(uuid4())
        sseq.add_batched_message(self.uuid, "chart.class.add", *arguments(new_class=self))
        self._sseq = sseq
        self._edges = []
        self._lock = threading.Lock()
        utils.copy_fields_from_kwargs(self, kwargs)

        # TODO: figure out what to do about uuids and multiple reads from same file.
        # We'd then get multiple objects with same uuid from file.
        # But if we replace all uuids on load, we will loose the cool capability of being able to track
        # a chart entity through multiple different save files.


        if "name" not in kwargs:
            self.name = ""
        
        if "transition_pages" not in kwargs:
            self.transition_pages = []
        
        if "visible" not in kwargs:
            self.visible = True

        if "x" not in kwargs:
            raise ValueError("""Class is missing mandatory argument "x".""")
        
        if "y" not in kwargs:
            raise ValueError("""Class is missing mandatory argument "y".""")

        if "node_list" not in kwargs:
            raise ValueError("""Class is missing mandatory argument "node_list".""")
        # utils.assign_fields(self, kwargs, [
        #     { "type" : "mandatory", "field" : "x" },
        #     { "type" : "mandatory", "field" : "y" },
        #     { "type" : "optional", "field" : "idx" },
        #     { "type" : "default",   "field" : "name",             "default" : "" },
        #     { "type" : "default",   "field" : "transition_pages", "default" : [] },
        #     { "type" : "mandatory", "field" : "node_list" },
        #     { "type" : "default",   "field" : "visible",          "default" : True },
        #     { "type" : "optional",  "field" : "xoffset" },
        #     { "type" : "optional",  "field" : "yoffset" },
        #     { "type" : "optional",  "field" : "tooltip" },
        # ])

        for (i, n) in enumerate(self.node_list):
            if type(self.node_list[i]) is int:
                self.node_list[i] = self._sseq.nodes[i].copy()
            elif type(self.node_list[i]) is dict:
                self.node_list[i] = ChartNode.from_json(self._sseq, self.node_list[i])

        sseq._classes[self.uuid] = self
        pos = (self.x, self.y)
        if pos not in sseq._classes_by_bidegree:
            sseq._classes_by_bidegree[pos] = []
        if not hasattr(self, "idx"):
            self.idx = len(sseq._classes_by_bidegree[pos])
        sseq._classes_by_bidegree[pos].append(self)
        # self.node_list = [ n.idx for n in self.node_list ]

    def needs_update(self):
        self._sseq.add_class_to_update(self)

    @staticmethod
    def from_json(sseq, json):
        return ChartClass(sseq, **json)

    def __repr__(self):
        return f"ChartClass({self.x},{self.y})"

    def to_json(self):
        return utils.public_fields(self)

    def get_page_idx(self, page):
        for i, v in enumerate(self.transition_pages):
            if v > page:
                return i
        return len(self.transition_pages)

    def set_node_field_by_idx(self, idx, field, value):
        n = self.node_list[idx]
        setattr(n, field, value)

    @utils.sseq_property
    def name(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def visible(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def x_nudge(self, storage_name):
        self.needs_update()

    @utils.sseq_property
    def y_nudge(self, storage_name):
        self.needs_update()

    def set_color(self, value):
        self.set_field("color", value)

    def set_field(self, field, value):
        # with self._lock:
            for i in range(len(self.node_list)):
                if self.node_list[i]:
                    self.set_node_field_by_idx(i, field, value)
            self.needs_update()
            return self

    def set_field_on_page(self, page, field, value):
        # with self._lock:
            i = self.get_page_idx(page)
            self.set_node_field_by_idx(i, field, value)
            self.needs_update()
            return self

    def add_page(self, page, node=None):
        # if page in self.transition_pages:
            # return False
        # with self._lock:
        if page in self.transition_pages:
            return False            
        idx = self.get_page_idx(page)
        self.transition_pages.insert(idx, page)
        self.node_list.insert(idx+1, node)
        self.needs_update()
        return self

    def copy_previous_node(self, page):
        idx = self.get_page_idx(page)
        if idx == 0:
            raise ValueError("No previous node.")
        self.node_list[idx] = self.node_list[idx - 1].copy()
        self.needs_update()
        return self

    def replace(self, **kwargs):
        n = self.node_list[-2].copy()
        self.node_list[-1] = n
        for [key, value] in kwargs.items():
            setattr(n, key, value)
        self.needs_update()
        return self

    def delete(self):
        """ We make no attempt to communicate the change to the client right now.
            Better refresh the page after using this...
            Probably should also delete edges
        """ 
        self._sseq.add_class_to_delete(self)
        del self._sseq._classes[self.uuid]
        for e in self._edges:
            e.delete()

    def __repr__(self):
        name_str = ")"
        if self.name != "":
            name_str = f""", name="{self.name}")""" # put paren here to prevent four quotes in a row, better way?
        return f"""Class({self.x}, {self.y}, {self.idx}{name_str}"""

