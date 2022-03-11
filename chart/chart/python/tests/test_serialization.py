from spectralsequence_chart import SseqChart
from spectralsequence_chart.serialization import JSON


def single_structline_chart():
    chart = SseqChart("test")
    c0 = chart.add_class(0, 0)
    c1 = chart.add_class(1, 1)
    chart.add_structline(c0, c1)
    return chart


def serialize_parse(o):
    return JSON.parse(JSON.stringify(o))


def assert_serialize_parse(o):
    assert o == serialize_parse(o)


def double_serialize_assert(o):
    [s1, s2] = double_serialize(o)
    assert s1 == s2


def double_serialize(o):
    s1 = JSON.stringify(o)
    s2 = JSON.stringify(JSON.parse(s1))
    return [s1, s2]


def test_serparse():
    chart = single_structline_chart()
    chart2 = serialize_parse(chart)
    assert len(chart.classes) == len(chart2.classes)
    assert len(chart.edges) == len(chart2.edges)


def test_serialize():
    chart = single_structline_chart()
    for c in chart.classes:
        double_serialize_assert(c)
    double_serialize_assert(chart)
