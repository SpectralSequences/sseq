""" TODO: Overview of the package.

"""

# from __future__ import annotations

# from spectralsequence_chart import chart_elements
# import spectralsequence_chart.chart
# import spectralsequence_chart.chart_elements
# import spectralsequence_chart.helper_classes

from .chart_class import ChartClass, ChartClassStyle, Shape
from .chart_edge import (
    ChartEdge, ChartEdgeStyle,
    ChartStructline, ChartDifferential, ChartExtension,
)
from .infinity import INFINITY
from .chart import SseqChart

__version__ = "0.0.11"
# __all__ = [*(x.__name__ for x in [
#     SseqChart, 
#     ChartClass, ChartClassStyle, Shape,
#     ChartEdge, ChartEdgeStyle,
#     ChartStructline, ChartDifferential, ChartExtension
# ]), "INFINITY"]

# __all__ = [*(x.__name__ for x in [
#     SseqChart, 
#     ChartClass, ChartClassStyle, Shape,
#     ChartEdge, ChartEdgeStyle,
#     ChartStructline, ChartDifferential, ChartExtension
# ]), "INFINITY"]
  
# __all__ = ["chart", "chart_elements"] 