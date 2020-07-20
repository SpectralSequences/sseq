from ..helper_types import (
    PageProperty, PagePropertyOrValue, ensure_page_property, 
    SignalDict
)
from ..infinity import INFINITY
from uuid import uuid4

from typing import TYPE_CHECKING, List, Any, Tuple, cast, Dict, Union, Optional
if TYPE_CHECKING:
    from ..chart import SseqChart
    from .chart_edge import ChartEdge

Color = Any
Shape = Any
UUID_str = str
# PropertyValue = 

class ChartClass:
    def __init__(self,
        degree : Tuple[int, ...], *,
        type : str = "ChartClass",
        idx : Optional[int] = None,
        uuid : UUID_str = "",
        name : PagePropertyOrValue[str] = "",
        shape : PagePropertyOrValue[Shape] = "default",
        color : PagePropertyOrValue[Color] = "default",
        fill : PagePropertyOrValue[Color] = "default",
        stroke : PagePropertyOrValue[Color] = "default",
        scale : PagePropertyOrValue[float] = 1,
        opacity : PagePropertyOrValue[int] = 1,
        visible : PagePropertyOrValue[bool] = True,
        x_nudge : PagePropertyOrValue[float] = 0,
        y_nudge : PagePropertyOrValue[float] = 0,
        dom_content : Optional[SignalDict[Union[str, PageProperty[str]]]] = None,
        user_data : Optional[SignalDict[Any]] = None
    ):
        assert type == self.__class__.__name__
        self._sseq : SseqChart
        self._degree = degree
        self.idx = idx
        self._max_page = INFINITY
        self._edges : List[ChartEdge] = []
        
        if uuid:
            self.uuid = uuid
        else:
            self.uuid = str(uuid4())

        # Type checker has difficulty with PagePropertyOrValue and the typing of ensure_page_property.
        self._name = cast(PageProperty[str], ensure_page_property(name, parent=self))
        self._shape = cast(PageProperty[Shape], ensure_page_property(shape, parent=self))
        self._color = cast(PageProperty[Color], ensure_page_property(color, parent=self))
        self._fill  = cast(PageProperty[Color], ensure_page_property(fill, parent=self))
        self._stroke = cast(PageProperty[Color], ensure_page_property(stroke, parent=self))
        self._scale = cast(PageProperty[float], ensure_page_property(scale, parent=self)) 
        self._opacity = cast(PageProperty[float], ensure_page_property(opacity, parent=self))
        self._visible = cast(PageProperty[bool], ensure_page_property(visible, parent=self))
        self._x_nudge = cast(PageProperty[float], ensure_page_property(x_nudge, parent=self))
        self._y_nudge = cast(PageProperty[float], ensure_page_property(y_nudge, parent=self))

        self._dom_content : SignalDict[Union[str, PageProperty[str]]]  = SignalDict(dom_content if dom_content else {}, parent=self)
        self._user_data : SignalDict[Any] = SignalDict(user_data if user_data else {}, parent=self)

    def needs_update(self):
        self._sseq.add_class_to_update(self)

    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartClass":
        return ChartClass(**json)

    def __repr__(self) -> str:
        return f"ChartClass({self.x},{self.y})"

    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=type(self).__name__,
            degree=self.degree,
            idx=self.idx,
            uuid=self.uuid,
            name=self.name,
            shape=self.shape,
            color=self.color,
            fill=self.fill,
            stroke=self.stroke,
            scale=self.scale,
            opacity=self.opacity,
            visible=self.visible,
            x_nudge=self.x_nudge,
            y_nudge=self.y_nudge,
            dom_content=self.dom_content,
            user_dict=self.user_data
        )

    def replace(self, **kwargs : Any):
        page = self.max_page + 1
        self._max_page = INFINITY
        # if self.max_page == INFINITY:
        #     raise ValueError("???")
        for [key, value] in kwargs.items():
            getattr(self, key)[page:] = value
        self.needs_update()
        return self

    def delete(self):
        self._sseq.add_class_to_delete(self)
        del self._sseq._classes[self.uuid]
        for e in self._edges:
            e.delete()

    def __repr__(self) -> str:
        fields = [str(self.x), str(self.y), str(self.idx)]
        if self.name[0] != "":
            fields.append(f'name="{self.name[0]}"')
        return f"Class({','.join(fields)})"

    @property
    def degree(self):
        return self._degree

    @property
    def x(self):
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.x_degree))

    @property
    def y(self):
        return sum(a*b for (a,b) in zip(self.degree,self._sseq.y_degree))

    # TODO: Should max_page exist?
    @property
    def max_page(self) -> int:
        return self._max_page
    
    @property
    def name(self) -> PageProperty[str]:
        return self._name

    @property
    def shape(self) -> PageProperty[Shape]:
        return self._shape
    
    @property
    def color(self) -> PageProperty[Color]:
        return self._color

    @property
    def stroke(self) -> PageProperty[Color]:
        return self._stroke

    @property
    def fill(self) -> PageProperty[Color]:
        return self._fill

    @property
    def scale(self) -> PageProperty[float]:
        return self._scale

    @property
    def opacity(self) -> PageProperty[float]:
        return self._opacity
        
    @property
    def visible(self) -> PageProperty[bool]:
        return self._visible

    @property
    def x_nudge(self) -> PageProperty[float]:
        return self._x_nudge

    @property
    def y_nudge(self) -> PageProperty[float]:
        return self._y_nudge

    @property
    def dom_content(self) -> SignalDict[Union[str, PageProperty[str]]]:
        return self._dom_content

    @property
    def user_data(self) -> SignalDict[Any]:
        return self._user_data