import threading
from typing import Tuple, List, Dict, Any, Set, Optional, Type, Union, Iterable
from uuid import uuid4

from . import utils
from .utils import arguments
from .chart_elements import (
    ChartClass, 
    ChartStructline, ChartDifferential, ChartExtension, ChartEdge
)
from .messages import *
from .infinity import INFINITY

ChartClassArg = Union[ChartClass, Iterable[int]]

class SseqChart:
    def __init__(self, 
        name : str, 
        num_gradings : int = 2,
        type : Optional[str] = None,
        initial_x_range : Tuple[int, int] = (0, 10),
        initial_y_range : Tuple[int, int] = (0, 10),
        x_range : Tuple[int, int] = (0, 10),
        y_range : Tuple[int, int] = (0, 10),
        x_projection : Optional[Tuple[int, ...]] = None,
        y_projection : Optional[Tuple[int, ...]] = None,
        page_list : Optional[List[Tuple[int, int]]] = None,
        min_page_idx : int = 0,
        classes : Optional[Dict[str, ChartClass]] = None,
        edges : Optional[Dict[str, ChartEdge]] = None        
    ):
        """ SseqChart is the main class which holds the data structure representing the chart.
            
            Arguments:
                "name" -- the name of the chart.
                "num_gradings" -- how many gradings the chart should have (default 2). 
                    If there are more than two gradings, the chart will still be displayed in 2d.
                    By default, the display projects onto the first two coordinates. The projection
                    can be modified by updating the fields "x_projection" and "y_projection".

            The rest of the optional arguments are for deserialization, not recommended 
            for direct usage.
        """
        if type:
            assert type == self.__class__.__name__
        assert min_page_idx >= 0
        self._initialized = False
        self._agent : Any = None
        self.name = name
        self.initial_x_range = initial_x_range
        self.initial_y_range = initial_y_range
        self.x_range = x_range
        self.y_range = y_range

        assert num_gradings >= 2
        self.num_gradings = num_gradings
        if x_projection:
            assert len(x_projection) == num_gradings
        else:
            x_projection = (1, 0) + (0,) * (num_gradings - 2)
        if y_projection:
            assert len(y_projection) == num_gradings 
        else:
            y_projection = (0, 1) + (0,) * (num_gradings - 2)       
        self.x_projection = x_projection
        self.y_projection = y_projection

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
        """ Get the list of all classes in the chart. This performs a copy. 
            This is the same as list(self.classes_iter()).
        """
        return list(self._classes.values())

    @property
    def classes_iter(self) -> Iterable[ChartClass]:
        """ Return an iterable for all the classes in the chart. 
            This performs no copy, will raise if chart.add_class() or class.delete() are called while iterating.
        """
        return self._classes.values()

    @property
    def edges(self) -> List[ChartEdge]:
        """ Get the list of all edges in the chart. This performs a copy. 
            This is the same as list(self.edges_iter()).
        """
        return list(self._edges.values())

    @property
    def edges_iter(self) -> Iterable[ChartClass]:
        """ Return an iterable for all the edges in the chart. 
            This performs no copy, will raise if chart.add_edge(), edge.delete(), or class.delete() are called while iterating.
        """        
        return self._classes.values()


    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            name=self.name,
            initial_x_range=self.initial_x_range,
            initial_y_range=self.initial_y_range,
            x_range=self.x_range,
            y_range=self.y_range,
            num_gradings=self.num_gradings,
            x_degree=self.x_projection,
            y_degree=self.y_projection,
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
        for [uuid, e] in result._edges.items():
            assert uuid == e.uuid

        for e in result.edges:
            result._commit_edge(e)
        return result
        
    def add_class(self, *degree : int) -> ChartClass:
        """ Add a class to the spectral sequence. """
        assert len(degree) == self.num_gradings
        c = ChartClass(degree)
        self._commit_class(c)
        return c

    def _commit_class(self, c : ChartClass):
        """ Common logic between add_class and deserialization of classes."""
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
        auto : bool = True
    ) -> ChartEdge:
        """ Add a differential.

            Arguments:
                page -- which page should the differential appear on.
                source_arg -- A class specifier: either a ChartClass or a list of integers of length num_gradings or num_gradings + 1.
                target_arg -- A class specifier, same format as source_arg.
                auto -- A boolean, default 'True'. If 'True', automatically hide source and target after page 'page',
                        use replace to modify. If False, the edge will be added but no change will be made to the source 
                        or target classes.
        """
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartDifferential(page=page, source_uuid=source.uuid, target_uuid=target.uuid)
        self._edges[e.uuid] = e
        if auto:
            source._max_page = page
            target._max_page = page
            self.add_page_range(page,page)
        self._commit_edge(e)        
        return e

    def add_structline(self, source_arg : ChartClassArg, target_arg : ChartClassArg) -> ChartStructline:
        """ Add a structline. By default will appear on all pages on which both the source and target of the edge appear.
            To adjust this behavior modify the page property edge.visible. For instance, if you want to set the edge to be invisible after
            page p, say edge.visible[p:] = False.

            Arguments:
                source_arg -- A class specifier: either a ChartClass or a list of integers of length num_gradings or num_gradings + 1.
                target_arg -- A class specifier, same format as source_arg.
        """
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartStructline(source_uuid=source.uuid, target_uuid=target.uuid)
        self._commit_edge(e)
        return e

    def add_extension(self, source_arg : ChartClassArg, target_arg : ChartClassArg) -> ChartExtension:
        """ Add an extension. The extension will only appear on page infinity.

            Arguments:
                source_arg -- A class specifier: either a ChartClass or a list of integers of length num_gradings or num_gradings + 1.
                target_arg -- A class specifier, same format as source_arg.
        """        
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartExtension(source_uuid=source.uuid, target_uuid=target.uuid)
        self._commit_edge(e)
        return e
    
    def _commit_edge(self, e : ChartEdge):
        """ Common logic between add_structline, add_differential, add_extension, and deserialization."""
        e._sseq = self
        self._edges[e.uuid] = e
        e.source = self._classes[e._source_uuid]
        e.target = self._classes[e._target_uuid]
        e.source._edges.append(e)
        e.target._edges.append(e)
        self._add_batched_message(e.uuid + ".new", "chart.edge.add", *utils.arguments(new_edge=e))

    def add_page_range(self, page_min : int, page_max : int):
        """ Add a range of pages to the list of page_views. This indicates to the display that when stepping 
            through the page views the new page range should be included.
        """
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
        """ Update the display. This will send a message to the display instructing it about how to 
            "catch up with" the current state of the SseqChart in the Python runtime.
            
            Note that the SseqClass controls the content and format of the message sent, but not
            the means of delivering the message (the means of delivering the message is defined 
            by an external class that implements communication with the display).
        """
        with self._batched_messages_lock:
            if self._agent:
                await self._agent.send_batched_messages_a(self._batched_messages)
            self._batched_messages = []
            self._update_keys = set()
    
    def _normalize_class_argument(self, class_arg : ChartClassArg) -> ChartClass:
        """ If the argument is of type ChartClass, return it unmodified.
            Otherwise, the argument must be an iterable of length either self.num_gradings + 1
            or self.num_gradings. If length is self.num_gradings + 1, will use most of the entries
            to indicate the grading and the last entry as index. If length is self.num_gradings,
            index is treated as zero.
        """
        if type(class_arg) is ChartClass:
            return class_arg
        if not isinstance(class_arg, Iterable):
            raise TypeError(f'Class specifier argument must either be of type "ChartClass" or an iterable of integers.')
        if isinstance(class_arg, (list, tuple)):
            class_arg2 = class_arg
        else:
            class_arg2 = list(class_arg)
        if self.num_gradings <= len(class_arg2) <= self.num_gradings + 1:
            raise TypeError(f'Iterable class specifier argument argument must have length "num_gradings" = {self.num_gradings} or "num_gradings" + 1 = {self.num_gradings+1}')
        from itertools import chain
        if len(class_arg2) == self.num_gradings:
            class_arg2 = chain(class_arg2, (0,))
        return self.class_by_idx(*class_arg2)

    def class_by_idx(self, *args : int) -> ChartClass:
        """ Get a specific class in the given degree.
            The arguments should be a sequence of integers of length "num_gradings + 1".
            The last argument is the index of the class returned, the rest of the arguments
            indicate the degree.
        """
        if len(args) != self.num_gradings + 1:
            raise TypeError(f'Argument to "class_by_index" must have length "num_gradings + 1" = {self.num_gradings+1}')
        return self.classes_in_degree(*args[:-1])[args[-1]]

    def classes_in_degree(self, *args : int) -> List[ChartClass]:
        """ Get the list of classes in a given degree.
            The arguments should be a sequence of integers of length "num_gradings".
        """
        if len(args) != self.num_gradings:
            raise TypeError(f'Argument to "classes_in_degree" must have length "num_gradings" = {self.num_gradings}')
        return self._classes_by_degree.get(args, [])

    @property
    def x_min(self):
        """ The minimum x view extent. This represents the minimum x value that is possible to look at with the display.
            The display will not zoom or scroll left of this value.
        """
        return self.x_range[0]

    @x_min.setter
    def x_min(self, value : int):
        self._add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        x_range = list(self.x_range)
        x_range[0] = value
        self.x_range = tuple(x_range)

    @property
    def x_max(self):
        """ The maximum x view extent. This represents the maximum x value that is possible to look at with the display.
            The display will not zoom or scroll right of this value.
        """
        return self.x_range[1]

    @x_max.setter
    def x_max(self, value : int):
        self._add_batched_message("x_range", "chart.set_x_range", *arguments(x_range=self.x_range))
        x_range = list(self.x_range)
        x_range[1] = value
        self.x_range = tuple(x_range)

    @property
    def y_min(self):
        """ The minimum y view extent. This represents the minimum y value that is possible to look at with the display.
            The display will not zoom or scroll below this value.
        """
        return self.y_range[0]
    
    @y_min.setter
    def y_min(self, value : int):
        y_range = list(self.y_range)
        y_range[0] = value
        self.y_range = tuple(y_range)
        self._add_batched_message(str(uuid4()), "chart.set_y_range", *arguments(y_range=self.y_range))

    @property
    def y_max(self):
        """ The maximum y view extent. This represents the maximum y value that is possible to look at with the display.
            The display will not zoom or scroll above this value.
        """
        return self.y_range[1]

    @y_max.setter
    def y_max(self, value : int):
        y_range = list(self.y_range)
        y_range[1] = value
        self.y_range = tuple(y_range)
        self._add_batched_message(str(uuid4()), "chart.set_y_range", *arguments(y_range=self.y_range))


    @property
    def initial_x_min(self):
        """ The initial x minimum. When the display is first loaded this will be the smallest, leftmost visible x value."""        
        return self.initial_x_range[0]


    @initial_x_min.setter
    def initial_x_min(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[0] = value
        self.initial_x_range = tuple(initial_x_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))

    @property
    def initial_x_max(self):
        """ The initial x maximum. When the display is first loaded this will be the largest, rightmost visible x value."""        
        return self.initial_x_range[1]

    @initial_x_max.setter
    def initial_x_max(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[1] = value
        self.initial_x_range = tuple(initial_x_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_x_range", *arguments(x_range=self.initial_x_range))

    @property
    def initial_y_min(self):
        """ The initial y minimum. When the display is first loaded this will be the smallest, bottommost visible y value."""        
        return self.initial_y_range[0]


    @initial_y_min.setter
    def initial_y_min(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[0] = value
        self.initial_y_range = tuple(initial_y_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_y_range", *arguments(x_range=self.initial_y_range))

    @property
    def initial_y_max(self):
        """ The initial y maximum. When the display is first loaded this will be the largest, topmost visible y value."""        
        return self.initial_y_range[1]

    @initial_y_max.setter
    def initial_y_max(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[1] = value
        self.initial_y_range = tuple(initial_y_range)
        self._add_batched_message(str(uuid4()), "chart.set_initial_y_range", *arguments(x_range=self.initial_y_range))