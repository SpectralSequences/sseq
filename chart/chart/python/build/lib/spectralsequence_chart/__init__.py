""" Represent spectral sequence charts.
Spectral sequence charts are rather inexact representations of the linear algebra data in a mathematical spectral sequence, 
and so in many situations disgression is necessary to decide what information about the mathematical spectral sequence to encode
and how to encode it. The goal of this package is to provide a toolkit for users to present charts.
Because of the intrinsic fuzziness of the situation, the emphasis of the package is to provide a good mixture of flexibility and 
ease of use. In particular, the package very unopionated about the particular way that the display models mathematics or any constraints.

At a basic level, the chart consists of `ChartClasses <ChartClass>` and `ChartEdges <ChartEdge>`. 

The appearance of the classes is controlled by a collection of properties which are allowed to vary from page to page. A class is fixed to a 
particular position but everything else about its appearance may change from page to page: the basic glyph designating the class, colors of the glyph,
the tooltip, and whether it appears at all.

The edges are divided into `ChartDifferentials <ChartDifferential>` which only appear on one page, `ChartExtensions <ChartExtension>` which only appear on the infinity page,
and `ChartStructlines <ChartStructline>`, which like classes may have all aspects of their appearance change from page to page.
"""

# from __future__ import annotations

# from spectralsequence_chart import chart_elements
# import spectralsequence_chart.chart
# import spectralsequence_chart.chart_elements
# import spectralsequence_chart.helper_classes

from .infinity import INFINITY
from .chart import SseqChart
from .chart_class import ChartClass, ChartClassStyle, Shape
from .chart_edge import (
    ChartEdge, ChartEdgeStyle,
    ChartStructline, ChartDifferential, ChartExtension,
)
from .display_primitives import ArrowTip, Color, Shape
from .serialization import JSON

__version__ = "0.0.28"
# __all__ = [ChartClass, ChartClassStyle, Shape, ]