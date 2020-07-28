import sys
import pyodide

from js import message_lookup, asyncCall

from .handler_decorator import *
from .completer import Completer
from .execution import Execution

def async_call(cmd, **kwargs):
    result = asyncCall(cmd, kwargs)
    if result is not None:
        return dict(result)

def sleep(duration):
    """ sleep for duration seconds (approximately) """
    async_call("sleep", duration=duration * 1000)

@collect_handlers("message_handlers")
class PyodideExecutor:
    def __init__(self, namespace=None, flags=0):
        self.namespace = namespace or {}
        self.namespace["async_call"] = async_call
        self.namespace["sleep"] = sleep;
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
    def execute(self, **kwargs):
        Execution(self, **kwargs).run()

    @handle("complete")
    def complete(self, uuid, subcmd, **kwargs):
        if subcmd == "new_completer":
            self.completers[uuid] = Completer(self, uuid=uuid, **kwargs)
            return
        if uuid not in self.completers:
            raise Exception(f"No completer with uuid {uuid}")
        self.completers[uuid].handle_message(subcmd=subcmd, **kwargs)
