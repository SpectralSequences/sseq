import json
from typing import Tuple, Any, Dict, Union, cast #, Protocol 
# Protocol absent from Python 3.6, comment out until I figure out how to get sphinx to use python 3.8

def stringifier(obj : Any) -> Union[str, Dict[str, Any]]:
    if hasattr(obj, "to_json"):
        return obj.to_json()
    elif hasattr(obj, "__dict__"):
        return obj.__dict__
    else:
        return str(obj)

# To make typechecker happy...
class Serializable: #(Protocol):
    @staticmethod
    def from_json(json : Dict[str, Any]):
        return Serializable()


class JSON:
    @staticmethod
    def stringify(obj : Any):
        # sort_keys needed to ensure that object equality ==> string equality,
        # useful for ease of testing.
        return json.dumps(obj, default=stringifier, sort_keys=True) 

    @staticmethod
    def parse(json_str : str) -> Any:
        return json.loads(json_str, object_hook = JSON.parser_object_hook )

    @staticmethod
    def parser_object_hook(json_dict : Dict[str, Any]) -> Any:
        JSON.ensure_types_are_initialized()
        if "type" not in json_dict:
            return json_dict
        return JSON.types[json_dict["type"]].from_json(json_dict)

    types : Dict[str, Serializable]
    @staticmethod
    def ensure_types_are_initialized():
        if hasattr(JSON, "types"):
            return
        from .chart import SseqChart 
        from .chart_class import (ChartClass, ChartClassStyle)
        from .chart_edge import (
            ChartEdgeStyle, ChartStructline, ChartDifferential, ChartExtension
        )
        from .display_primitives import (
            ArrowTip, Color, Shape
        )
        from .signal_dict import (SignalDict, SignalList)
        from .page_property import PageProperty
        JSON.types = { t.__name__ : cast(Serializable, t) for t in [
            SseqChart,
            ChartClass, ChartClassStyle,
            ChartEdgeStyle, ChartStructline, ChartDifferential, ChartExtension,
            ArrowTip, Color, Shape,
            PageProperty,
            SignalDict, SignalList
        ]}