from ..helper_classes import (
    PageProperty, PagePropertyOrValue, ensure_page_property, 
    SignalDict
)
from ..infinity import INFINITY
from uuid import uuid4
from .chart_types import UUID_str, Color
from .chart_shape import Shape

from typing import TYPE_CHECKING, List, Any, Tuple, cast, Dict, Union, Optional
if TYPE_CHECKING:
    from ..chart import SseqChart
    from .chart_edge import ChartEdge



class ChartClass:
    def __init__(self,
        degree : Tuple[int, ...], *,
        type : Optional[str] = None,
        idx : Optional[int] = None,
        uuid : UUID_str = "",
        name : PagePropertyOrValue[str] = "",
        max_page : int = INFINITY,
        visible : PagePropertyOrValue[bool] = True,
        x_nudge : PagePropertyOrValue[float] = 0,
        y_nudge : PagePropertyOrValue[float] = 0,
        scale : PagePropertyOrValue[float] = 1,
        shape : PagePropertyOrValue[Shape] = None,
        background_color : PagePropertyOrValue[Color] = None,
        border_color : PagePropertyOrValue[Color] = None,
        border_thickness : PagePropertyOrValue[float] = None,
        foreground_color : PagePropertyOrValue[Color] = None,
        dom_content : Optional[SignalDict[Union[str, PageProperty[str]]]] = None,
        user_data : Optional[SignalDict[Any]] = None
    ):
        """ Do not call SseqClass constructor directly, use instead SseqChart.add_class(), or JSON.parse()."""
        if type:
            assert type == self.__class__.__name__
        self._sseq : SseqChart
        self._degree = tuple(degree)
        self.idx = idx
        self._max_page = max_page
        self.edges : List[ChartEdge] = []
        
        if uuid:
            self.uuid = uuid
        else:
            self.uuid = str(uuid4())

        # Type checker has difficulty with PagePropertyOrValue and the typing of ensure_page_property.
        self._name = cast(PageProperty[str], ensure_page_property(name, parent=self))
        self._shape = ensure_page_property(shape or Shape().circled(5), parent=self) 
        self._background_color = ensure_page_property(background_color or (0, 0, 0, 1), parent=self)
        self._border_color = ensure_page_property(border_color or (0, 0, 0, 1), parent=self)
        self._border_thickness = ensure_page_property(border_thickness or 3, parent=self)
        self._foreground_color = ensure_page_property(foreground_color or (0, 0, 0, 1), parent=self)
        self._scale = cast(PageProperty[float], ensure_page_property(scale, parent=self)) 
        self._visible = cast(PageProperty[bool], ensure_page_property(visible, parent=self))
        self._x_nudge = cast(PageProperty[float], ensure_page_property(x_nudge, parent=self))
        self._y_nudge = cast(PageProperty[float], ensure_page_property(y_nudge, parent=self))

        self._dom_content : SignalDict[Union[str, PageProperty[str]]]  = SignalDict(dom_content if dom_content else {}, parent=self) # type: ignore
        self._user_data : SignalDict[Any] = SignalDict(user_data if user_data else {}, parent=self) # type: ignore

    def _needs_update(self):
        self._sseq._add_class_to_update(self)

    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartClass":
        return ChartClass(**json)

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
        """ Deletes the current class. Also deletes any edges incident to it."""
        self._sseq._add_class_to_delete(self)
        del self._sseq._classes[self.uuid]
        idx = cast(int, self.idx)
        del self._sseq._classes_by_degree[self.degree][idx]
        # Update indices of other classes
        for (idx, c) in enumerate(self._sseq._classes_by_degree[self.degree]):
            if idx != c.idx:
                c.idx = idx
                self._sseq._add_class_to_update(self)

        for e in self.edges:
            e.delete()

    def __repr__(self) -> str:
        fields = [repr(x) for x in (*self.degree, self.idx)]
        if self.name[0] != "":
            fields.append(f'name="{self.name[0]}"')
        return f"{type(self).__name__}({', '.join(fields)})"

    @property
    def degree(self):
        return self._degree

    @property
    def x(self):
        """ Get the coordinate on the x-axis that the class will be displayed in. \
            
            Returns: The dot product of sseq.x_projection with class.degree
        """
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.x_projection))

    @property
    def y(self):
        """ Get the coordinate on the y-axis that the class will be displayed in. \
            
            Returns: The dot product of sseq.y_projection with class.degree.
        """
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.y_projection))

    @property
    def max_page(self) -> int:
        """ Get the maximum page the class may appear on. Note that the PageProperty "visible" also
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
        """ The name of the class. This appears in the tooltip in the display among other places. 
            Note that the name is not normally usable to look up a class (you can make your own dict
            for this purpose though).
        """
        return self._name

    @name.setter
    def name(self, v : PagePropertyOrValue[str]): # type: ignore
        self._name = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def shape(self) -> PageProperty[Shape]:
        return self._shape
    
    @shape.setter
    def shape(self, v : PagePropertyOrValue[Shape]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._shape = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def background_color(self) -> PageProperty[Color]:
        return self._background_color
    
    @background_color.setter
    def background_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._background_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def border_color(self) -> PageProperty[Color]:
        return self._border_color
    
    @shape.setter
    def border_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._border_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def foreground_color(self) -> PageProperty[Color]:
        return self._foreground_color
    
    @foreground_color.setter
    def foreground_color(self, v : PagePropertyOrValue[Color]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._foreground_color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def border_thickness(self) -> PageProperty[float]:
        return self._border_thickness
    
    @border_thickness.setter
    def border_thickness(self, v : PagePropertyOrValue[float]): # type: ignore
        """ Control how to draw the class. Note that it is the responsibility of display implementations to handle these shapes."""
        self._border_thickness = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def scale(self) -> PageProperty[float]:
        return self._scale

    @scale.setter 
    def scale(self, v : PagePropertyOrValue[float]): # type: ignore
        self._scale = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def visible(self) -> PageProperty[bool]:
        """ Determine whether the class is visible on a certain page. Note that the field "max_page" also determines whether the class
            appears on a given page. The class appears if class.visible[page] is "True" and page <= max_page.
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
        """ A dictionary with extra fields that are used for purposes defined by the display. All data MUST BE SERIALIZABLE. """ 
        return self._dom_content

    @property
    def user_data(self) -> SignalDict[Any]:
        """ Miscellaneous extra data that the user wants to add. All data MUST BE SERIALIZABLE. """
        return self._user_data