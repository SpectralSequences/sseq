from abc import ABC, abstractmethod
from ..helper_classes import (
    PageProperty, PagePropertyOrValue, ensure_page_property,
    SignalDict
)
from uuid import uuid4

from typing import Optional, TYPE_CHECKING, Any, Dict, cast, Union, List, Tuple
if TYPE_CHECKING:
    from ..chart import SseqChart
    from .chart_class import ChartClass

UUID_str = str
Color = Tuple[float, float, float, float]
DashPattern = List[int]
LineWidth = Union[float, str]
ArrowTip = Any

class ChartEdge(ABC):
    def __init__(self,
        source_uuid : UUID_str, target_uuid : UUID_str, *,
        type : Optional[str] = None, 
        uuid : UUID_str = "",
        user_data : Optional[SignalDict[Any]] = None
    ):
        """ Do not call SseqEdge constructor directly, use instead SseqChart.add_structline(),
            SseqChart.add_differential(), SseqChart.add_extension(), or JSON.parse()."""
        self._sseq : SseqChart
        self._source_uuid = source_uuid
        self._target_uuid = target_uuid
        self.source : ChartClass
        self.target : ChartClass

        if type:
            assert type == self.__class__.__name__
        
        if uuid:
            self.uuid = uuid 
        else:
            self.uuid = str(uuid4())
        self._user_data : SignalDict[Any] = SignalDict(user_data if user_data else {}) # type: ignore

    def __repr__(self):
        fields = [repr(x) for x in [self.source, self.target]]
        return f"{type(self).__name__}({', '.join(fields)})"


    def _needs_update(self):
        self._sseq._add_edge_to_update(self)

    def replace_source(self, **kwargs : Any):
        self.source.replace(**kwargs)
    
    def replace_target(self, **kwargs : Any):
        self.target.replace(**kwargs)

    def delete(self):
        self._sseq._add_edge_to_delete(self)
        del self._sseq._edges[self.uuid]
        del self.source.edges[self.source.edges.index(self)]
        del self.target.edges[self.target.edges.index(self)]

    _EDGE_TYPE_DICT : Dict[str, type]
    @staticmethod
    def from_json(json : Dict[str, Any]) -> "ChartEdge":
        if not hasattr(ChartEdge, "EDGE_TYPE_DICT"):
            ChartEdge._EDGE_TYPE_DICT = {edge_type.__name__ : edge_type for edge_type in [ChartStructline, ChartDifferential, ChartExtension]}
        edge_type = json["type"]
        if edge_type in ChartEdge._EDGE_TYPE_DICT:
            return ChartEdge._EDGE_TYPE_DICT[edge_type](**json)
        else:
            type_names = list(ChartEdge._EDGE_TYPE_DICT.keys())
            types_list = ",".join(f'"{type}"' for type in type_names[:-1])
            types_list += f', or "${type_names[-1]}"'
            raise ValueError(f'"edge_type" should be one of {types_list}, not "{edge_type}"')

    @abstractmethod
    def to_json(self) -> Dict[str, Any]:
        return dict(
            type=self.__class__.__name__,
            uuid=self.uuid,
            source_uuid=self._source_uuid,
            target_uuid=self._target_uuid,
            # color=self.color,
            # dash_pattern=self.dash_pattern,
            # line_width=self.line_width,
            # bend=self.bend,
            # visible=self.visible,
            user_data=self._user_data
        )

