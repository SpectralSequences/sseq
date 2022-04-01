import pexpect
import pytest
import json
from pathlib import Path
from hypothesis.stateful import rule
from contextlib import contextmanager

import sys

DIR = Path(__file__).parent.resolve()

TEST_DRIVER = DIR / "node_test_driver.js"
SSEQ_CHART = DIR / "../javascript/dist/sseq_chart_node.js"

sys.path.append(str(DIR / "../python"))
sys.path.append(str(DIR / "../python/tests"))

from spectralsequence_chart import SseqChart
from spectralsequence_chart.signal_dict import SignalDict
from spectralsequence_chart.display_primitives import Color
from spectralsequence_chart.serialization import JSON
from test_hypothesis import HypothesisStateMachine


class JavascriptException(Exception):
    def __init__(self, msg, stack):
        self.msg = msg
        self.stack = stack
        # In chrome the stack contains the message
        if self.stack and self.stack.startswith(self.msg):
            self.msg = ""

    def __str__(self):
        return "\n\n".join(x for x in [self.msg, self.stack] if x)


def node_driver():
    p = pexpect.spawn(
        f"node {TEST_DRIVER} {SSEQ_CHART}"
    )
    p.setecho(False)
    p.delaybeforesend = None
        
    _logs = []
    # _timeout = 20
    def run_js(code):
        wrapped = """
            let result = await (async () => { %s })();
            return result;
        """ % (
            code,
        )
        from uuid import uuid4

        cmd_id = str(uuid4())
        p.sendline(cmd_id)
        p.sendline(wrapped)
        p.sendline(cmd_id)
        p.expect_exact(f"{cmd_id}:UUID\r\n")
        p.expect_exact(f"{cmd_id}:UUID\r\n")
        if p.before:
            _logs.append(p.before.decode()[:-2].replace("\r", ""))
        p.expect(f"[01]\r\n")
        success = int(p.match[0].decode()[0]) == 0
        p.expect_exact(f"\r\n{cmd_id}:UUID\r\n")
        if success:
            return json.loads(p.before.decode().replace("undefined", "null"))
        else:
            raise JavascriptException("", p.before.decode())
    try:
        yield run_js
    finally:
        p.sendeof()



node_driver_ctx = contextmanager(node_driver)

@pytest.fixture(scope="module")
def run_js():
    with node_driver_ctx() as run_js:
        yield run_js

from functools import partial

class StateMachinePythonToJavascript(HypothesisStateMachine):
    def __init__(self):
        super().__init__()
        self.chart.update = partial(self.update_patch, self.chart)
        self._driver_gen = node_driver()
        self.driver = next(self._driver_gen)
        # print(self.driver(
        #     """
        #     return Array.from(getJsonTypes().keys());
        #     """
        # ))
        self.driver(
            f"""
            globalThis.chart = parse(JSON.parse({JSON.stringify(self.chart)!r}));
            """
        )

    def update_patch(state_machine, chart):
        messages = chart._batched_messages
        # print(state_machine.driver("return typeof globalThis.chart;"))
        # print(state_machine.driver("return Reflect.ownKeys(globalThis.chart);"))
        print("msgs:", JSON.stringify(messages))
        for msg in messages:
            state_machine.driver(
                f"""
                globalThis.chart.handleMessage(parse(JSON.parse({JSON.stringify(msg)!r})));
                """
            )
        chart._clear_batched_messages()

#    @rule()
    def update_1(self):
        self.chart.update()
        s1 = JSON.stringify(self.chart)
        s2 = self.driver(
            """
            return JSON.stringify(globalThis.chart);
            """
        )
        c1 = json.loads(s1)
        del c1["version"]
        c2 = json.loads(s2)
        assert c1 == c2

    def teardown(self):
        try:
            next(self._driver_gen)
        except StopIteration:
            pass
        else:
            raise AssertionError()

TestPythonToJavascript = StateMachinePythonToJavascript.TestCase


chart = SseqChart("hi")
cls = chart.add_class(0,0)
sl = chart.add_structline(cls, cls)
d = chart.add_differential(2, cls, cls)
e = chart.add_extension(cls, cls)
examples_list = {
    "sigdict" : SignalDict(),
    "color" : Color(0, 0, 0),
    "class":  cls,
    "structline": sl,
    "differential": d,
    "extension" : e,
}

@pytest.mark.parametrize("c", examples_list.values(), ids=examples_list.keys())
def test_js_python_serialization_agree(run_js, c):
    json1 = JSON.stringify(c)
    print("json1:", json1)        
    json2 = run_js(f"return JSON.stringify(parse({json1}))")
    print("json2:", json2)
    o1 = json.loads(json1)
    o2 = json.loads(json2)
    o2.pop("color_vec", None)
    assert o1 == o2




if __name__ == "__main__":
    state = StateMachinePythonToJavascript()
    v1 = state.add_class(k=(0, 0))
    # c = state.chart.classes[0]
    # c.background_color = Color.
    state.update_1()
    state.teardown()
    import pprint
    chart = SseqChart("a")
    chart.add_class(0,0)
    pprint.pprint(JSON.stringify(chart))





