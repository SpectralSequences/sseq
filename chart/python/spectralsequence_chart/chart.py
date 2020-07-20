import threading
from typing import Tuple, List, Dict, Any, Set, Optional, Union
from uuid import uuid4

from . import utils
from .utils import arguments
from .chart_elements import (
    ChartClass, 
    ChartStructline, ChartDifferential, ChartExtension, ChartEdge
)
from .messages import *
from .infinity import INFINITY

ChartClassArg = Union[ChartClass, Tuple[int, ...], List[int]]

class SseqChart:
    def __init__(self, 
        name : str, 
        type : str = "SseqChart",
        initial_x_range : Tuple[int, int] = (0, 10),
        initial_y_range : Tuple[int, int] = (0, 10),
        x_range : Tuple[int, int] = (0, 10),
        y_range : Tuple[int, int] = (0, 10),
        num_gradings : int = 2,
        x_degree : Tuple[int, ...] = (1, 0),
        y_degree : Tuple[int, ...] = (0, 1),
        page_list : Optional[List[Tuple[int, int]]] = None,
        min_page_idx : int = 0,
        classes : Optional[Dict[str, ChartClass]] = None,
        edges : Optional[Dict[str, ChartEdge]] = None        
    ):
        assert type == self.__class__.__name__
        assert len(x_degree) == num_gradings
        assert len(y_degree) == num_gradings
        assert min_page_idx >= 0
        self._initialized = False
        self._agent : Any = None
        self.name = name
        self.initial_x_range = initial_x_range
        self.initial_y_range = initial_y_range
        self.x_range = x_range
        self.y_range = y_range

        self.num_gradings = num_gradings
        self.x_degree = x_degree
        self.y_degree = y_degree

        if page_list:
            self.page_list = page_list
        else:
            self.page_list : List[Tuple[int, int]] = [(2, INFINITY), (INFINITY, INFINITY)]
        self._page_list_lock = threading.Lock()
        self.min_page_idx = min_page_idx
        
        self._classes : Dict[str, ChartClass] = classes or {}
        self._classes_by_degree : Dict[Tuple[int, ...], List[ChartClass]] = {}
        
        self._edges : Dict[str, ChartEdge] = edges or {}
        
        self._batched_messages : List[Message] = []
        self._update_keys : Set[str] = set()
        self._batched_messages_lock = threading.Lock()
        self._initialized : bool = True

    @property
    def classes(self) -> List[ChartClass]:
        return list(self._classes.values())

    @property
    def edges(self) -> List[ChartEdge]:
        return list(self._edges.values())

    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            name=self.name,
            initial_x_range=self.initial_x_range,
            initial_y_range=self.initial_y_range,
            x_range=self.x_range,
            y_range=self.y_range,
            num_gradings=self.num_gradings,
            x_degree=self.x_degree,
            y_degree=self.y_degree,
            page_list=self.page_list,
            min_page_idx=self.min_page_idx,
            classes=self._classes,
            edges=self._edges
        )
        

    @staticmethod
    def from_json(json_obj : Dict[str, Any]) -> "SseqChart":
        result = SseqChart(**json_obj)
        for c in result.classes:
            result._commit_class(c)
        for e in result.edges:
            result._commit_edge(e)
        return result
        
    def add_class(self, *degree : int, **kwargs : Any) -> ChartClass:
        c = ChartClass(degree, **kwargs)
        self._commit_class(c)
        return c

    def _commit_class(self, c : ChartClass):
        if len(c.degree) != self.num_gradings:
            raise ValueError(f"Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}")

        c._sseq = self
        self._add_batched_message(c.uuid, "chart.class.add", *utils.arguments(new_class=self))
        self._classes[c.uuid] = c
        if c.degree not in self._classes_by_degree:
            self._classes_by_degree[c.degree] = []

        if c.idx is None:
            c.idx = len(self._classes_by_degree[c.degree])
        self._classes_by_degree[c.degree].append(c)

    def add_differential(self, 
        page : int, source_arg : ChartClassArg, target_arg : ChartClassArg, 
        auto : bool = True, **kwargs : Any
    ) -> ChartEdge:
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartDifferential(page=page, source_uuid=source.uuid, target_uuid=target.uuid, **kwargs)
        self._edges[e.uuid] = e
        if auto:
            source._max_page = page
            target._max_page = page
            self.add_page_range(page,page)
        self._commit_edge(e)        
        return e

    def add_structline(self, source_arg : ChartClassArg, target_arg : ChartClassArg,  **kwargs : Any) -> ChartStructline:
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartStructline(source_uuid=source.uuid, target_uuid=target.uuid, **kwargs)
        self._commit_edge(e)
        return e

    def add_extension(self, source_arg : ChartClassArg, target_arg : ChartClassArg, **kwargs : Any) -> ChartExtension:
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartExtension(source_uuid=source.uuid, target_uuid=target.uuid, **kwargs)
        self._commit_edge(e)
        return e
    
    def _commit_edge(self, e : ChartEdge):
        e._sseq = self
        self._edges[e.uuid] = e
        e.source = self._classes[e._source_uuid]
        e.target = self._classes[e._target_uuid]
        e.source._edges.append(e)
        e.target._edges.append(e)
        self._add_batched_message(e.uuid + ".new", "chart.edge.add", *utils.arguments(new_edge=e))

    def add_page_range(self, page_min : int, page_max : int):
        page_range = (page_min, page_max)
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
            self._add_batched_message(str(uuid4()), "chart.insert_page_range", *arguments(page_range=page_range, idx=idx))
    

    def _add_class_to_update(self, c : ChartClass):
        self._add_batched_message(c.uuid, "chart.class.update", *arguments(
            class_to_update=c
        ))

    def _add_class_to_delete(self, c : ChartClass):
        self._add_batched_message(c.uuid + ".delete", "chart.class.delete", *arguments(
            class_to_delete=c
        ))

    def _add_edge_to_update(self, e : ChartEdge):
        self._add_batched_message(e.uuid, "chart.edge.update", *arguments(
            edge_to_update=e
        ))

    def _add_edge_to_delete(self, e : ChartEdge):
        self._add_batched_message(e.uuid + ".delete", "chart.edge.delete", *arguments(
            edge_to_delete=e
        ))

    def _add_batched_message(self, key : str, cmd : str, args : Tuple, kwargs : Dict[str, Any]):
        if not self._initialized:
            return        
        if key in self._update_keys:
            return
        with self._batched_messages_lock:
            self._add_batched_message_raw(key, cmd, args, kwargs)

    def _add_batched_message_raw(self, key : str, cmd_str : str, args : Tuple, kwargs : Dict[str, Any]):
        if key in self._update_keys:
            return
        if key is not None:       
            self._update_keys.add(key)
        cmd = Command().set_str(cmd_str)
        message = Message(cmd, args, kwargs)
        self._batched_messages.append(message)

    async def update_a(self):
        with self._batched_messages_lock:
            if self._agent:
                await self._agent.send_batched_messages_a(self._batched_messages)
            self._batched_messages = []
            self._update_keys = set()
    
    def _normalize_class_argument(self, class_arg : ChartClassArg) -> ChartClass:
        if type(class_arg) is ChartClass:
            return class_arg
        return self.class_by_idx(*class_arg)

    def class_by_idx(self, *args : int) -> ChartClass:
        return self.classes_in_degree(*args[:-1])[args[-1]]

    def classes_in_degree(self, *args : int) -> List[ChartClass]:
        assert len(args) == self.num_gradings
        return self._classes_by_degree.get(args, [])

    @property
    def x_min(self):
        return self.x_range[0]

    @x_min.setter
    def x_min(self, value : int):
        self._add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        x_range = list(self.x_range)
        x_range[0] = value
        self.x_range = tuple(x_range)

    @property
    def x_max(self):
        return self.x_range[1]

    @x_max.setter
    def x_max(self, value : int):
        self._add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        x_range = list(self.x_range)
        x_range[1] = value
        self.x_range = tuple(x_range)

    @property
    def y_min(self):
        return self.y_range[0]
    
    @y_min.setter
    def y_min(self, value : int):
        y_range = list(self.y_range)
        y_range[0] = value
        self.y_range = tuple(y_range)
        self._add_batched_message(str(uuid4()), "chart.set_y_range", *arguments(y_range=self.y_range))

    @property
    def y_max(self):
        return self.y_range[1]

    @y_max.setter
    def y_max(self, value : int):
        y_range = list(self.y_range)
        y_range[1] = value
        self.y_range = tuple(y_range)
        self._add_batched_message(str(uuid4()), "chart.set_y_range", *arguments(y_range=self.y_range))


    @property
    def initial_x_min(self):
        return self.initial_x_range[0]


    @initial_x_min.setter
    def initial_x_min(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[0] = value
        self.initial_x_range = tuple(initial_x_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))

    @property
    def initial_x_max(self):
        return self.initial_x_range[1]

    @initial_x_max.setter
    def initial_x_max(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[1] = value
        self.initial_x_range = tuple(initial_x_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))

    @property
    def initial_y_min(self):
        return self.initial_y_range[0]


    @initial_y_min.setter
    def initial_y_min(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[0] = value
        self.initial_y_range = tuple(initial_y_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_y_range", *arguments(x_range=self.initial_y_range))

    @property
    def initial_y_max(self):
        return self.initial_y_range[1]

    @initial_y_max.setter
    def initial_y_max(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[1] = value
        self.initial_y_range = tuple(initial_y_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_y_range", *arguments(x_range=self.initial_y_range))