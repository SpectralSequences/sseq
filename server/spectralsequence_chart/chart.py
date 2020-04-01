
import asyncio
import json
from readerwriterlock import rwlock
import threading

from . import utils
from .chart_elements import *



from message_passing_tree import Agent
from message_passing_tree.decorators import (
    collect_transforms, 
    subscribe_to, 
    transform_inbound_messages
)
from message_passing_tree.utils import arguments

INFINITY = 65535

class ChartData:
    def __init__(self, agent, name):
        self._agent = agent
        self.name = name
        self.initial_x_range = [0, 10]
        self.initial_y_range = [0, 10]
        
        self.page_list = [[2, INFINITY], [INFINITY, INFINITY]]
        self._page_list_lock = threading.Lock()
        self.min_page_idx = 0
        default_node = ChartNode(self, shape="circle")
        default_node.idx = 0
        self.nodes = [default_node]
        self._nodes_dict = {hash(default_node) : default_node}
        self._nodes_lock = rwlock.RWLockFair()
        
        self.classes = {}
        self._classes_by_bidegree = {}
        
        self.edges = {}
        
        self._updated_elements = set()
        self._updated_elements_lock = threading.Lock()

    def to_json(self):
        return utils.public_fields(self)

    @staticmethod
    def from_json(agent, json):
        result = ChartData(agent)
        result.name = json["name"]
        result.initial_x_range = json["initial_x_range"]
        result.initial_x_range = json["initial_x_range"]
        result.x_range = json["x_range"]
        result.y_range = json["y_range"]
        result.min_page_idx = json["min_page_idx"]
        for n in json["nodes"]:
            result.nodes.append(ChartNode.from_json(self, json))
        for c in json["classes"]:
            result.classes[c["uuid"]].append(
                ChartClass.from_json(self, c)
            )
        

    def add_class(self, **kwargs):
        c = ChartClass(self, **kwargs)
        if "color" in kwargs:
            c.set_field("color", kwargs["color"])
        self.classes[c.uuid] = c
        pos = (c.x, c.y)
        if pos not in self._classes_by_bidegree:
            self._classes_by_bidegree[pos] = []
        self._classes_by_bidegree[pos].append(c)
        return c

    def add_structline(self, source, target, **kwargs):
        e = ChartStructline(self, source=source, target=target, **kwargs)
        self.add_edge_common_code(e)
        return e

    def add_extension(self, source, target, **kwargs):
        e = ChartExtension(self, source=source, target=target, **kwargs)
        self.add_edge_common_code(e)
        return e


    def add_differential(self, page, source, target, auto, **kwargs):
        e = ChartDifferential(self, page=page, source=source, target=target, **kwargs)
        self.add_edge_common_code(e)
        if auto:
            source.add_page(page)
            target.add_page(page)
        return e

    def add_edge_common_code(self, e):
        source = e.get_source()
        target = e.get_target()
        self.edges[e.uuid] = e
        source._edges.append(e)
        target._edges.append(e)

    

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

    def add_element_to_update(self, e):
        with self._updated_elements_lock:
            self._updated_elements.add(e)


class DisplayState:
    def __init__(self):
        self.background_color = SpectralSequenceChart.default_background_color

