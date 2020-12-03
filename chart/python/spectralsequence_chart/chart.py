""" SseqChart is the main class which holds the data structure representing the chart."""

import asyncio

from spectralsequence_chart.page_property import PageProperty
from spectralsequence_chart.display_primitives import ArrowTip, Color, Shape
import spectralsequence_chart
from spectralsequence_chart.signal_dict import SignalList
import threading
from typing import (
    Any, Dict, Iterable, List, 
    Set, Tuple, Union
)
from uuid import uuid4

from .infinity import INFINITY
from .chart_class import (ChartClass, ChartClassArg, ChartClassStyle)
from .chart_edge import (ChartStructline, ChartDifferential, ChartExtension, ChartEdge, ChartEdgeStyle)


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
        """
        self.name = name
        assert num_gradings >= 2
        self.num_gradings = num_gradings

        self._agent : Any = None
        self._batched_messages : List[Dict[str, Any]] = []
        # type: ignore
        self._update_keys : Dict[str, int] = {}
        self._global_fields_to_update : Set[str] = set()
        self._batched_messages_lock = threading.Lock()

        self._uuid = str(uuid4())

        self._page_list =  SignalList([(2, INFINITY), (INFINITY, INFINITY)], callback=self._add_setting_message)
        self._initial_x_range = (0, 10)
        self._initial_y_range = (0, 10)
        self._x_range = (0, 10)
        self._y_range = (0, 10)
  
        self._default_class_style = ChartClassStyle()
        self._default_structline_style = ChartEdgeStyle()
        self._default_differential_style = ChartEdgeStyle(color="blue", end_tip=ArrowTip())
        self._default_extension_style = ChartEdgeStyle()
        self._class_styles : Dict[str, ChartClassStyle] = {}
        self._edge_styles : Dict[str, ChartEdgeStyle] = {}
        self._shapes : Dict[str, Shape] = {}
        self._colors : Dict[str, Color] = {}

        self.register_shape("stdcircle", Shape.circle(5))

        self._page_list_lock = threading.Lock()
        self._classes : Dict[str, ChartClass] = {}
        self._edges : Dict[str, ChartEdge] = {}
        self._classes_by_degree : Dict[Tuple[int, ...], List[ChartClass]] = {}
        self.x_projection = (1, 0) + (0,) * (num_gradings - 2)
        self.y_projection = (0, 1) + (0,) * (num_gradings - 2)       
        self._initialized : bool = True

    # def validate(self):
    #     if not self._initialized:
    #         raise RuntimeError("Uninitialized chart.")
    #     if self._agent and not hasattr(self._agent, "send_batched_messages_a"):
    #         raise TypeError(f"_agent is of type {type(self._agent).__name__} which has no attribute send_batched_messages_a")
        
    #     if not isinstance(self.num_gradings, int) and self.num_gradings >= 2:
    #         raise TypeError(f"num_gradings should be an integer greater than or equal to 2, instead is {self.num_gradings}.")

    #     if len(self.x_projection) != self.num_gradings:
    #         raise TypeError(f"x_projection has length {len(self.x_projection)} not equal to num_gradings {self.num_gradings}")

    #     if len(self.y_projection) != self.num_gradings:
    #         raise TypeError(f"y_projection has length {len(self.y_projection)} not equal to num_gradings {self.num_gradings}")
        
    #     for c in self.classes_iter:
    #         pass

    #     for e in self.edges_iter:
    #         pass


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
            This is the same as ``list(self.classes_iter())``.
        """
        return list(self._classes.values())

    @property
    def classes_iter(self) -> Iterable[ChartClass]:
        """ Return an iterable for all the classes in the chart. 
            This performs no copy, will raise an exception if `SseqChart.add_class` or `ChartClass.delete` are called while iterating.
        """
        return self._classes.values()

    @property
    def edges(self) -> List[ChartEdge]:
        """ Get the list of all edges in the chart. This performs a copy. 
            This is the same as ``list(self.edges_iter())``.
        """
        return list(self._edges.values())

    @property
    def edges_iter(self) -> Iterable[ChartClass]:
        """ Return an iterable for all the edges in the chart. 
            This performs no copy, will raise if `SseqChart.add_structline`, `SseqChart.add_differential`,
            `SseqChart.add_extension`, `ChartEdge.delete`, or `ChartClass.delete` are called while iterating.
        """        
        return self._classes.values()


    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            version=spectralsequence_chart.__version__,
            uuid=self.uuid,
            name=self.name,
            initial_x_range=self._initial_x_range,
            initial_y_range=self._initial_y_range,
            x_range=self._x_range,
            y_range=self._y_range,
            num_gradings=self.num_gradings,
            x_projection=self.x_projection,
            y_projection=self.y_projection,
            page_list=self.page_list,
            classes=list(self._classes.values()),
            edges=list(self._edges.values())
        )
        

    @staticmethod
    def from_json(json_obj : Dict[str, Any]) -> "SseqChart":
        chart = SseqChart(json_obj.pop("name"), json_obj.pop("num_gradings"))
        chart._from_json_helper(**json_obj)
        return chart

    def _from_json_helper(self,
        type : str,
        uuid : str,
        version : str,
        page_list : List[Tuple[int, int]],
        initial_x_range : Tuple[int, int],
        initial_y_range : Tuple[int, int],
        x_range : Tuple[int, int],
        y_range : Tuple[int, int],
        x_projection : Tuple[int, ...],
        y_projection : Tuple[int, ...],
        classes : List[ChartClass],
        edges : List[ChartEdge]
    ):
        assert type == SseqChart.__name__
        self._uuid = uuid
        self._page_list = page_list
        page_list.set_callback(self._add_setting_message)
        self._initial_x_range = initial_x_range
        self._initial_y_range = initial_y_range
        self._x_range = x_range
        self._y_range = y_range
        self._x_projection = x_projection
        self._y_projection = y_projection
        for c in classes:
            self._commit_class(c)
        for e in edges:
            self._commit_edge(e)
        
        
    def add_class(self, *degree : int) -> ChartClass:
        """ Add a `ChartClass` to the spectral sequence.

            Example: 
                ``chart.add_class(2, 3)`` 
                If you want to create a new class and set the style try: ``chart.add_class(2, 3).set_style("Z")``

            Args:
                *degree (int): A list of integers of length self.num_gradings.

            Returns: 
                The `ChartClass` added.
        """
        assert len(degree) == self.num_gradings
        idx = len(self._classes_by_degree.get(degree, []))
        c = ChartClass(degree, idx)
        c.set_style(self.default_class_style)
        self._commit_class(c)
        return c

    def _commit_class(self, c : ChartClass):
        """ Common logic between add_class and deserialization of classes."""
        if len(c.degree) != self.num_gradings:
            raise ValueError(f"Wrong number of gradings: degree {c.degree} has length {len(c.degree)} but num_gradings is {self.num_gradings}")

        c._sseq = self
        c._normalize_attributes()
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

                source_arg (ChartClassArg): The source class, specified by either a `ChartClass` or a tuple with the bidegree and index of the class.

                target_arg (ChartClassArg): The target class, specified by either a `ChartClass` or a tuple with the bidegree and index of the class.

                auto (bool, optional): If 'True', automatically set max_page of source and target to 'page'. 
                    If False, the edge will be added but no change will be made to the source or target classes. Defaults to 'True'.
            
            Returns: The added differential.
        """
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        if auto:
            # TODO: Does any sort of checking belong here?
            # if source._max_page < page:
            
            source._max_page = page
            target._max_page = page
            # self.add_page_range(page,page)
        e = ChartDifferential(page=page, source_uuid=source.uuid, target_uuid=target.uuid)
        e.set_style(self._default_differential_style)
        self._edges[e.uuid] = e
        self._commit_edge(e)        
        return e

    def add_structline(self, source_arg : ChartClassArg, target_arg : ChartClassArg) -> ChartStructline:
        """ Add a structline. By default will appear on all pages on which both the source and target of the edge appear.
            To adjust this behavior modify the page property edge.visible. For instance, if you want to set the edge to be invisible after
            page p, say ``edge.visible[p:] = False``.

            Args:
                source_arg (ChartClassArg): The source class, specified by either a `ChartClass` or by a tuple which is passed to `SseqChart.get_class` to get the class.
                target_arg (ChartClassArg): The target class, specified by either a `ChartClass` or by a tuple which is passed to `SseqChart.get_class` to get the class.

            Returns: The added structline.
        """
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartStructline(source_uuid=source.uuid, target_uuid=target.uuid)
        e.set_style(self._default_structline_style)
        self._commit_edge(e)
        return e

    def add_extension(self, source_arg : ChartClassArg, target_arg : ChartClassArg) -> ChartExtension:
        """ Add an extension. The extension will only appear on page pairs (infinity, _).

            Args:
                source_arg (ChartClassArg): The source class, specified by either a `ChartClass` or by a tuple which is passed to `SseqChart.get_class` to get the class.
                target_arg (ChartClassArg): The target class, specified by either a `ChartClass` or by a tuple which is passed to `SseqChart.get_class` to get the class.

            Returns: The added extension.
        """        
        source = self._normalize_class_argument(source_arg)
        target = self._normalize_class_argument(target_arg)
        e = ChartExtension(source_uuid=source.uuid, target_uuid=target.uuid)
        e.set_style(self._default_extension_style)
        self._commit_edge(e)
        return e
    
    def _commit_edge(self, e : ChartEdge):
        """ Common logic between add_structline, add_differential, add_extension, and deserialization."""
        e._sseq = self
        e._normalize_attributes()
        self._edges[e.uuid] = e
        e._source = self._classes[e._source_uuid]
        e._target = self._classes[e._target_uuid]
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
    

    def _add_class_to_update(self, c : ChartClass):
        self._add_update_message(c)

    def _add_class_to_delete(self, c : ChartClass):
        self._add_delete_message(c)

    def _add_edge_to_update(self, e : ChartEdge):
        self._add_update_message(e)

    def _add_edge_to_delete(self, e : ChartEdge):
        self._add_delete_message(e)

    def _add_batched_message(self, key : str, kwargs : Dict[str, Any], replace=False):
        """ If replace is False, then if key """
        if not self._initialized:
            return        
        if key in self._update_keys and not replace:
            return
        with self._batched_messages_lock:
            self._add_batched_message_raw(key, kwargs, replace)

    def _add_batched_message_raw(self, key : str, kwargs : Dict[str, Any], replace):
        # If we're actually bothering with locking we need to check again to make sure
        # key is not in dict to make sure that it didn't get inserted before we got the lock.
        if key in self._update_keys:
            if replace:
                self._batched_messages[self._update_keys[key]] = kwargs
            return
        if key is not None:       
            self._update_keys[key] = len(self._batched_messages)
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
        self._add_batched_message("settings", 
            dict(
                chart_id=self.uuid,
                target_type=type(self).__name__,
                command="update",
                target_fields=self.get_settings(),
            ),
            replace=True
        )

    def get_settings(self) -> Dict[str, any]:
        return dict(
            page_list=self._page_list,
            x_projection=self.x_projection,
            y_projection=self.y_projection,
            x_range=self._x_range,
            y_range=self._y_range,
            # default_class_style=self.default_class_style,
            # default_structline_style=self.default_structline_style,
            # default_differential_style=self.default_differential_style,
            # default_extension_style=self.default_extension_style,
        )

    def _clear_batched_messages(self):
            self._batched_messages = []
            self._update_keys = {}

    @property
    def display(self):
        return self._agent
            
    def update(self):
        """ If the chart is attached to a display, update the attached display. 
            This will send a message to the display instructing it about how to 
            "catch up with" the current state of the `SseqChart` in the Python runtime.
            This is a wrapper around `SseqChart.update_a`.
        """
        asyncio.get_event_loop().call_soon(self.update_a())

    async def update_a(self):
        """ If the chart is attached to a display, update the attached display. 
            This will send a message to the display instructing it about how to 
            "catch up with" the current state of the `SseqChart` in the Python runtime.
            This is an asynchronous method and must be called like ``await chart.update_a()``.
            See `SseqChart.update` for a convenient synchronous wrapper.
        """
        with self._batched_messages_lock:
            if not self._batched_messages:
                return
            if self._agent:
                await self._agent.send_batched_messages_a(self._batched_messages)
            self._clear_batched_messages()
    
    async def save_a(self, *args, **kwargs):
        if not self._agent or not hasattr(self._agent, "save_a"):
            raise RuntimeError("Unsupported operation")
        await self._agent.save_a(*args, **kwargs)

    async def save_as_a(self, *args, **kwargs):
        if not self._agent or not hasattr(self._agent, "save_as_a"):
            raise RuntimeError("Unsupported operation")
        await self._agent.save_as_a(*args, **kwargs)

    async def load_a(self, *args, **kwargs):
        if not self._agent or not hasattr(self._agent, "load_a"):
            raise RuntimeError("Unsupported operation")
        await self._agent.load_a(*args, **kwargs)

    
    def _normalize_class_argument(self, class_arg : ChartClassArg) -> ChartClass:
        """ If the argument is of type `ChartClass`, return it unmodified.
            Otherwise, the argument is passed to `SseqChart.get_class`.
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
            raise TypeError(f'Class specifier argument argument must have length "num_gradings" = {self.num_gradings} or "num_gradings" + 1 = {self.num_gradings+1}')
        return self.get_class(*class_arg2)

    def get_class(self, *args : int) -> ChartClass:
        """ Get a specific class in the given degree.

            Args:
                *args (int): A sequence of integers of length ``num_gradings + 1``.
                    The first ``num_gradings`` arguments indicate the polydegree of the class, the last argument indicates the index.
                    You may optionally leave off the index in which case it is assumed to be 0.
            
            Examples:            
                ``chart.get_class(0, 0)`` or ``chart.get_class(0, 0, 0)`` both get the class of index 0 in bidegree (0, 0). 
                ``chart.get_class(0,0,1)`` gets the class of index 1.

        """
        if not self.num_gradings <= len(args) <= self.num_gradings + 1:
            raise TypeError(f'Iterable class specifier argument argument must have length "num_gradings" = {self.num_gradings} or "num_gradings" + 1 = {self.num_gradings+1}')
        if len(args) == self.num_gradings + 1:
            index = args[-1]
            args = args[:-1]
        else:
            index = 0
        return self.classes_in_degree(*args)[index]

    def classes_in_degree(self, *args : int) -> List[ChartClass]:
        """ Get the list of classes in a given degree.
            The arguments should be a sequence of integers of length ``num_gradings``.
        """
        if len(args) != self.num_gradings:
            raise TypeError(f'Argument to "classes_in_degree" must have length "num_gradings" = {self.num_gradings}')
        return self._classes_by_degree.get(args, [])

    @property
    def default_class_style(self) -> ChartClassStyle:
        """
            The default style for all new classes. Changes to the default will not affect existing classes. 
        """
        return self._default_class_style

    @default_class_style.setter
    def default_class_style(self, value : ChartClassStyle):
        self._default_class_style = value
        self.chart_styles["default"] = value
        self.add_class()
        range(2)

    @property
    def default_structline_style(self) -> ChartEdgeStyle:
        """ The default style for all new structlines. Changes to the default will not affect existing structlines. """
        return self._default_structline_style

    @default_structline_style.setter
    def default_structline_style(self, value : ChartEdgeStyle):
        self._default_structline_style = value

    @property
    def default_differential_style(self) -> ChartEdgeStyle:
        """ The default style for all new differentials. Changes to the default will not affect existing differentials. """
        return self._default_differential_style

    @default_differential_style.setter
    def default_differential_style(self, value : ChartEdgeStyle):
        self._default_differential_style = value

    @property
    def default_extension_style(self) -> ChartEdgeStyle:
        """ The default style for all new extensions. Changes to the default will not affect existing extensions. """
        return self._default_extension_style

    @default_extension_style.setter
    def default_extension_style(self, value : ChartEdgeStyle):
        self._default_extension_style = value


    @property
    def x_min(self):
        """ The minimum x view extent. This represents the minimum x value that is possible to look at with the display.
            The display will not zoom or scroll left of this value.
        """
        return self._x_range[0]

    @x_min.setter
    def x_min(self, value : int):
        x_range = list(self._x_range)
        x_range[0] = value
        self._x_range = tuple(x_range)
        self._add_setting_message()

    @property
    def x_max(self):
        """ The maximum x view extent. This represents the maximum x value that is possible to look at with the display.
            The display will not zoom or scroll right of this value.
        """
        return self._x_range[1]

    @x_max.setter
    def x_max(self, value : int):
        x_range = list(self._x_range)
        x_range[1] = value
        self._x_range = tuple(x_range)
        self._add_setting_message()

    @property
    def y_min(self):
        """ The minimum y view extent. This represents the minimum y value that is possible to look at with the display.
            The display will not zoom or scroll below this value.
        """
        return self._y_range[0]
    
    @y_min.setter
    def y_min(self, value : int):
        y_range = list(self._y_range)
        y_range[0] = value
        self._y_range = tuple(y_range)
        self._add_setting_message()

    @property
    def y_max(self):
        """ The maximum y view extent. This represents the maximum y value that is possible to look at with the display.
            The display will not zoom or scroll above this value.
        """
        return self._y_range[1]

    @y_max.setter
    def y_max(self, value : int):
        y_range = list(self._y_range)
        y_range[1] = value
        self._y_range = tuple(y_range)
        self._add_setting_message()


    @property
    def initial_x_min(self):
        """ The initial x minimum. When the display is first loaded this will be the smallest, leftmost visible x value."""        
        return self._initial_x_range[0]


    @initial_x_min.setter
    def initial_x_min(self, value : int):
        initial_x_range = list(self._initial_x_range)
        initial_x_range[0] = value
        self._initial_x_range = tuple(initial_x_range)
        self._add_setting_message()

    @property
    def initial_x_max(self):
        """ The initial x maximum. When the display is first loaded this will be the largest, rightmost visible x value."""        
        return self._initial_x_range[1]

    @initial_x_max.setter
    def initial_x_max(self, value : int):
        initial_x_range = list(self._initial_x_range)
        initial_x_range[1] = value
        self._initial_x_range = tuple(initial_x_range)
        self._add_setting_message()

    @property
    def initial_y_min(self):
        """ The initial y minimum. When the display is first loaded this will be the smallest, bottommost visible y value."""        
        return self._initial_y_range[0]


    @initial_y_min.setter
    def initial_y_min(self, value : int):
        initial_y_range = list(self._initial_y_range)
        initial_y_range[0] = value
        self._initial_y_range = tuple(initial_y_range)
        self._add_setting_message()

    @property
    def initial_y_max(self):
        """ The initial y maximum. When the display is first loaded this will be the largest, topmost visible y value."""        
        return self._initial_y_range[1]

    @initial_y_max.setter
    def initial_y_max(self, value : int):
        initial_y_range = list(self._initial_y_range)
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
        """ The page list for the spectral sequence. This is a list of tuple pairs ``(page, max_differential_length)``.
            When changing the display page forward or backwards the "display page" steps through each pair in the page list.
            On a given display page ``(page, max_differential_length)``, all of the classes and structlines will appear as if on page ``page``,
            while differentials will appear if the length of the differential is between ``page`` and ``max_differential_length`` inclusive.
        """
        return self._page_list

    @page_list.setter
    def page_list(self, v : List[Tuple[int, int]]):
        self._page_list = SignalList(v, callback=self._add_setting_message)
        self._add_setting_message()


    def register_class_style(self, class_style : ChartClassStyle):
        """ Register class style. This uses `class_style.group_name <ChartClassStyle.group_name>` as an index.
            Once registered, `class_style.group_name <ChartClassStyle.group_name>` may be used as a style in the arguments of 
            to `ChartClass.set_style`, `ChartClass.replace`, `ChartEdge.replace_source` and `ChartEdge.replace_target`.

            Example::

                style = ChartClassStyle(shape = Shape().boxed(10), group_name="Z")
                chart.register_class_style(style)
                chart.add_class(0, 0).set_style("Z")

            Args:
                class_style (ChartClassStyle): The class style to register.
        """    
        if not class_style.group_name:
            raise ValueError("register_class_style called on class_style with empty group_name.")
        if class_style.group_name in self._class_styles:
            if class_style.to_json() == self._class_styles[class_style.group_name].to_json():
                return
            raise ValueError(f'A different class_style with group_name "{class_style.group_name}" is already registered.')
        from copy import deepcopy
        self._class_styles[class_style.group_name] = deepcopy(class_style)

    def register_edge_style(self, edge_style : ChartEdgeStyle):
        """ Register edge style. This uses `edge_style.action <ChartEdgeStyle.action>` as an index.
            Once registered, `edge_style.action <ChartEdgeStyle.action>` may be used as a style in the arguments of
            `ChartEdge.set_style`.

            Args:
                edge_style (ChartEdgeStyle): The edge style to register.
        """    
        if not edge_style.action:
            raise ValueError("register_class_style called on class_style with empty action.")
        if edge_style.action in self._edge_styles:
            if edge_style.to_json() == self._edge_styles[edge_style.action].to_json():
                return
            raise ValueError(f'A different class_style with action "{edge_style.action}" is already registered.')
        from copy import deepcopy
        self._edge_styles[edge_style.action] = deepcopy(edge_style)

    def register_shape(self, name : str, shape : Shape):
        shape._name = name
        self._shapes[name] = shape

    def register_color(self, name : str, color : Color):
        color._name = name
        self._colors[name] = color

    @property
    def class_styles(self) -> Dict[str, ChartClassStyle]:
        """ A dictionary of `ChartClassStyles <ChartClassStyle>`. `SseqChart.register_class_style` adds styles to this.
            You can use this to unregister class styles, etc.
            If you pass a string argument to `ChartClass.set_style`, it will look up the style in this dictionary. 
            
            Keys for this dictionary may be used as arguments for `ChartClass.set_style`, `ChartClass.replace`, 
            `ChartEdge.replace_source` and `ChartEdge.replace_target`.
        """
        return self._class_styles

    @class_styles.setter
    def class_styles(self, v : Dict[str, ChartClassStyle]):
        self._class_styles = v
        self._class_styles["default"] = self._default_class_style


    @property
    def edge_styles(self) -> Dict[str, ChartEdgeStyle]:
        """ A dictionary of `ChartEdgeStyles <ChartEdgeStyle>`. `SseqChart.register_edge_style` adds styles to this dictionary. 
            You can use this to unregister edge styles, etc.
            If you pass a string argument to `ChartEdge.set_style`, it will look up the style in this dictionary. 
            
            Keys for this dictionary may be used as arguments for `ChartEdge.set_style`.
        """
        return self._edge_styles 

    @edge_styles.setter
    def edge_styles(self, v : Dict[str, ChartEdgeStyle]):
        self._edge_styles = v


    @property
    def shapes(self) -> Dict[str, Shape]:
        """ A dictionary of `Shapes <Shape>`. `SseqChart.register_shape` adds shapes to this dictionary. 
            You can use this to unregister shapes, etc.
            If you set the shape of a class to a string, the actual `Shape` will be looked up in this dictionary.            
        """
        return self._shapes 

    @shapes.setter
    def shapes(self, v : Dict[str, Shape]):
        self._shapes = v

    @property
    def colors(self) -> Dict[str, Color]:
        """ A dictionary of `Colors <Color>`. `SseqChart.register_color` adds colors to this dictionary. 
            You can use this to unregister shapes, etc.
            If you set the color of a class or edge to a string, the actual `Color` will be looked up in this dictionary.
        """
        return self._colors 

    @colors.setter
    def colors(self, v : Dict[str, Color]):
        self._colors = v

    def get_shape(self, shape : Union[str, Shape]) -> Shape:
        if type(shape) is Shape:
            return shape
        if shape in self.shapes:
            return self.shapes[shape]
        raise ValueError(f"Unrecognized shape '{shape}'")

    def get_color(self, color : Union[str, Color]) -> Color:
        if type(color) is Color:
            return color
        if type(color) is not str:
            raise TypeError(f"Expected argument to be of type 'Color' or 'str' not '{type(color)}'")
        if color in self.colors:
            return self.colors[color]
        return Color.from_string(color)

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

