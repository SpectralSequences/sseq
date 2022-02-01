from .page_property import (PageProperty, PagePropertyOrValue, ensure_page_property)
from .signal_dict import SignalDict

from .infinity import INFINITY
from uuid import uuid4  
from .display_primitives import UUID_str, Color, Shape
# from .chart_edge import ChartEdge

from typing import TYPE_CHECKING, List, Any, Tuple, cast, Dict, Union, NewType, Iterable
if TYPE_CHECKING:
    from .chart import SseqChart
    from .chart_edge import ChartEdge
 

 
class ChartClassStyle:
    """ The data that determine the visual style of a class on a particular page. 
        We also include a field `ChartClassStyle.group_name` which is used as shorthand
        to represent the `ChartClassStyle` and as a signifier of the intended purpose of the style.
    """
    def __init__(self,
        group_name : str = "",
        shape : Union[Shape, str] = "stdcircle",
        background_color : Union[Color, str] = "black",
        border_color : Union[Color, str] = "black",  
        foreground_color : Union[Color, str] = "black",
        border_width : float = 2,
    ):
        self._group_name = group_name
        self._shape = shape
        self._background_color = background_color
        self._border_color = border_color
        self._foreground_color = foreground_color
        self._border_width = border_width

    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            group_name = self._group_name,
            shape = self._shape,
            background_color = self._background_color,
            border_color = self._border_color,
            border_width = self._border_width,
            foreground_color = self._foreground_color,
        )

    @classmethod
    def from_json(cls, json : Dict[str, Any]) -> "ChartClassStyle":
        assert json.pop("type") == cls.__name__
        return cls(**json)

    @property
    def group_name(self) -> str:
        """ The name of the "group" represented by the current glyph. This does not directly affect the rendered chart. 
            If :attr:`group_name` is present it is treated as if it uniquely identifies the `ChartClassStyle`.
            For instance, if you register the `ChartClassStyle` with `SseqChart.register_class_style`
            then methods such as `ChartClass.set_style` that accept a `ChartClassStyle` as an argument
            will treat the string :attr:`group_name` as an alias for the registered `ChartClassStyle`. 
        """        
        return self._group_name
    
    @group_name.setter
    def group_name(self, v : str):
        self._group_name = v

    @property
    def shape(self) -> Shape:
        """ Control how to draw the class. """
        return self._shape
    
    @shape.setter
    def shape(self, v : Shape): # type: ignore
        self._shape = v

    @property
    def background_color(self) -> Color:
        """ The `color` to use to draw the shape background. """
        return self._background_color
    
    @background_color.setter
    def background_color(self, v : Color): # type: ignore
        """ Sets the `Color` to use to draw the background component of the `Shape`."""
        self._background_color = v

    @property
    def border_color(self) -> Color:
        """ The `Color` to use to draw the shape border. """
        return self._border_color
    
    @border_color.setter
    def border_color(self, v : Color): # type: ignore
        self._border_color = v

    @property
    def foreground_color(self) -> Color:
        """ The `Color` to use to draw the foreground component of the `Shape`."""
        return self._foreground_color
    
    @foreground_color.setter
    def foreground_color(self, v : Color): # type: ignore
        self._foreground_color = v

    @property
    def border_width(self) -> float:
        """ The thickness to draw the border of the `Shape`. """
        return self._border_width
    
    @border_width.setter
    def border_width(self, v : float): # type: ignore
        self._border_width = v

    def __repr__(self):
        if self.group_name:
            return f"ClassStyle({self.group_name})"
        d = self.to_json()
        del d["type"]
        del d["group_name"]
        return f"ClassStyle({d})"


