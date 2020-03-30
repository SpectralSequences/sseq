
import asyncio
from readerwriterlock import rwlock
import threading

from . import utils
from .chart_elements import ChartNode, ChartClass, ChartEdge

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
        self.page_list = [[2, INFINITY], [INFINITY, INFINITY]]
        self._page_list_lock = threading.Lock()
        self.initial_x_range = [0, 10]
        self.initial_y_range = [0, 10]
        default_node = ChartNode(self, shape="circle")
        default_node.idx = 0
        self.nodes = [default_node]
        self.classes = []
        self.edges = []
        self.min_page_idx = 0
        self._nodes_dict = {hash(default_node) : default_node}
        self._nodes_lock = rwlock.RWLockFair()
        self._classes_by_bidegree = {}

    def to_json(self):
        return utils.public_fields(self)
    
    # TODO: Add a setting to turn off eager deduping.
    # In that case, maybe dedup whenever someone calls get_state?
    # Need to think about batch mode and stuff.
    async def get_node(self, n : ChartNode) -> ChartNode:
        # if hash(n) in self._nodes_dict:
            # return self._nodes_dict[hash(n)]
        with self._nodes_lock.gen_rlock():
            if hash(n) in self._nodes_dict:
                return self._nodes_dict[hash(n)]
        with self._nodes_lock.gen_wlock():
            # Maybe someone else already put this node in before we got the lock.
            if hash(n) in self._nodes_dict: 
                return self._nodes_dict[hash(n)]
            else:
                self._nodes_dict[hash(n)] = n
                await self._agent.add_node(n)
                return n


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
    
    @transform_inbound_messages
    async def consume_new_user(self, source_agent_path, cmd):
        await self.send_message_outward("chart.state", *arguments(
            state=self.data, display_state=self.display_state
        ))

    async def add_node(self, node : ChartNode):
        node.idx = len(self.data.nodes)
        self.data.nodes.append(node)
        await self.broadcast("chart.node.add", *arguments(node=node))

    async def add_page_range(self, page_range):
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
            await self.broadcast("chart.insert_page_range", *arguments(page_range=page_range, idx=idx))
            

    async def add_class(self, x : int, y : int, **kwargs):
        kwargs.update({"x" : x, "y" : y, "node_list" : [0]})
        c = ChartClass(self.data, **kwargs)
        if "color" in kwargs:
            await c.set_field("color", kwargs["color"])
        c.id = len(self.data.classes)
        self.data.classes.append(c)
        pos = (c.x, c.y)
        if pos not in self.data._classes_by_bidegree:
            self.data._classes_by_bidegree[pos] = []
        self.data._classes_by_bidegree[pos].append(c)
        kwargs.update({"id" : c.id})
        await self.broadcast("chart.class.add", *arguments(new_class=c))
        return c

    async def update_classes(self, classes):
        await self.broadcast("chart.class.update", *arguments(to_update=classes))

    async def set_class_name(self, x, y, idx, name):
        cc = self.get_classes_in_bidegree(x, y)[idx]
        cc.name = name
        await self.broadcast("chart.class.set_name", *arguments( 
            x=x,
            y=y,
            idx=idx,
            name=name
        ))

    async def add_edge(self, edge_type, source, target, **kwargs):
        kwargs.update({"type" : edge_type, "source" : source, "target" : target})
        e = ChartEdge(self.data, edge_type, **kwargs)
        e.id = len(self.data.edges)
        self.data.edges.append(e)
        e.get_source()._edges.append(e)
        e.get_target()._edges.append(e)
        kwargs.update({"id" : e.id, "source" : source.id, "target" : target.id})
        await self.broadcast("chart.edge.add", *arguments(**kwargs))
        return e

    async def add_structline(self, source, target, **kwargs):
        await self.add_edge("structline",source, target, **kwargs)

    async def add_differential(self, page, source, target, auto=True, **kwargs):
        if auto:
            update_classes = []
            if await source.add_page(page):
                update_classes.append(source)
            if await target.add_page(page):
                update_classes.append(target)
            await self.update_classes(update_classes)
            await self.add_page_range([page, page])
        e = await self.add_edge("differential", source, target, page=page, **kwargs)
        e.page = page

    # async def add_differential_interactive():


    @transform_inbound_messages
    async def consume_click(self, source_agent_path, cmd, chart_class):
        self.click

    def get_class_by_idx(self, x, y, idx):
        return self.get_classes_in_bidegree(x, y)[idx]

    def get_classes_in_bidegree(self, x, y):
        return self.data._classes_by_bidegree.get((x,y), [])

    async def set_x_range(self, x_min, x_max):
        self.data.x_range = [x_min, x_max]
        await self.broadcast("chart.set_x_range", *arguments())

    async def set_y_range(self, y_min, y_max):
        self.data.y_range = [y_min, y_max]
        await self.broadcast("chart.set_y_range", *arguments())

    async def set_initial_x_range(self, x_min, x_max):
        self.data.initial_x_range = [x_min, x_max]        
        await self.broadcast("chart.set_initial_x_range", *arguments())

    async def set_initial_y_range(self, y_min, y_max):
        self.data.initial_y_range = [y_min, y_max]
        await self.broadcast("chart.set_initial_y_range", *arguments())

    async def set_background_color(self, color):
        self.data.background_color = color
        await self.broadcast("display.set_background_color", *arguments(color=color))

    def to_json(self):
        return utils.public_fields(self)

