
import asyncio
import json
from readerwriterlock import rwlock
import threading
from uuid import uuid4

from . import utils
from .chart_elements import *

from message_passing_tree.prelude import *
from message_passing_tree import Command, Message

INFINITY = 65535

class ChartData:
    def __init__(self, name):
        self._agent = None
        self.name = name
        self.initial_x_range = [0, 10]
        self.initial_y_range = [0, 10]
        self.x_range = [0, 10]
        self.y_range = [0, 10]

        self.page_list = [[2, INFINITY], [INFINITY, INFINITY]]
        self._page_list_lock = threading.Lock()
        self.min_page_idx = 0
        default_node = ChartNode(self, shape="circle")
        default_node.idx = 0
        self.nodes = [default_node]
        self._nodes_dict = {hash(default_node) : default_node}
        # self._nodes_lock = rwlock.RWLockFair()
        
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
        result["classes"] = self._classes
        result["edges"] = self._edges
        return result

    @staticmethod
    def from_json(json):
        result = ChartData(json["name"])
        utils.copy_fields_from_kwargs(result, json)

        result.nodes = []
        result._classes = {}
        result._edges = {}
        for node in json["nodes"]:
            result.nodes.append(ChartNode.from_json(result, node))

        for c in json["classes"].values():
            result._classes[c["uuid"]] = ChartClass.from_json(result, c)

        for e in json["edges"].values():
            result._edges[e["uuid"]] = ChartEdge.from_json(result, e)

        # We need to replace the uuids so that they are actually unique.
        # (should we do this?)
        for node in result.nodes:
            node.uuid = str(uuid4())

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
        
    def add_class(self, x, y, **kwargs):
        kwargs.update({"x" : x, "y" : y, "node_list" : [0]})
        c = ChartClass(self, **kwargs)
        if "color" in kwargs:
            c.set_field("color", kwargs["color"])
        return c

    def add_differential(self, page, source, target, auto = True, **kwargs):
        e = ChartDifferential(self, page=page, source=source, target=target, **kwargs)
        self._edges[e.uuid] = e
        if auto:
            source.add_page(page)
            target.add_page(page)
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
            self.add_batched_message("chart.insert_page_range", *arguments(page_range=page_range, idx=idx))
    

    # # TODO: Add a setting to turn off eager deduping.
    # # In that case, maybe dedup whenever someone calls get_state?
    # # Need to think about batch mode and stuff.
    # async def get_node_a(self, n : ChartNode) -> ChartNode:
    #     # if hash(n) in self._nodes_dict:
    #         # return self._nodes_dict[hash(n)]
    #     with self._nodes_lock.gen_rlock():
    #         if hash(n) in self._nodes_dict:
    #             return self._nodes_dict[hash(n)]
    #     with self._nodes_lock.gen_wlock():
    #         # Maybe someone else already put this node in before we got the lock.
    #         if hash(n) in self._nodes_dict: 
    #             return self._nodes_dict[hash(n)]
    #         else:
    #             self._nodes_dict[hash(n)] = n
    #             await self._agent.add_node_a(n)
    #             return n

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

class DisplayState:
    def __init__(self):
        self.background_color = ChartAgent.default_background_color

@subscribe_to(["*"])
@collect_handlers(inherit=False) # Nothing to inherit
class ChartAgent(Agent):
    default_agent=None
    default_background_color = "#FFFFFF"
    def __init__(self, name, sseq=None):
        super().__init__()
        self.sseq = None
        sseq = ChartData(name)
        self.set_sseq(sseq)
        self.display_state = DisplayState()

    def set_sseq(self, sseq):
        if self.sseq is not None:
            self.sseq._agent = None
        self.sseq = sseq
        self.sseq._agent = self

    def load_json(self, json_obj):
        if type(json_obj) is str:
            json_obj = json.loads(json_obj)
        self.set_sseq(ChartData.from_json(json_obj))

    async def reset_state_a(self):
        with self.sseq._batched_messages_lock:
            self.sseq._batched_messages = []
        await self.send_message_outward_a("chart.state.reset", *arguments(state = self.sseq))

    async def update_a(self):
        await self.sseq.update_a()

    async def send_batched_messages_a(self, messages):
        await self.send_message_outward_a("chart.batched", *arguments(
            messages = messages
        ))


    # async def add_node_a(self, node : ChartNode):
    #     node.idx = len(self.data.nodes)
    #     self.data.nodes.append(node)
    #     await self.broadcast_a("chart.node.add", *arguments(node=node))

    # async def set_background_color_a(self, color):
    #     self.data.background_color = color
    #     await self.broadcast_a("display.set_background_color", *arguments(color=color))