class ChartStructline(ChartEdge):
    def __init__(self, 
        color : PagePropertyOrValue[Color] = (0, 0, 0, 1),
        dash_pattern : PagePropertyOrValue[DashPattern] = [],
        line_width : PagePropertyOrValue[float] = 3,
        bend : PagePropertyOrValue[float] = 0,
        start_tip : PagePropertyOrValue[ArrowTip] = None,
        end_tip : PagePropertyOrValue[ArrowTip] = None,
        visible : PagePropertyOrValue[bool] = True,
        **kwargs : Any
    ):
        super().__init__(**kwargs)
        self._color = cast(PageProperty[Color], ensure_page_property(color, parent=self))
        self._dash_pattern = cast(PageProperty[DashPattern], ensure_page_property(dash_pattern, parent=self))
        self._line_width = cast(PageProperty[Union[float, str]], ensure_page_property(line_width, parent=self))
        self._bend = cast(PageProperty[float], ensure_page_property(bend, parent=self)) 
        self._start_tip = cast(PageProperty[ArrowTip], ensure_page_property(start_tip, parent=self)) 
        self._end_tip = cast(PageProperty[ArrowTip], ensure_page_property(end_tip, parent=self)) 
        self._visible = cast(PageProperty[bool], ensure_page_property(visible, parent=self))
    
    def to_json(self) -> Dict[str, Any]:
        return dict(
            super().to_json(),
            type=self.__class__.__name__,
            uuid=self.uuid,
            source_uuid=self._source_uuid,
            target_uuid=self._target_uuid,
            color=self.color,
            dash_pattern=self.dash_pattern,
            line_width=self.line_width,
            bend=self.bend,
            visible=self.visible,
            user_data=self._user_data
        )

    @property
    def color(self) -> PageProperty[Color]:
        return self._color

    @color.setter
    def color(self, v : PagePropertyOrValue[Color]): # type: ignore
        self._color = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def dash_pattern(self) -> PageProperty[DashPattern]:
        return self._dash_pattern

    @dash_pattern.setter
    def dash_pattern(self, v : PagePropertyOrValue[DashPattern]): # type: ignore
        self._dash_pattern = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def line_width(self) -> PageProperty[LineWidth]:
        return self._line_width


    @line_width.setter
    def line_width(self, v : PagePropertyOrValue[LineWidth]): # type: ignore
        self._line_width = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def bend(self) -> PageProperty[float]:
        return self._bend

    @bend.setter
    def bend(self, v : PagePropertyOrValue[float]): # type: ignore
        self._bend = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def visible(self) -> PageProperty[bool]:
        return self._visible

    @visible.setter
    def visible(self, v : PagePropertyOrValue[bool]): # type: ignore
        self._visible = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def start_tip(self) -> PageProperty[ArrowTip]:
        return self._start_tip


    @start_tip.setter
    def start_tip(self, v : PagePropertyOrValue[ArrowTip]):
        self._start_tip = ensure_page_property(v, parent=self)
        self._needs_update()

    @property
    def end_tip(self) -> PageProperty[ArrowTip]:
        return self._end_tip

    @end_tip.setter
    def end_tip(self, v : PagePropertyOrValue[ArrowTip]):
        self._end_tip = ensure_page_property(v, parent=self)
        self._needs_update()

class SinglePageChartEdge(ChartEdge):
    def __init__(self, 
        color : Color = (0, 0, 0, 1),
        dash_pattern : DashPattern = [],
        line_width : float = 3,
        bend : float = 0,
        start_tip : ArrowTip = None,
        end_tip : ArrowTip = None,
        visible : bool = True,
        **kwargs : Any
    ):
        super().__init__(**kwargs)
        self._color = color
        self._dash_pattern = dash_pattern
        self._line_width = line_width
        self._bend = bend
        self._start_tip = start_tip
        self._end_tip = end_tip
        self._visible = visible

    @property
    def color(self) -> Color:
        return self._color

    @color.setter
    def color(self, v : Color):
        self._color = v
        self._needs_update()

    @property
    def dash_pattern(self) -> DashPattern:
        return self._dash_pattern

    @dash_pattern.setter
    def dash_pattern(self, v : DashPattern):
        self._dash_pattern = v
        self._needs_update()

    @property
    def line_width(self) -> LineWidth:
        return self._line_width


    @line_width.setter
    def line_width(self, v : LineWidth):
        self._line_width = v
        self._needs_update()

    @property
    def bend(self) -> float:
        return self._bend

    @bend.setter
    def bend(self, v : float):
        self._bend = v
        self._needs_update()

    @property
    def visible(self) -> bool:
        return self._visible

    @visible.setter
    def visible(self, v : bool):
        self._visible = v
        self._needs_update()


    @property
    def start_tip(self) -> ArrowTip:
        return self._start_tip


    @start_tip.setter
    def start_tip(self, v : ArrowTip):
        self._start_tip = v
        self._needs_update()

    @property
    def end_tip(self) -> ArrowTip:
        return self._end_tip

    @end_tip.setter
    def end_tip(self, v : ArrowTip):
        self._end_tip = v
        self._needs_update()


    def to_json(self) -> Dict[str, Any]:
        return dict(
            super().to_json(),
            type=self.__class__.__name__,
            uuid=self.uuid,
            source_uuid=self._source_uuid,
            target_uuid=self._target_uuid,
            color=self.color,
            start_tip=self.start_tip,
            end_tip=self.end_tip,
            dash_pattern=self.dash_pattern,
            line_width=self.line_width,
            bend=self.bend,
            visible=self.visible,
            user_data=self._user_data
        )


class ChartDifferential(ChartEdge):
    def __init__(self, page : int, **kwargs : Any):
        super().__init__(**kwargs)
        self.page : int = page

    def to_json(self) -> Dict[str, Any]:
        return dict(
            super().to_json(),
            page=self.page
        )

class ChartExtension(ChartEdge):
    def __init__(self, **kwargs : Any):
        super().__init__(**kwargs)

    def to_json(self) -> Dict[str, Any]:
        return super().to_json()