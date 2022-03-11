def get_namespace():
    from spectralsequence_chart.chart import SseqChart
    from spectralsequence_chart.chart_class import (ChartClass, ChartClassArg,
                                                    ChartClassStyle)
    from spectralsequence_chart.chart_edge import (ChartDifferential,
                                                   ChartEdgeStyle,
                                                   ChartExtension,
                                                   ChartStructline)
    from spectralsequence_chart.display_primitives import (ArrowTip, Color,
                                                           Shape)
    from spectralsequence_chart.infinity import INFINITY
    from spectralsequence_chart.page_property import PageProperty
    from spectralsequence_chart.serialization import JSON
    from spectralsequence_chart.utils import format_monomial
    from sseq_display import create_display, load_display_a

    objects = [
        create_display,
        load_display_a,
        SseqChart,
        ChartClassStyle,
        ChartEdgeStyle,
        ArrowTip,
        Color,
        Shape,
        format_monomial,
        JSON,
    ]
    namespace = {n.__name__: n for n in objects}
    namespace["INFINITY"] = INFINITY
    return namespace
