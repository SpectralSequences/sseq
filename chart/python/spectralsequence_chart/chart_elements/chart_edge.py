from ..helper_types import (
    PageProperty, PagePropertyOrValue, ensure_page_property,
    SignalDict
)
from uuid import uuid4

from typing import Optional, TYPE_CHECKING, Any, Dict, cast, Union
if TYPE_CHECKING:
    from ..chart import SseqChart
    from .chart_class import ChartClass

UUID_str = str
Color = Any
DashPattern = Any
LineWidth = Union[float, str]

class ChartEdge:
    def __init__(self,
        source_uuid : UUID_str, target_uuid : UUID_str, *,
        type : Optional[str] = None, 
        uuid : UUID_str = "",
        color : PagePropertyOrValue[Color] = "default",
        dash_pattern : PagePropertyOrValue[DashPattern] = "default",
        line_width : PagePropertyOrValue[LineWidth] = "default",
        bend : PagePropertyOrValue[float] = 0,
        visible : PagePropertyOrValue[bool] = True,
        user_data : Optional[SignalDict[Any]] = None
    ):
        self._sseq : SseqChart
        self.source_uuid = source_uuid
        self.target_uuid = target_uuid
        self.source : ChartClass
        self.target : ChartClass

        if type:
            assert type == self.__class__.__name__
        
        if uuid:
            self.uuid = uuid 
        else:
            self.uuid = str(uuid4())

        self._color = cast(PageProperty[Color], ensure_page_property(color, parent=self))
        self._dash_pattern = cast(PageProperty[DashPattern], ensure_page_property(dash_pattern, parent=self))
        self._line_width = cast(PageProperty[Union[float, str]], ensure_page_property(line_width, parent=self))
        self._bend = cast(PageProperty[float], ensure_page_property(bend, parent=self)) 
        self._visible = cast(PageProperty[bool], ensure_page_property(visible, parent=self))
        self._user_data : SignalDict[Any] = SignalDict(user_data if user_data else {})
        # setattr(self, utils.PROPERTY_PREFIX + "source",  self.source.uuid)
        # setattr(self, utils.PROPERTY_PREFIX + "target",  self.target.uuid)



    def needs_update(self):
        self._sseq.add_edge_to_update(self)

    def replace_source(self, **kwargs : Any):
        self.source.replace(**kwargs)
    
    def replace_target(self, **kwargs : Any):
        self.target.replace(**kwargs)

    def delete(self):
        self._sseq.add_edge_to_delete(self)
        del self._sseq._edges[self.uuid]
        del self.source._edges[self.source._edges.index(self)]
        del self.target._edges[self.target._edges.index(self)]

    EDGE_TYPE_DICT : Dict[str, type]
    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartEdge":
        if hasattr(ChartEdge, "EDGE_TYPE_DICT"):
            ChartEdge.EDGE_TYPE_DICT = {edge_type.__name__ : edge_type for edge_type in [ChartStructline, ChartDifferential, ChartExtension]}
        edge_type = json["type"]
        if edge_type in ChartEdge.EDGE_TYPE_DICT:
            return ChartEdge.EDGE_TYPE_DICT[edge_type](**json)
        else:
            type_names = list(ChartEdge.EDGE_TYPE_DICT.keys())
            types_list = ",".join(f'"{type}"' for type in type_names[:-1])
            types_list += f', or "${type_names[-1]}"'
            raise ValueError(f'"edge_type" should be one of {types_list}, not "{edge_type}"')

    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=self.__class__.__name__,
            color=self.color,
            dash_pattern=self.dash_pattern,
            line_width=self.line_width,
            bend=self.bend,
            visible=self.visible
        )

    @property
    def color(self) -> PageProperty[Color]:
        return self._color

    @property
    def dash_pattern(self) -> PageProperty[DashPattern]:
        return self._dash_pattern

    @property
    def line_width(self) -> PageProperty[LineWidth]:
        return self._line_width

    @property
    def bend(self) -> PageProperty[float]:
        return self._bend

    @property
    def visible(self) -> PageProperty[bool]:
        return self._visible
    

class ChartDifferential(ChartEdge):
    def __init__(self, page : int, **kwargs : Any):
        super().__init__(**kwargs)
        self.page : int = page

class ChartStructline(ChartEdge):
    def __init__(self, **kwargs : Any):
        super().__init__(**kwargs)

class ChartExtension(ChartEdge):
    def __init__(self, **kwargs : Any):
        super().__init__(**kwargs)