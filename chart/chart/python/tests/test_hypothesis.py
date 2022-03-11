from spectralsequence_chart import SseqChart, ChartClass
from spectralsequence_chart.css_colors import CSS_COLORS_JSON
from spectralsequence_chart.display_primitives import Color
from spectralsequence_chart.serialization import JSON
from hypothesis.stateful import Bundle, RuleBasedStateMachine, rule, consumes
import hypothesis.strategies as st
import json

from typing import Optional

def update_patch(self):
    messages = self._batched_messages
    for msg in messages:
        self.other_chart.handle_message(json.loads(JSON.stringify(msg)))
    self._clear_batched_messages()

colors_list = sorted(CSS_COLORS_JSON.keys())
colors_strategy = st.one_of(st.tuples(*((st.integers(),)*4)).map(lambda t: Color(*t)), st.sampled_from(colors_list))


@st.composite
def slices(draw) -> Optional[slice]:
    """Generates slices that will select indices up to the supplied size

    Generated slices will have start and stop indices that range from -size to size - 1
    and will step in the appropriate direction. Slices should only produce an empty selection
    if the start and end are the same.

    Examples from this strategy shrink toward 0 and smaller values
    """
    if not draw(st.booleans()):
        return None
    # For slices start is inclusive and stop is exclusive
    size = 10_000
    start = draw(st.integers(0, size - 1) | st.none())
    stop = draw(st.integers((start or 0) + 1, size) | st.none())
    return slice(start, stop)



class HypothesisStateMachine(RuleBasedStateMachine):
    def __init__(self):
        super().__init__()
        self.chart = SseqChart("test")
        self.num_classes = 0
        self.num_edges = 0

    classes = Bundle("classes")
    structlines = Bundle("structlines")
    differentials = Bundle("differentials")
    extensions = Bundle("extensions")
    edges = st.one_of(structlines, differentials, extensions)
    chart_objects = st.one_of(edges, classes)
    edge_and_range = st.one_of(st.tuples(structlines, slices()), st.tuples(st.one_of(differentials, extensions), st.none()))
    obj_and_range = st.one_of(st.tuples(classes, slices()), edge_and_range)

    @rule(target=classes, k=st.tuples(st.integers(), st.integers()))
    def add_class(self, k):
        self.num_classes += 1
        return self.chart.add_class(*k)
    
    @rule(target=structlines, c1=classes, c2=classes)
    def add_structline(self, c1, c2):
        self.num_edges += 1
        return self.chart.add_structline(c1, c2)
        
    @rule(target=extensions, c1=classes, c2=classes)
    def add_extension(self, c1, c2):
        self.num_edges += 1
        return self.chart.add_extension(c1, c2)

    @rule(target=differentials, page=st.integers(), c1=classes, c2=classes, auto=st.booleans())
    def add_differential(self, page, c1, c2, auto):
        self.num_edges += 1
        return self.chart.add_differential(page, c1, c2, auto)

    @rule(o=classes, prop=st.sampled_from(["name", "group_name"]), val=st.text())
    def set_class_name(self, o, prop, val):
        setattr(o, prop, val)

    @rule(o=classes, prop=st.sampled_from(["background_color", "border_color", "foreground_color"]), page_range=slices(), val=colors_strategy)
    def set_class_color(self, o, prop, page_range, val):
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)

    @rule(o=classes, prop=st.sampled_from(["border_width", "scale", "x_nudge", "y_nudge"]), page_range=slices(), val=st.integers())
    def set_class_number(self, o, prop, page_range, val):
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)

    @rule(obj_and_range=obj_and_range, val=st.booleans())
    def set_visible(self, obj_and_range, val):
        [o, page_range] = obj_and_range
        prop = "visible"
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)

    @rule(edge_and_range=edge_and_range, val=colors_strategy)
    def set_edge_color(self, edge_and_range, val):
        [o, page_range] = edge_and_range
        prop = "color"
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)

    @rule(edge_and_range=edge_and_range, val=st.lists(st.integers()))
    def set_edge_dash_pattern(self, edge_and_range, val):
        [o, page_range] = edge_and_range
        prop = "dash_pattern"
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)
    
    @rule(edge_and_range=edge_and_range, prop=st.sampled_from(["line_width", "bend"]), val=st.integers())
    def set_edge_number(self, edge_and_range, prop, val):
        [o, page_range] = edge_and_range
        if page_range:
            getattr(o, prop)[page_range] = val
        else:
            setattr(o, prop, val)

    @rule()
    def check_num_classes(self):
        assert self.num_classes == len(self.chart.classes)

    @rule()
    def check_num_edges(self):
        assert self.num_edges == len(self.chart.edges)

    @rule()
    def double_serialize(self):
        s1 = JSON.stringify(self.chart)
        s2 = JSON.stringify(JSON.parse(s1))
        assert json.loads(s1) == json.loads(s2)

    @rule(o=st.one_of(consumes(classes), consumes(extensions), consumes(structlines), consumes(differentials)))
    def delete_object(self, o):
        if isinstance(o, ChartClass):
            self.num_classes -= 1
            self.num_edges -= len(set(o.edges))
        else:
            self.num_edges -= 1
        o.delete()


class StateMachinePythonToPython(HypothesisStateMachine):
    def __init__(self):
        super().__init__()
        self.other_chart = JSON.parse(JSON.stringify(self.chart))
        self.chart.other_chart = self.other_chart
        self.chart.update = update_patch.__get__(self.chart)

    @rule()
    def update_1(self):
        self.chart.update()
        s1 = JSON.stringify(self.chart)
        s2 = JSON.stringify(self.other_chart)
        assert json.loads(s1) == json.loads(s2)

TestStateMachinePythonToPython = StateMachinePythonToPython.TestCase
