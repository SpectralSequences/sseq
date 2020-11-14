import threading
from typing import (
    Any, Dict, Iterable, List, NewType, 
    Optional,  Set, Tuple, Union, TYPE_CHECKING
)
import typing
from uuid import uuid4

from . import utils
from .utils import arguments
from .chart_elements import (
    ChartClass, 
    ChartStructline, ChartDifferential, ChartExtension, ChartEdge
)
from .messages import *
from .infinity import INFINITY

if not TYPE_CHECKING:
    class NewType:
        def __init__(self, name, tp):
            self.__name__ = name
            self.__supertype__ = tp

        def __call__(self, x):
            return x

        def __repr__(self):
            return self.__name__

ChartClassArg = NewType("ChartClassArg", Union[ChartClass, Iterable[int]])

class SseqChart:
    """ SseqChart is the main class which holds the data structure representing the chart. """
    def __init__(self, 
        name : str, 
        num_gradings : int = 2,
    ):
        """      
            Args:
                name (str): the name of the chart.
                num_gradings (int, optional): how many gradings the chart should have. Defaults to 2. 
                    If there are more than two gradings, the chart will still be displayed in 2d.
                    By default, the display projects onto the first two coordinates. The projection
                    can be modified by updating the fields "x_projection" and "y_projection".

            The rest of the optional arguments are for deserialization, not intended for direct usage.
        """
        self.name = name
        assert num_gradings >= 2
        self.num_gradings = num_gradings
        self._initialized = True

        self._agent : Any = None
        self._batched_messages : List[Dict[str, Any]] = []
        # type: ignore
        self._update_keys : Set[str] = set()
        self._global_fields_to_update : Set[str] = set()
        self._batched_messages_lock = threading.Lock()

        self._uuid = str(uuid4())

        self._page_list =  [(2, INFINITY), (INFINITY, INFINITY)]
        self._initial_x_range = (0, 10)
        self._initial_y_range = (0, 10)
        self._x_range = (0, 10)
        self._y_range = (0, 10)
  
        self._page_list_lock = threading.Lock()
        self._classes : Dict[str, ChartClass] = {}
        self._edges : Dict[str, ChartEdge] = {}
        self._classes_by_degree : Dict[Tuple[int, ...], List[ChartClass]] = {}
        self.x_projection = (1, 0) + (0,) * (num_gradings - 2)
        self.y_projection = (0, 1) + (0,) * (num_gradings - 2)       
        self._initialized : bool = True

    def validate(self):
        if not self._initialized:
            raise RuntimeError("Uninitialized chart.")
        if self._agent and not hasattr(self._agent, "send_batched_messages_a"):
            raise TypeError(f"_agent is of type {type(self._agent).__name__} which has no attribute send_batched_messages_a")
        
        if not isinstance(self.num_gradings, int) and self.num_gradings >= 2:
            raise TypeError(f"num_gradings should be an integer greater than or equal to 2, instead is {self.num_gradings}.")

        if len(self.x_projection) != self.num_gradings:
            raise TypeError(f"x_projection has length {len(self.x_projection)} not equal to num_gradings {self.num_gradings}")

        if len(self.y_projection) != self.num_gradings:
            raise TypeError(f"y_projection has length {len(self.y_projection)} not equal to num_gradings {self.num_gradings}")
        
        # for e in self.page_list:
        #     if 

        for c in self.classes_iter:
            pass

        for e in self.edges_iter:
            pass


    def __repr__(self):
        fields = []
        fields.append(f'"{self.name}"')
        nc = len(self._classes)
        ne = len(self._edges)
        fields.append(f"classes=[{nc} class{'es' if nc != 1 else ''}]")
        fields.append(f"edges=[{ne} edge{'s' if ne != 1 else ''}]")
        return f"{type(self).__name__}({', '.join(fields)})"

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
            initial_x_range=self._initial_x_range,
            initial_y_range=self._initial_y_range,
            x_range=self._x_range,
            y_range=self._y_range,
            num_gradings=self.num_gradings,
            x_degree=self.x_projection,
            y_degree=self.y_projection,
            page_list=self.page_list,
            classes=list(self._classes.values()),
            edges=list(self._edges.values())
        )
        

    @staticmethod
    def from_json(json_obj : Dict[str, Any]) -> "SseqChart":
        result = SseqChart._from_json_helper(**json_obj)
        return result

    @staticmethod
    def _from_json_helper(
        name : str, 
        num_gradings : int,
        type : str,
        uuid : str,
        page_list : List[Tuple[int, int]],
        initial_x_range : Tuple[int, int],
        initial_y_range : Tuple[int, int],
        x_range : Tuple[int, int],
        y_range : Tuple[int, int],
        x_projection : Tuple[int, ...],
        y_projection : Tuple[int, ...],
        classes : List[ChartClass],
        edges : List[ChartEdge]
    )  -> "SseqChart":
        assert type == SseqChart.__name__
        chart = SseqChart(name, num_gradings)
        chart._uuid = uuid
        chart._page_list = page_list
        chart._initial_x_range = initial_x_range
        chart._initial_y_range = initial_y_range
        chart._x_range = x_range
        chart._y_range = y_range
        chart._x_projection = x_projection
        chart._y_projection = y_projection
        for c in classes:
            chart._commit_class(c)
        for e in edges:
            chart._commit_edge(e)
        return chart
        
        
    def add_class(self, *degree : int) -> ChartClass:
        """ Add a class to the spectral sequence. 
        
            Args:
                *degree (int): A list of integers of length self.num_gradings.

            Returns: The class added.
        """
        assert len(degree) == self.num_gradings
        idx = len(self._classes_by_degree.get(degree, []))
        c = ChartClass(degree, idx)
        self._commit_class(c)
        return c

    def _commit_class(self, c : ChartClass):
        """ Common logic between add_class and deserialization of classes."""
        if len(c.degree) != self.num_gradings:
            raise ValueError(f"Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}")

        c._sseq = self
        self._add_create_message(c)
        self._classes[c.uuid] = c
        if c.degree not in self._classes_by_degree:
            self._classes_by_degree[c.degree] = []
        self._classes_by_degree[c.degree].append(c)

    def add_differential(self, 
        page : int, source_arg : ChartClassArg, target_arg : ChartClassArg, 
        auto : bool = True
    ) -> ChartDifferential:
        """ Add a differential.

            Args:
                page (int): which page should the differential appear on.
                source_arg (ChartClassArg): The source class. Represented as either a ChartClass or a list of integers of 
                    length num_gradings or num_gradings + 1.
                target_arg (ChartClassArg): The target class, same format as source_arg.
                auto (bool, optional): If 'True', automatically set max_page of source and target to 'page'. 
                    If False, the edge will be added but no change will be made to the source or target classes. Defaults to 'True'.
            
            Returns: The added differential.
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
            page p, say "edge.visible[p:] = False".

            Args:
                source_arg (ChartClassArg): The source class. Represented as either a ChartClass or a list of integers of length num_gradings or num_gradings + 1.
                target_arg (ChartClassArg): The target class, same format as source_arg.

            Returns: The added structline.
        """
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartStructline(source_uuid=source.uuid, target_uuid=target.uuid)
        self._commit_edge(e)
        return e

    def add_extension(self, source_arg : ChartClassArg, target_arg : ChartClassArg) -> ChartExtension:
        """ Add an extension. The extension will only appear on page pairs (infinity, _).

            Args:
                source_arg (ChartClassArg): The source class. Represented as either a ChartClass or a list of integers of length num_gradings or num_gradings + 1.
                target_arg (ChartClassArg): The target class, same format as source_arg.

            Returns: The added extension.
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
        e.source.edges.append(e)
        e.target.edges.append(e)
        self._add_create_message(e)

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
            self._add_setting_message()
    

    def _add_class_to_update(self, c : ChartClass):
        self._add_update_message(c)

    def _add_class_to_delete(self, c : ChartClass):
        self._add_delete_message(c)


    def _add_edge_to_update(self, e : ChartEdge):
        self._add_update_message(e)


    def _add_edge_to_delete(self, e : ChartEdge):
        self._add_delete_message(e)

    def _add_batched_message(self, key : str, kwargs : Dict[str, Any]):
        if not self._initialized:
            return        
        if key in self._update_keys:
            return
        with self._batched_messages_lock:
            self._add_batched_message_raw(key, kwargs)

    def _add_batched_message_raw(self, key : str, kwargs : Dict[str, Any]):
        # If we're actually bothering with locking we need to check again to make sure
        # key is not in dict to make sure that it didn't get inserted before we got the lock.
        if key in self._update_keys:
            return
        if key is not None:       
            self._update_keys.add(key)
        self._batched_messages.append(kwargs)

    def _add_create_message(self, target_object : Any):
        self._add_batched_message(
            target_object.uuid,
            dict(
                chart_id=self.uuid,
                target_type=type(target_object).__name__,
                command="create",
                target=target_object
            )
        )

    def _add_update_message(self, target_object : Any):
        self._add_batched_message(
            target_object.uuid,
            dict(
                chart_id=self.uuid, # okay to merge update with earlier create.
                target_type=type(target_object).__name__,
                command="update",
                target_uuid=target_object.uuid,
                update_fields=target_object
            )
        )

    def _add_delete_message(self, target_object : Any):
        self._add_batched_message(
            target_object.uuid + "--delete", # Don't merge delete event with earlier update
            dict(
                chart_id=self.uuid,
                target_type=type(target_object).__name__,
                command="delete",
                target_uuid=target_object.uuid,
            )
        )

    def _add_setting_message(self):
        pass

    async def update_a(self):
        """ Update the display. This will send a message to the display instructing it about how to 
            "catch up with" the current state of the SseqChart in the Python runtime.
            
            Note that the SseqClass controls the content and format of the message sent, but not
            the means of delivering the message (the means of delivering the message is defined 
            by an external class that implements communication with the display).
        """
        with self._batched_messages_lock:
            if not self._batched_messages:
                return
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
        if not self.num_gradings <= len(class_arg2) <= self.num_gradings + 1:
            raise TypeError(f'Iterable class specifier argument argument must have length "num_gradings" = {self.num_gradings} or "num_gradings" + 1 = {self.num_gradings+1}')
        from itertools import chain
        if len(class_arg2) == self.num_gradings:
            class_arg2 = chain(class_arg2, (0,))
        return self.class_by_idx(*class_arg2)

    def class_by_idx(self, *args : int) -> ChartClass:
        """ Get a specific class in the given degree.

            Args:
                *args (int): A sequence of integers of length "num_gradings + 1".
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
        x_range = list(self.x_range)
        x_range[0] = value
        self._x_range = tuple(x_range)
        self._add_setting_message()

    @property
    def x_max(self):
        """ The maximum x view extent. This represents the maximum x value that is possible to look at with the display.
            The display will not zoom or scroll right of this value.
        """
        return self.x_range[1]

    @x_max.setter
    def x_max(self, value : int):
        x_range = list(self.x_range)
        x_range[1] = value
        self._x_range = tuple(x_range)
        self._add_setting_message()

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
        self._y_range = tuple(y_range)
        self._add_setting_message()

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
        self._y_range = tuple(y_range)
        self._add_setting_message()


    @property
    def initial_x_min(self):
        """ The initial x minimum. When the display is first loaded this will be the smallest, leftmost visible x value."""        
        return self.initial_x_range[0]


    @initial_x_min.setter
    def initial_x_min(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[0] = value
        self._initial_x_range = tuple(initial_x_range)
        self._add_setting_message()

    @property
    def initial_x_max(self):
        """ The initial x maximum. When the display is first loaded this will be the largest, rightmost visible x value."""        
        return self.initial_x_range[1]

    @initial_x_max.setter
    def initial_x_max(self, value : int):
        initial_x_range = list(self.initial_x_range)
        initial_x_range[1] = value
        self._initial_x_range = tuple(initial_x_range)
        self._add_setting_message()

    @property
    def initial_y_min(self):
        """ The initial y minimum. When the display is first loaded this will be the smallest, bottommost visible y value."""        
        return self.initial_y_range[0]


    @initial_y_min.setter
    def initial_y_min(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[0] = value
        self._initial_y_range = tuple(initial_y_range)
        self._add_setting_message()

    @property
    def initial_y_max(self):
        """ The initial y maximum. When the display is first loaded this will be the largest, topmost visible y value."""        
        return self.initial_y_range[1]

    @initial_y_max.setter
    def initial_y_max(self, value : int):
        initial_y_range = list(self.initial_y_range)
        initial_y_range[1] = value
        self._initial_y_range = tuple(initial_y_range)
        self._add_setting_message()

    @property
    def x_projection(self):
        """ The x projection for the spectral sequence. Each class c is displayed in x degree the dot product of c.degree and x_projection."""
        return self._x_projection
    
    @x_projection.setter
    def x_projection(self, value : Tuple[int]):
        assert len(value) == self.num_gradings
        self._x_projection = value

    @property
    def y_projection(self):
        """ The y projection for the spectral sequence. Each class c is displayed in y degree the dot product of c.degree and y_projection."""
        return self._y_projection
    
    @y_projection.setter
    def y_projection(self, value : Tuple[int]):
        assert len(value) == self.num_gradings
        self._y_projection = value

    @property
    def uuid(self):
        """ A unique id for the chart. For internal use. """
        return self._uuid

    @property
    def page_list(self):
        """ The page list for the spectral sequence. This is a list of tuple pairs (<page>, <max_differential_length>).
            When changing the display page forward or backwards the "display page" steps through each pair in the page list.
            On a given display page (<page>, <max_differential_length>), all of the classes and structlines will appear as if on page <page>,
            while differentials will appear if the length of the differential is between <page> and <max_differential_length> inclusive.
        """
        return self._page_list

    # @property
    # def initial_x_range(self):
    #     """ The initial x range when the display is first loaded. A tuple pair."""
    #     return self._initial_x_range

    # @property
    # def initial_y_range(self):
    #     """ The initial y range when the display is first loaded. A tuple pair."""
    #     return self._initial_y_range

    # @property
    # def x_range(self):
    #     """ The maximum x range. It is impossible to scroll the display horizontally beyond this extent."""
    #     return self._x_range

    # @property
    # def y_range(self):
    #     """ The maximum x range. It is impossible to scroll the display horizontally beyond this extent."""
    #     return self._y_range

