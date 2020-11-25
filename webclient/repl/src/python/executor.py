from re import sub
from js import console

from .handler_decorator import *
from .completer import Completer
from .execution import Execution
from .sseq_display import SseqDisplay

@collect_handlers("message_handlers")
class Executor:
    executor = None
    def __init__(self, loop, send_message_a, namespace=None):
        self.namespace = namespace or {}
        from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.flags = PyCF_ALLOW_TOP_LEVEL_AWAIT
        self.completers = {}
        self.loop = loop
        self.send_message_a = send_message_a

    def handle_message(self, cmd, **kwargs):
        if cmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized command "{cmd}"')
        self.loop.call_soon(self.message_handlers[cmd](self, **kwargs))

    @handle("execute")
    async def execute_a(self, **kwargs):
        await Execution(self, **kwargs).run_a()

    @handle("complete")
    async def complete_a(self, uuid, subcmd, **kwargs):
        console.log("complete", "subcmd:", subcmd, kwargs)
        if subcmd == "new_completer":
            self.completers[uuid] = Completer(self, uuid=uuid, **kwargs)
            return
        if uuid not in self.completers:
            raise Exception(f"No completer with uuid {uuid}")
        await self.completers[uuid].handle_message_a(subcmd=subcmd, **kwargs)

    @handle("subscribe_chart_display")
    async def add_subscriber_a(self, uuid, chart_name, port):
        display = SseqDisplay.displays[chart_name]
        await display.add_subscriber(uuid, port)
