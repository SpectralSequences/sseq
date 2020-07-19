import json
import threading
from uuid import uuid4

from . import utils
from .utils import arguments
from .chart_elements import *
from .page_property import PageProperty
from .messages import *

INFINITY = 65535


class SseqChart:
    def __init__(self, name):
        self._agent = None
        self.name = name
        self.initial_x_range = [0, 10]
        self.initial_y_range = [0, 10]
        self.x_range = [0, 10]
        self.y_range = [0, 10]

        self.num_gradings = 2
        self.x_degree = [1, 0]
        self.y_degree = [0, 1]

        self.page_list = [[2, INFINITY], [INFINITY, INFINITY]]
        self._page_list_lock = threading.Lock()
        self.min_page_idx = 0
        
        self._classes = {}
        self._classes_by_bidegree = {}
        
        self._edges = {}
        
        self._batched_messages = []
        self._objects_to_update = set()
        self._batched_messages_lock = threading.Lock()
        self._initialized = True

    @property
    def classes(self):
        return list(self._classes.values())

    @classes.setter
    def classes(self, value):
        if type(value) is not dict:
            raise TypeError(f"Value should be a dictionary not a {type(value).__name__}")
        self._classes = value

    @property
    def edges(self):
        return list(self._edges.values())

    @edges.setter
    def edges(self, value):
        if type(value) is not dict:
            raise TypeError(f"Value should be a dictionary not a {type(value).__name__}")
        self._edges = value

    def to_json(self):
        result = utils.public_fields(self)
        result["type"] = type(self).__name__
        result["classes"] = self._classes
        result["edges"] = self._edges
        return result

    @staticmethod
    def from_json(json_obj):
        result = SseqChart(json_obj["name"])
        utils.copy_fields_from_kwargs(result, json_obj)
        for c in json_obj["classes"].values():
            result._classes[c["uuid"]] = ChartClass.from_json(result, c)

        for e in json_obj["edges"].values():
            result._edges[e["uuid"]] = ChartEdge.from_json(result, e)

        for chart_class in result.classes:
            del result._classes[chart_class.uuid]
            chart_class.uuid = str(uuid4())
            result._classes[chart_class.uuid] = chart_class

        for edge in result.edges:
            del result._edges[edge.uuid]
            edge.uuid = str(uuid4())
            result._edges[edge.uuid] = edge
            edge.source = edge.source.uuid
            edge.target = edge.target.uuid
        
        return result
        
    def add_class(self, *degree, **kwargs):
        kwargs.update({"degree" : degree})
        c = ChartClass(self, **kwargs)
        if "color" in kwargs:
            c.set_field("color", kwargs["color"])
        return c

    def add_differential(self, page, source, target, auto = True, **kwargs):
        e = ChartDifferential(self, page=page, source=source, target=target, **kwargs)
        self._edges[e.uuid] = e
        if auto:
            source.max_page = page
            target.max_page = page
            self.add_page_range([page,page])
        return e

    def add_structline(self, source, target, **kwargs):
        e = ChartStructline(self, source=source, target=target, **kwargs)
        self._edges[e.uuid] = e
        return e

    def add_extension(self, source, target, **kwargs):
        e = ChartExtension(self, source=source, target=target, **kwargs)
        self._edges[e.uuid] = e
        return e

    def add_page_range(self, page_range):
        if page_range in self.page_list:
            return
        with self._page_list_lock:
            if page_range in self.page_list:
                return
            for (i, p) in enumerate(self.page_list):
                if p[0] > page_range[0]:
                    idx = i
                    break
            else:
                idx = len(self.page_list)
            self.page_list.insert(idx, page_range)
            self.add_batched_message(uuid4(), "chart.insert_page_range", *arguments(page_range=page_range, idx=idx))
    

    def add_class_to_update(self, c):
        self.add_batched_message(c.uuid, "chart.class.update", *arguments(
            class_to_update=c
        ))

    def add_class_to_delete(self, c):
        self.add_batched_message(c.uuid + ".delete", "chart.class.delete", *arguments(
            class_to_delete=c
        ))

    def add_edge_to_update(self, e):
        self.add_batched_message(e.uuid, "chart.edge.update", *arguments(
            edge_to_update=e
        ))

    def add_edge_to_delete(self, e):
        self.add_batched_message(e.uuid + ".delete", "chart.edge.delete", *arguments(
            edge_to_delete=e
        ))

    def add_batched_message(self, key, cmd, args, kwargs):
        if not hasattr(self, "_initialized"):
            return        
        if key in self._objects_to_update:
            return
        with self._batched_messages_lock:
            self.add_batched_message_raw(key, cmd, args, kwargs)

    def add_batched_message_raw(self, key, cmd, args, kwargs):
        if key in self._objects_to_update:
            return
        if key is not None:       
            self._objects_to_update.add(key)
        cmd = Command().set_str(cmd)
        message = Message(cmd, args, kwargs)
        self._batched_messages.append(message)

    async def update_a(self):
        with self._batched_messages_lock:
            if self._agent:
                await self._agent.send_batched_messages_a(self._batched_messages)
            self._batched_messages = []
            self._objects_to_update = set()
    
    def class_by_idx(self, x, y, idx):
        return self.classes_in_bidegree(x, y)[idx]

    def classes_in_bidegree(self, x, y):
        return self._classes_by_bidegree.get((x,y), [])

    @property
    def x_min(self):
        return self.x_range[0]

    @x_min.setter
    def x_min(self, value):
        self.add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        self.x_range[0] = value

    @property
    def x_max(self):
        return self.x_range[1]

    @x_max.setter
    def x_max(self, value):
        self.add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        self.x_range[1] = value

    @property
    def y_min(self):
        return self.y_range[0]
    
    @y_min.setter
    def y_min(self, value):
        self.add_batched_message("y_range", "chart.set_y_range", *arguments(y_range=self.y_range))
        self.y_range[0] = value

    @property
    def y_max(self):
        return self.y_range[1]

    @y_max.setter
    def y_max(self, value):
        self.add_batched_message("y_range", "chart.set_y_range", *arguments(y_range=self.y_range))
        self.y_range[1] = value

    @property
    def x_min_initial(self):
        return self.x_range_initial[0]

    @x_min.setter
    def x_min_initial(self, value):
        self.add_batched_message("initial_x_range", "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))
        self.x_range_initial[0] = value

    @property
    def x_max_initial(self):
        return self.x_range_initial[1]

    @x_max.setter
    def x_max_initial(self, value):
        self.add_batched_message("initial_x_range", "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))
        self.x_range_initial[1] = value

    @property
    def y_min_initial(self):
        return self.y_range_initial[0]
    
    @y_min.setter
    def y_min_initial(self, value):
        self.add_batched_message("initial_y_range", "chart.set_initial_y_range", *arguments(y_range=self.initial_y_range))
        self.y_range_initial[0] = value

    @property
    def y_max_initial(self):
        return self.y_range_initial[1]

    @y_max.setter
    def y_max_initial(self, value):
        self.add_batched_message("initial_y_range", "chart.set_initial_y_range", *arguments(y_range=self.y_range))
        self.y_range_initial[1] = value

    @utils.sseq_property
    def x_range(self, storage_name):
        pass
    
    @x_range.setter
    def x_range(self, storage_name, value):
        range_list = getattr(self, storage_name, [0, 0])
        range_list[0] = value[0]
        range_list[1] = value[1]
        setattr(self, storage_name, range_list)
        self.add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))

    @utils.sseq_property
    def y_range(self, storage_name):
        pass
    
    @y_range.setter
    def y_range(self, storage_name, value):
        range_list = getattr(self, storage_name, [0, 0])
        range_list[0] = value[0]
        range_list[1] = value[1]
        setattr(self, storage_name, range_list)
        self.add_batched_message("y_range", "chart.set_y_range", *arguments(y_range=self.y_range))


    @utils.sseq_property
    def x_range_initial(self, storage_name):
        pass
    
    @x_range_initial.setter
    def x_range_initial(self, storage_name, value):
        range_list = getattr(self, storage_name, [0, 0])
        range_list[0] = value[0]
        range_list[1] = value[1]
        setattr(self, storage_name, range_list)
        self.add_batched_message("x_range_initial", "chart.set_initial_x_range", *arguments(x_range=self.x_range))

    @utils.sseq_property
    def y_range_initial(self, storage_name):
        pass
    
    @y_range_initial.setter
    def y_range_initial(self, storage_name, value):
        range_list = getattr(self, storage_name, [0, 0])
        range_list[0] = value[0]
        range_list[1] = value[1]
        setattr(self, storage_name, range_list)
        self.add_batched_message("y_range_initial", "chart.set_initial_y_range", *arguments(y_range=self.y_range))