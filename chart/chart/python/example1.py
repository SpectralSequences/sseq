
s = ChartClassStyle()
s.shape = Shape().boxed(10)
from spectralsequence_chart.chart_class import ChartClassStyle
chart = channel.chart.sseq
s.group_name = "Z"
chart.register_class_style(s)
s.background_color = Color.TRANSPARENT
s.group_name = "2Z"
chart.register_class_style(s)
chart.default_differential_style
chart.default_differential_style.end_tip = ArrowTip()
chart.default_differential_style.color = Color.BLUE

chart.add_class(0,0).set_style("Z")
chart.add_class(1,1)
chart.add_structline((0,0),(1,1))
chart.add_class(2,2)
chart.add_structline((1,1),(2,2))
chart.add_class(3,3)
chart.add_structline((2,2),(3,3))

chart.add_class(4,0).set_style("Z")

chart.add_differential(3, (4,0), (3,3)).replace_source("2Z")

await chart.update_a()