import json
from typing import Any, Dict, Tuple, Union, cast  # , Protocol

# Protocol absent from Python 3.6, comment out until I figure out how to get sphinx to use python 3.8


def stringifier(obj: Any) -> Union[str, dict[str, Any]]:
    if hasattr(obj, "to_json"):
        return obj.to_json()
    elif hasattr(obj, "__dict__"):
        return obj.__dict__
    elif obj is None:
        return None
    else:
        return str(obj)


# To make typechecker happy...
class Serializable:  # (Protocol):
    @staticmethod
    def from_json(json: dict[str, Any]):
        return Serializable()


class JSON:
    @staticmethod
    def stringify(obj: Any):
        # sort_keys needed to ensure that object equality ==> string equality,
        # useful for ease of testing.
        return json.dumps(obj, default=stringifier, sort_keys=True)

    @staticmethod
    def parse(json_str: str) -> Any:
        return json.loads(json_str, object_hook=JSON.parser_object_hook)

    @staticmethod
    def parser_object_hook(json_dict: dict[str, Any]) -> Any:
        JSON.ensure_types_are_initialized()
        if "type" not in json_dict:
            return json_dict
        return JSON.types[json_dict["type"]].from_json(json_dict)

    types: dict[str, Serializable]

    @staticmethod
    def ensure_types_are_initialized():
        if hasattr(JSON, "types"):
            return
        from .chart import SseqChart
        from .chart_class import ChartClass, ChartClassStyle
        from .chart_edge import (ChartDifferential, ChartEdgeStyle,
                                 ChartExtension, ChartStructline)
        from .display_primitives import ArrowTip, Color, Shape
        from .page_property import PageProperty
        from .signal_dict import SignalDict, SignalList

        JSON.types = {
            t.__name__: cast(Serializable, t)
            for t in [
                SseqChart,
                ChartClass,
                ChartClassStyle,
                ChartEdgeStyle,
                ChartStructline,
                ChartDifferential,
                ChartExtension,
                ArrowTip,
                Color,
                Shape,
                PageProperty,
                SignalDict,
                SignalList,
            ]
        }
