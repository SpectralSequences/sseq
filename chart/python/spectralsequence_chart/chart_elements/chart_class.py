from ..helper_classes import (
    PageProperty, PagePropertyOrValue, ensure_page_property, 
    SignalDict
)

from ..infinity import INFINITY
from uuid import uuid4
from .chart_types import UUID_str, Color
from .chart_shape import Shape
from .chart_edge import ChartEdge

from typing import TYPE_CHECKING, List, Any, Tuple, cast, Dict, Union, Optional
if TYPE_CHECKING:
    from ..chart import SseqChart
    from .chart_edge import ChartEdge



class ChartClass:
    """ """
    def __init__(self, degree : Tuple[int, ...], idx : int):
        """ Do not call SseqClass constructor directly, use instead SseqChart.add_class(), or JSON.parse()."""
        self._sseq : SseqChart
        self._degree = tuple(degree)
        self._idx = idx
        self._max_page = INFINITY
        self._edges : List[ChartEdge] = []
        self._uuid = str(uuid4())

        self._name = PageProperty("", parent=self)
        self._shape = PageProperty(Shape().circled(5), parent=self)
        self._background_color = PageProperty((0, 0, 0, 1), parent=self)
        self._border_color = PageProperty((0, 0, 0, 1), parent=self)
        self._foreground_color = PageProperty((0, 0, 0, 1), parent=self)
        self._border_thickness = PageProperty(3, parent=self)
        self._scale = PageProperty(1, parent=self)
        self._visible = PageProperty(True, parent=self)
        self._x_nudge = PageProperty(0, parent=self)
        self._y_nudge = PageProperty(0, parent=self)
        self._dom_content = SignalDict({}, parent=self)
        self._user_data = SignalDict({}, parent=self)

    def _needs_update(self):
        self._sseq._add_class_to_update(self)

    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartClass":
        assert json.pop("type") == ChartClass.__name__
        degree = json.pop("degree")
        idx = json.pop("idx")
        c = ChartClass(degree, idx)
        return c._from_json_helper(**json)

    @staticmethod
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
        border_thickness : PagePropertyOrValue[float],
        foreground_color : PagePropertyOrValue[Color],
        dom_content : Dict[str, Union[str, PageProperty[str]]],
        user_data : Dict[str, Any] = None
    ) -> "ChartClass":
        self._uuid = uuid
        self._max_page = max_page
        # Type checker has difficulty with PagePropertyOrValue and the typing of ensure_page_property.
        self._name = cast(PageProperty[str], ensure_page_property(name, parent=self))
        self._shape = ensure_page_property(shape, parent=self) 
        self._background_color = ensure_page_property(background_color, parent=self)
        self._border_color = ensure_page_property(border_color, parent=self)
        self._border_thickness = ensure_page_property(border_thickness, parent=self)
        self._foreground_color = ensure_page_property(foreground_color, parent=self)
        self._scale = cast(PageProperty[float], ensure_page_property(scale, parent=self)) 
        self._visible = cast(PageProperty[bool], ensure_page_property(visible, parent=self))
        self._x_nudge = cast(PageProperty[float], ensure_page_property(x_nudge, parent=self))
        self._y_nudge = cast(PageProperty[float], ensure_page_property(y_nudge, parent=self))

        self._dom_content = SignalDict(dom_content, parent=self) # type: ignore
        self._user_data = SignalDict(user_data, parent=self) # type: ignore

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
            border_thickness = self._border_thickness,
            foreground_color = self._foreground_color,
            scale=self.scale,
            visible=self.visible,
            x_nudge=self.x_nudge,
            y_nudge=self.y_nudge,
            dom_content=self.dom_content,
            user_data=self.user_data
        )

    def replace(self, **kwargs : Any):
        """ If a class currently not a "permanent cycle" then set it to be a permanent cycle. \
            Takes keyword arguments to set the properties of the "replaced" class.
            For instance:
                ``c.replace(color="red", name="2x")``
            Is the same as::

                page=c.max_page + 1
                c.max_page=INFINITY
                c.color[page:] = "red"
                c.name[page:] = "2x"
        """
        if self.max_page == INFINITY:
            raise ValueError("Class is already alive")
        page = self.max_page + 1
        self._max_page = INFINITY
        for [key, value] in kwargs.items():
            getattr(self, key)[page:] = value
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
    def edges(self) -> List[ChartEdge]:
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
        """ The maximum page the class may appear on. Note that the PageProperty "visible" also
            affects whether the class appears on a certain page: the class appears if class.visible[page]
            is "True" and page <= max_page.
        """
        return self._max_page
    
    @max_page.setter
    def max_page(self, v : int):
        self._max_page = v
        self._sseq._add_class_to_update(self)

    @property
    def name(self) -> PageProperty[str]:
        """ The name of the class. This appears in the tooltip in the display among other places. """
        return self._name

    @name.setter
    def name(self, v : PagePropertyOrValue[str]): # type: ignore
        self._name = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def shape(self) -> PageProperty[Shape]:
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        return self._shape
    
    @shape.setter
    def shape(self, v : PagePropertyOrValue[Shape]): # type: ignore
        self._shape = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def background_color(self) -> PageProperty[Color]:
        """ The color to use to draw the shape background. """
        return self._background_color
    
    @background_color.setter
    def background_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._background_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def border_color(self) -> PageProperty[Color]:
        """ The color to use to draw the shape border. """
        return self._border_color
    
    @shape.setter
    def border_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._border_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def foreground_color(self) -> PageProperty[Color]:
        """ The color to use to draw the shape foreground. """
        return self._foreground_color
    
    @foreground_color.setter
    def foreground_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._foreground_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def border_thickness(self) -> PageProperty[float]:
        """ The thickness to draw the shape border. """
        return self._border_thickness
    
    @border_thickness.setter
    def border_thickness(self, v : PagePropertyOrValue[float]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._border_thickness = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def scale(self) -> PageProperty[float]:
        """ The class scale. """
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

    @property
    def y_nudge(self) -> PageProperty[float]:
        """ The y displacement of the class from its default position. """
        return self._y_nudge

    @property
    def dom_content(self) -> SignalDict[Union[str, PageProperty[str]]]:
        """ A dictionary with extra fields that are used for purposes defined by the display. All data added must be serializable. """ 
        return self._dom_content

    @property
    def user_data(self) -> SignalDict[Any]:
        """ Miscellaneous extra data that the user wants to add. All data added must be serializable. """
        return self._user_data