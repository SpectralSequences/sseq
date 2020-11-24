import sys
import pyodide

from js import messageLookup as js_message_lookup
from .async_js import WebLoop

from .handler_decorator import *
from .completer import Completer
from .execution import Execution
from .sseq_display import SseqDisplay

@collect_handlers("message_handlers")
class PyodideExecutor:
    executor = None

    def __init__(self, namespace=None):
        self.namespace = namespace or {}
        from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.flags = PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.completers = {}
        self.loop = WebLoop() 

    def handle_message(self, message_id):
        message = dict(js_message_lookup[message_id])
        del js_message_lookup[message_id]
        self.handle_message_helper(**message)

    def handle_message_helper(self, cmd, **kwargs):
        if cmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized command "{cmd}"')
        self.loop.call_soon(self.message_handlers[cmd](self, **kwargs))

    @handle("execute")
    async def execute(self, **kwargs):
        self.loop.call_soon(Execution(self, **kwargs).run())

    @handle("complete")
    async def complete(self, uuid, subcmd, **kwargs):
        if subcmd == "new_completer":
            self.completers[uuid] = Completer(self, uuid=uuid, **kwargs)
            return
        if uuid not in self.completers:
            raise Exception(f"No completer with uuid {uuid}")
        self.completers[uuid].handle_message(subcmd=subcmd, **kwargs)

    @handle("subscribe_chart_display")
    async def add_subscriber(self, uuid, chart_name, port):
        display = SseqDisplay.displays[chart_name]
        await display.add_subscriber(uuid, port)
