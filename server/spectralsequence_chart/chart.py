
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
        
        self._batched_messages = []
        self._batched_messages_lock = threading.Lock()

    def to_json(self):
        return utils.public_fields(self)

    @staticmethod
    def from_json(chart, json):
        result = ChartData(chart, json["name"])
        utils.copy_fields_from_kwargs(result, json)

        result.nodes = []
        result.classes = {}
        result.edges = {}
        for node in json["nodes"]:
            result.nodes.append(ChartNode.from_json(result, node))

        for c in json["classes"].values():
            result.classes[c["uuid"]] = ChartClass.from_json(result, c)

        for e in json["edges"].values():
            result.edges[e["uuid"]] = ChartEdge.from_json(result, e)

        # We need to replace the uuids so that they are actually unique.
        for node in result.nodes:
            node.uuid = str(uuid4())

        for chart_class in list(result.classes.values()):
            del result.classes[chart_class.uuid]
            chart_class.uuid = str(uuid4())
            result.classes[chart_class.uuid] = chart_class

        for edge in list(result.edges.values()):
            del result.edges[edge.uuid]
            edge.uuid = str(uuid4())
            result.edges[edge.uuid] = edge
            edge.source = edge._source.uuid
            edge.target = edge._target.uuid
        
        return result
        

    def add_class(self, **kwargs):
        c = ChartClass(self, **kwargs)
        if "color" in kwargs:
            c.set_field("color", kwargs["color"])
        return c


    def add_differential(self, page, source, target, auto, **kwargs):
        e = ChartDifferential(self, page=page, source=source, target=target, **kwargs)
        self.edges[e.uuid] = e
        if auto:
            source.add_page(page)
            target.add_page(page)
            self.add_page_range([page,page])
        return e

    def add_structline(self, source, target, **kwargs):
        e = ChartStructline(self, source=source, target=target, **kwargs)
        self.edges[e.uuid] = e
        return e

    def add_extension(self, source, target, **kwargs):
        e = ChartExtension(self, source=source, target=target, **kwargs)
        self.edges[e.uuid] = e
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
        self.add_batched_message("chart.class.update", *arguments(
            class_to_update=c
        ))

    def add_edge_to_update(self, c):
        self.add_batched_message("chart.edge.update", *arguments(
            edge_to_update=c
        ))

    def add_batched_message(self, cmd, args, kwargs):
        cmd = Command().set_str(cmd)
        message = Message(cmd, args, kwargs)
        with self._batched_messages_lock:
            self._batched_messages.append(message)

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
    
    def load_json(self, json_obj):
        if type(json_obj) is str:
            json_obj = json.loads(json_obj)
        self.data = ChartData.from_json(self, json_obj)

    async def reset_state_a(self):
        await self.send_message_outward_a("chart.state.reset", *arguments(state = self.data))

    async def send_batched_messages_a(self):
        with self.data._batched_messages_lock:
            await self.send_message_outward_a("chart.batched", *arguments(
                messages = self.data._batched_messages
            ))
            self.data._batched_messages = []

    async def update_a(self):
        await self.send_batched_messages_a()

    @transform_inbound_messages
    async def consume_new_user_a(self, source_agent_path, cmd):
        await self.send_message_outward_a("initialize.chart.state", *arguments(
            state=self.data, display_state=self.display_state
        ))

    async def add_node_a(self, node : ChartNode):
        node.idx = len(self.data.nodes)
        self.data.nodes.append(node)
        await self.broadcast_a("chart.node.add", *arguments(node=node))

            
    # async def add_class_a(self, x : int, y : int, **kwargs):
    #     """ Add class immediate """
    #     kwargs.update({"x" : x, "y" : y, "node_list" : [0]})
    #     c = self.data.add_class(**kwargs)
    #     await self.broadcast_a("chart.class.add", *arguments(new_class=c))
    #     return c

    def add_class(self, x : int, y : int, **kwargs):
        """ Add class batched """
        kwargs.update({"x" : x, "y" : y, "node_list" : [0]})
        c = self.data.add_class(**kwargs)
        self.data.add_batched_message("chart.class.add", *arguments(new_class=c))
        return c

    async def set_class_name_a(self, x, y, idx, name):
        c = self.get_classes_in_bidegree(x, y)[idx]
        c.name = name
        self.data.add_element_to_update(c)


    def add_structline(self, source, target, **kwargs):
        e = self.data.add_structline(source, target, **kwargs)
        self.data.add_batched_message("chart.edge.add", *arguments(
            type = e.type,
            uuid = e.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        ))
        return e

    def add_extension(self, source, target, **kwargs):
        e = self.data.add_extension(source, target, **kwargs)
        self.data.add_batched_message("chart.edge.add", *arguments(
            type = e.type,
            uuid = e.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        ))
        return e


    def add_differential_main(self, page, source, target, auto=True, **kwargs):
        e = self.data.add_differential(page=page, source=source, target=target, auto=auto, **kwargs)
        self.data.add_page_range([page, page])
        # if auto:
        #     
        #     await self.update_a()
        return (e, "chart.edge.add", *arguments(
            page = page,
            type = e.type,
            uuid = e.uuid,
            source = source.uuid,
            target = target.uuid,
            **kwargs
        ))

    def add_differential(self, page, source, target, **kwargs):
        (e, *rest) = self.add_differential_main(page, source, target, **kwargs)
        self.data.add_batched_message(*rest)
        return e

    # async def add_differential_a(self, source, target, **kwargs):
    #     (e, *rest) = self.add_differential_main(source, target, **kwargs)
    #     await self.broadcast_a(*rest)
    #     return e

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