@subscribe_to(["chart", "display"])
@collect_transforms(inherit=False) # Nothing to inherit
class SpectralSequenceChart(Agent):
    default_agent=None
    default_background_color = "#FFFFFF"
    def __init__(self, name, sseq=None):
        super().__init__()
        self.data = ChartData(self, name)
        self.display_state = DisplayState()
        self._click = asyncio.Event()

    def get_state(self):        
        return self.data
    
    def load_json(self, json):
        self.data = ChartData.from_json(self, json)

    async def broadcast_a(self, cmd, args, kwargs):
        await self.send_message_outward_a(cmd, args, kwargs)

    @transform_inbound_messages
    async def consume_new_user_a(self, source_agent_path, cmd):
        await self.send_message_outward_a("initialize.chart.state", *arguments(
            state=self.data, display_state=self.display_state
        ))

    async def add_node_a(self, node : ChartNode):
        node.idx = len(self.data.nodes)
        self.data.nodes.append(node)
        await self.broadcast_a("chart.node.add", *arguments(node=node))

    async def add_page_range_a(self, page_range):
        if page_range in self.data.page_list:
            return
        with self.data._page_list_lock:
            if page_range in self.data.page_list:
                return
            for (i, p) in enumerate(self.data.page_list):
                if p[0] > page_range[0]:
                    idx = i
                    break
            else:
                idx = len(self.data.page_list)
            self.data.page_list.insert(idx, page_range)
            await self.broadcast_a("chart.insert_page_range", *arguments(page_range=page_range, idx=idx))
            

    async def add_class_a(self, x : int, y : int, **kwargs):
        kwargs.update({"x" : x, "y" : y, "node_list" : [0]})
        c = self.data.add_class(**kwargs)
        kwargs.update({"uuid" : c.uuid})
        await self.broadcast_a("chart.class.add", *arguments(new_class=c))
        return c

    async def update_a(self):
        with self.data._updated_elements_lock:
            await self.broadcast_a("chart.class.update", *arguments(
                to_update=list(self.data._updated_elements)
            ))
            self.data._updated_elements = set()

    async def set_class_name_a(self, x, y, idx, name):
        c = self.get_classes_in_bidegree(x, y)[idx]
        c.name = name
        self.data.add_element_to_update(c)


    async def add_structline_a(self, source, target, **kwargs):
        e = self.data.add_structline(source, target, **kwargs)
        await self.broadcast_a("chart.edge.add", *arguments(
            type = e.type,
            uuid = e.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        ))
        return e

    async def add_differential_a(self, page, source, target, auto=True, **kwargs):
        e = self.data.add_differential(page=page, source=source, target=target, auto=auto, **kwargs)
        if auto:
            await self.add_page_range_a([page, page])
            await self.update_a()
        await self.broadcast_a("chart.edge.add", *arguments(
            page = page,
            type = e.type,
            uuid = e.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        ))
        return e

    @transform_inbound_messages
    async def consume_click_a(self, source_agent_path, cmd, chart_class):
        pass
        # self.click

    def get_class_by_idx(self, x, y, idx):
        return self.get_classes_in_bidegree(x, y)[idx]

    def get_classes_in_bidegree(self, x, y):
        return self.data._classes_by_bidegree.get((x,y), [])

    @property
    def x_min(self):
        return self.data.x_range[0]

    @property
    def x_max(self):
        return self.data.x_range[1]

    @property
    def y_min(self):
        return self.data.y_range[0]
    
    @property
    def y_max(self):
        return self.data.y_range[1]

    async def set_x_range_a(self, x_min, x_max):
        self.data.x_range = [x_min, x_max]
        await self.broadcast_a("chart.set_x_range", *arguments(x_min=x_min, x_max=x_max))

    async def set_y_range_a(self, y_min, y_max):
        self.data.y_range = [y_min, y_max]
        await self.broadcast_a("chart.set_y_range", *arguments(y_min=y_min, y_max=y_max))

    async def set_initial_x_range_a(self, x_min, x_max):
        self.data.initial_x_range = [x_min, x_max]        
        await self.broadcast_a("chart.set_initial_x_range", *arguments(x_min=x_min, x_max=x_max))

    async def set_initial_y_range_a(self, y_min, y_max):
        self.data.initial_y_range = [y_min, y_max]
        await self.broadcast_a("chart.set_initial_y_range", *arguments(y_min=y_min, y_max=y_max))

    async def set_background_color_a(self, color):
        self.data.background_color = color
        await self.broadcast_a("display.set_background_color", *arguments(color=color))

    def to_json(self):
        return utils.public_fields(self)

