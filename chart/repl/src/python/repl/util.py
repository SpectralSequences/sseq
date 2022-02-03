from contextlib import contextmanager
from pyodide import to_js as _to_js
from js import Object
from pyodide_js import setInterruptBuffer

__all__ = ["to_js", "set_interrupt_buffer"]


def to_js(o):
    return _to_js(o, dict_converter=Object.fromEntries)


@contextmanager
def set_interrupt_buffer(ib):
    try:
        setInterruptBuffer(ib)
        yield
    finally:
        setInterruptBuffer()