class ChartClass:
    """ A `ChartClass` is roughly intended to represent a summand of the the E2 page in a particular bidegree.
        The class may change its appearance from page to page, typically whenever some part of that summand is either in
        the image of a differential or supports a nontrivial differential. 
    """
    def __init__(self, degree : Tuple[int, ...], idx : int):
        """ Do not call `ChartClass` constructor directly, use instead `SseqChart.add_class`, or `JSON.parse`."""
        self._sseq : SseqChart = None
        self._degree = tuple(degree)
        self._idx = idx
        self._max_page = INFINITY
        self._edges : List["ChartEdge"] = []
        self._uuid = str(uuid4())

        # These values don't really matter, just need to initialize the PageProperties or set_style will raise.
        self.group_name = ""
        self.shape = Shape().circled(5)
        self.background_color = (0, 0, 0, 1)
        self.border_color = (0, 0, 0, 1)
        self.foreground_color = (0, 0, 0, 1)
        self.border_width = 2

        self.name = ""
        self.scale = 1
        self.visible = True
        self.x_nudge = 0
        self.y_nudge = 0 
        self._user_data = SignalDict({}, parent=self)

    def get_style(self, page : int) -> ChartClassStyle:
        """ Gets the display style of the class on the given page. This can be stored in `SseqChart.class_styles` 
            or applied to other classes with `ChartClass.set_style`.
        """
        result = ChartClassStyle(
            group_name=self.group_name[page],
            shape=self.shape[page],
            background_color=self.background_color[page],
            border_color=self.border_color[page],
            foreground_color=self.foreground_color[page],
            border_width=self.border_width[page]
        )
        if result.group_name in self._sseq.class_styles:
            if result != self._sseq.class_styles[result.group_name]:
                result.group_name = result.group_name + " (modified)"
        return result

    def set_style(self, style : Union[ChartClassStyle, str], page : Union[int, Tuple[int, int]] = None) -> "ChartClass":
        """ Sets the display style of the class. 
            
            Arguments:
                style (ChartClassStyle | str): The style to set. If ``style`` is a string,
                    then the appropriate style is looked up in the dictionary `SseqChart.chart_class_styles`.
                    Otherwise, we use the `ChartClassStyle` provided.
                
                page (int | Tuple[int, int]):
                    If argument ``page`` is omitted or ``None`` then the style is set on all pages.
                    If ``page`` is a single integer, then the stlye is set starting on that page and all later pages.
                    If ``page`` is a pair of integers, the style is set on that range of pages inclusive of the lower 
                    endpoint and exclusive of the upper endpoint.
        """ 
        if page is None:
            page = slice(None)
        if isinstance(page, (tuple, list)):
            page = slice(page[0], page[1])
        if type(style) is str:
            if style not in self._sseq.class_styles:
                raise ValueError(f'Unknown class style "{style}". Register a class style with this group name using SseqChart.register_class_style first.')
            style = self._sseq._class_styles[style]
        self.group_name[page] = style.group_name
        self.shape[page] = style.shape
        self.background_color[page] = style.background_color
        self.border_color[page] = style.border_color
        self.foreground_color[page] = style.foreground_color
        self.border_width[page] = style.border_width
        return self
    
    def _normalize_attributes(self):
        self.shape._callback()
        self.background_color._callback()
        self.border_color._callback()
        self.foreground_color._callback()

    def _needs_update(self):
        if self._sseq:
            self._sseq._add_class_to_update(self)

    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartClass":
        assert json.pop("type") == ChartClass.__name__
        degree = json.pop("degree")
        idx = json.pop("idx")
        c = ChartClass(degree, idx)
        c._from_json_helper(**json)
        return c

    def _from_json_helper(self,     
        uuid : UUID_str,
        name : PagePropertyOrValue[str],
        max_page : int,
        visible : PagePropertyOrValue[bool],
        x_nudge : PagePropertyOrValue[float],
        y_nudge : PagePropertyOrValue[float],
        scale : PagePropertyOrValue[float],
        shape : PagePropertyOrValue[Shape],
        background_color : PagePropertyOrValue[Color],
        border_color : PagePropertyOrValue[Color],
        border_width : PagePropertyOrValue[float],
        foreground_color : PagePropertyOrValue[Color],
        user_data : SignalDict[Any]
    ):
        self._uuid = uuid
        self._max_page = max_page
        # Type checker has difficulty with PagePropertyOrValue and the typing of ensure_page_property.
        self.name = name
        self.shape = shape
        self.background_color = background_color
        self.border_color = border_color
        self.border_width = border_width
        self.foreground_color = foreground_color
        self.scale = scale
        self.visible = visible
        self.x_nudge = x_nudge
        self.y_nudge = y_nudge

        self._user_data = user_data # type: ignore
        user_data.set_parent(self)

    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            degree=self.degree,
            idx=self.idx,
            uuid=self.uuid,
            name=self.name,
            max_page=self._max_page,
            shape = self._shape,
            background_color = self._background_color,
            border_color = self._border_color,
            border_width = self._border_width,
            foreground_color = self._foreground_color,
            scale=self.scale,
            visible=self.visible,
            x_nudge=self.x_nudge,
            y_nudge=self.y_nudge,
            user_data=self.user_data
        )

    def replace(self, style : Union[ChartClassStyle, str]) -> "ChartClass":
        """ If the class has ``ChartClass.max_page`` less than infinity, then set it to be a permanent cycle.
            For instance::

                c.replace(some_style)
            
            Is the same as::
            
                page = c.max_page + 1
                c.max_page = INFINITY
                c.set_style(some_style, page)
        """
        if self.max_page == INFINITY:
            raise ValueError("Class is already alive")
        page = self.max_page + 1
        self._max_page = INFINITY
        if style:
            self.set_style(style, page)
        self._needs_update()
        return self

    def delete(self):
        """ Deletes the class. Also deletes any edges incident to it."""
        self._sseq._add_class_to_delete(self)
        del self._sseq._classes[self.uuid]
        for e in self.edges:
            e.delete()

    def __repr__(self) -> str:
        fields = [repr(x) for x in (*self.degree, self.idx)]
        if self.name[0] != "":
            fields.append(f'name="{self.name[0]}"')
        return f"{type(self).__name__}({', '.join(fields)})"

    @property
    def idx(self) -> str:
        """ The index of the class. Used to distinguish between classes in same bidegree and determine placement. """
        return self._idx

    @property
    def uuid(self) -> str:
        """ A unique id for the class. For internal use. """
        return self._uuid

    @property
    def edges(self) -> List["ChartEdge"]:
        """The list of edges incident to the class. Includes structure lines, differentials, and extensions. Order is arbitrary."""
        return self._edges

    @property
    def degree(self) -> Tuple[int, ...]:
        """The multigrading of the class."""
        return self._degree

    @property
    def x(self) -> int:
        """ The coordinate on the x-axis that the class will be displayed in, calculated as the dot product of sseq.x_projection and class.degree
        """
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.x_projection))

    @property
    def y(self) -> int:
        """ The coordinate on the y-axis that the class will be displayed in, calculated as the dot product of sseq.y_projection and class.degree.
        """
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.y_projection))

    @property
    def max_page(self) -> int:
        """ The maximum page the class may appear on. Note that the `PageProperty` `class.visible` also
            affects whether the class appears on a certain page: the class appears if ``class.visible[page]``
            is ``True`` and $page \leq max_page$.
        """
        return self._max_page
    
    @max_page.setter
    def max_page(self, v : int):
        self._max_page = v
        self._needs_update()

    @property
    def name(self) -> PageProperty[str]:
        """ The name of the class. This appears in the tooltip in the display among other places. """
        return self._name

    @name.setter
    def name(self, v : PagePropertyOrValue[str]): # type: ignore
        self._name = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def group_name(self) -> PageProperty[str]:
        """ The name of the "group" represented by the current class style. This does not directly affect the rendered chart, 
            it is intended for clerical purposes.
        """
        return self._group_name
    

    @group_name.setter
    def group_name(self, v : PagePropertyOrValue[str]):
        self._group_name = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def shape(self) -> PageProperty[Shape]:
        """ Control how to draw the class. """
        return self._shape
    
    @shape.setter
    def shape(self, v : PagePropertyOrValue[Shape]): # type: ignore
        pp = ensure_page_property(v, parent=self)
        def callback():
            if self._sseq:
                pp.map_values_in_place(self._sseq.get_shape)
        callback()
        pp.set_callback(callback)
        self._shape = pp
        self._needs_update()

    @property
    def background_color(self) -> PageProperty[Color]:
        """ The `Color` to use to draw the background component of the `Shape`."""
        return self._background_color
    
    @background_color.setter
    def background_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        pp = ensure_page_property(v, parent=self)
        def callback():
            if self._sseq:
                pp.map_values_in_place(self._sseq.get_color)
        callback()
        pp.set_callback(callback)
        self._background_color = pp
        self._needs_update()

    @property
    def border_color(self) -> PageProperty[Color]:
        """ The `Color` to use to draw the border component of the `Shape`. """
        return self._border_color
    
    @border_color.setter
    def border_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        pp = ensure_page_property(v, parent=self)
        def callback():
            if self._sseq:
                pp.map_values_in_place(self._sseq.get_color)
        callback()
        pp.set_callback(callback)   
        self._border_color = pp
        self._needs_update()

    @property
    def foreground_color(self) -> PageProperty[Color]:
        """ The `Color` to use to draw the foreground component of the `Shape`."""
        return self._foreground_color
    
    @foreground_color.setter
    def foreground_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        pp = ensure_page_property(v, parent=self)
        def callback():
            if self._sseq:
                pp.map_values_in_place(self._sseq.get_color)
        callback()
        pp.set_callback(callback)
        self._foreground_color = pp
        self._needs_update()

    @property
    def border_width(self) -> PageProperty[float]:
        """ The thickness to draw the border of the `Shape`. """
        return self._border_width
    
    @border_width.setter
    def border_width(self, v : PagePropertyOrValue[float]): # type: ignore
        self._border_width = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def scale(self) -> PageProperty[float]:
        """ Scale the shape drawn on the screen. The apparent size of the shape depends on this scale factor,
            the intrinsic size of the `Shape`, the global chart scale factor, and the current zoom factor in the display.
        """
        return self._scale

    @scale.setter 
    def scale(self, v : PagePropertyOrValue[float]): # type: ignore
        self._scale = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def visible(self) -> PageProperty[bool]:
        """ Determine whether the class is visible on a certain page. Note that the field "max_page" also determines whether the class
            appears on a given page. The class appears if "class.visible[page] and sseq.page_range[0] <= class.max_page".
        """        
        return self._visible

    @visible.setter
    def visible(self, v : PagePropertyOrValue[bool]): # type: ignore
        self._visible = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def x_nudge(self) -> PageProperty[float]:
        """ The x displacement of the class from its default position. """
        return self._x_nudge

    @x_nudge.setter
    def x_nudge(self, v : PagePropertyOrValue[float]): # type: ignore
        self._x_nudge = ensure_page_property(v, parent=self)
        self._needs_update()


    @property
    def y_nudge(self) -> PageProperty[float]:
        """ The y displacement of the class from its default position. """
        return self._y_nudge

    @y_nudge.setter
    def y_nudge(self, v : PagePropertyOrValue[float]): # type: ignore
        self._y_nudge = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def user_data(self) -> SignalDict[Any]:
        """ Miscellaneous extra data that the user wants to add. All data added must be serializable. """
        return self._user_data


if TYPE_CHECKING:
    ChartClassArg = NewType("ChartClassArg", Union[ChartClass, Iterable[int]])
else:
    class ChartClassArg:
        """ This is a type name which refers to either a `ChartClass` or a tuple of ints.
            It is used as an input to various `SseqChart` methods.
            To specify a class as an argument, either pass a reference to a `ChartClass` 
            or a tuple which will be passed to `SseqChart.get_class`.
            For instance ``(0,0)`` and ``(0, 0, 0)`` both refer to the class of index 0 at position (0, 0).
            ``(0, 0, 1)`` refers tot the class of index 1.
        """
        pass