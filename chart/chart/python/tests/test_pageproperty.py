from spectralsequence_chart.helper_classes.page_property import PageProperty
from spectralsequence_chart.infinity import INFINITY
from spectralsequence_chart.utils import JSON

from .test_serialization import assert_serialize_parse


class TestParent:
    def __init__(self):
        self.i = 0

    def _needs_update(self):
        self.i += 1


def test_signal():
    parent = TestParent()
    pp: PageProperty[int] = PageProperty(0, parent=parent)
    pp[:] = 1
    assert parent.i == 1
    pp[2] = 2
    assert parent.i == 2
    pp[2:] = 3
    assert parent.i == 3
    pp[:2] = 3
    assert parent.i == 4


def test_page_property():
    pp: PageProperty[int] = PageProperty(0)
    pp[:] = 1
    assert pp._values == [(-INFINITY, 1)]
    pp[2] = 2
    assert pp._values == [(-INFINITY, 1), (2, 2)]
    pp[3] = 3
    assert pp._values == [(-INFINITY, 1), (2, 2), (3, 3)]
    pp[2] = 1
    assert pp._values == [(-INFINITY, 1), (3, 3)]
    pp[5:10] = 7
    assert pp._values == [(-INFINITY, 1), (3, 3), (5, 7), (10, 3)]
    pp[4:8] = 10
    assert pp._values == [(-INFINITY, 1), (3, 3), (4, 10), (8, 7), (10, 3)]
    pp[4] = 3
    assert pp._values == [(-INFINITY, 1), (3, 3), (8, 7), (10, 3)]
    pp[2:9] = 10
    assert pp._values == [(-INFINITY, 1), (2, 10), (9, 7), (10, 3)]
    pp[4] = 7
    assert pp._values == [(-INFINITY, 1), (2, 10), (4, 7), (10, 3)]
    pp[7:20] = 15
    assert pp._values == [(-INFINITY, 1), (2, 10), (4, 7), (7, 15), (20, 3)]
    pp[6:] = 9
    assert pp._values == [(-INFINITY, 1), (2, 10), (4, 7), (6, 9)]
    pp[:3] = 2
    assert pp._values == [(-INFINITY, 2), (3, 10), (4, 7), (6, 9)]
    pp[:] = 77
    assert pp._values == [(-INFINITY, 77)]


def test_serialize():
    pp: PageProperty[int] = PageProperty(0)
    pp[:] = 1
    assert_serialize_parse(pp)
    pp[2] = 2
    assert_serialize_parse(pp)
    pp[3] = 3
    assert_serialize_parse(pp)
    pp[2] = 1
    assert_serialize_parse(pp)
    pp[5:10] = 7
    assert_serialize_parse(pp)
    pp[4:8] = 10
    assert_serialize_parse(pp)
    pp[4] = 3
    assert_serialize_parse(pp)
    pp[2:9] = 10
    assert_serialize_parse(pp)
    pp[4] = 7
    assert_serialize_parse(pp)
    pp[7:20] = 15
    assert_serialize_parse(pp)
    pp[6:] = 9
    assert_serialize_parse(pp)
    pp[:3] = 2
    assert_serialize_parse(pp)
    pp[:] = 77
    assert_serialize_parse(pp)
