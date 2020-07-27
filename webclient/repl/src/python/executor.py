import sys
import pyodide

from js import message_lookup

from .handler_decorator import *
from .completer import Completer
from .execution import Execution


@collect_handlers("message_handlers")
class PyodideExecutor:
    def __init__(self, namespace=None, flags=0):
        self.namespace = namespace or {}
        self.namespace["enable_interrupts"] = True
        self.flags = flags
        self.completers = {}

    def handle_message(self, message_id):
        message = dict(message_lookup[message_id])
        self.handle_message_helper(**message)

    def handle_message_helper(self, cmd, **kwargs):
        if cmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized command "{cmd}"')
        handler = self.message_handlers[cmd]
        handler(self, **kwargs)
        
    @handle("execute")
    def execute(self, *, code, uuid, interrupt_buffer):
        Execution(self, code, uuid, interrupt_buffer).run()

    @handle("complete")
    def complete(self, uuid, subcmd, **kwargs):
        if subcmd == "new_completer":
            self.completers[uuid] = Completer(self, uuid=uuid, **kwargs)
            return
        if uuid not in self.completers:
            raise Exception(f"No completer with uuid {uuid}")
        self.completers[uuid].handle_message(subcmd=subcmd, **kwargs)
