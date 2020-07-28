from .send_message import send_message
from .handler_decorator import *
from uuid import uuid4
from collections import OrderedDict

class LRU(OrderedDict):
    'Limit size, evicting the least recently looked-up key when full'

    def __init__(self, maxsize=5, *args, **kwdargs):
        self.maxsize = maxsize
        super().__init__(*args, **kwdargs)

    def __getitem__(self, key):
        value = super().__getitem__(key)
        self.move_to_end(key)
        return value

    def __setitem__(self, key, value):
        if key in self:
            self.move_to_end(key)
        super().__setitem__(key, value)
        if len(self) > self.maxsize:
            oldest = next(iter(self))
            del self[oldest]

@collect_handlers("message_handlers")
class Completer:
    def __init__(self, executor, *, uuid):
        self.executor = executor
        self.uuid = uuid
        self.code = None
        self.states = LRU()

    def handle_message(self, subcmd, **kwargs):
        if subcmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized subcommand "{subcmd}"')
        handler = self.message_handlers[subcmd]
        handler(self, **kwargs)

    def send_message(self, subcmd, subuuid, **kwargs):
        send_message("complete", self.uuid, subcmd=subcmd, subuuid=subuuid, **kwargs)

    

    
    @handle("completions")
    def get_completions(self, subuuid, code, lineNumber, column):
        import jedi
        self.code = code
        state_id = str(uuid4())
        completions = jedi.Interpreter(code, [self.executor.namespace]) \
                        .complete(line=lineNumber, column=column - 1, fuzzy=True)
        self.states[state_id] = completions
        result = []
        for comp in completions:
            # docstring = comp.docstring(raw=True)
            result.append(dict(
                name=comp.name, 
                kind=comp.type
            ))
        self.send_message("completions", subuuid, completions=result, state_id=state_id)

    @handle("completion_detail")
    def get_completion_info(self, subuuid, state_id, idx):
        completion = self.states[state_id][idx]
        docstring = completion.docstring() or completion._get_docstring()
        signature = completion._get_docstring_signature()
        self.send_message("completion_detail", subuuid, docstring=docstring, signature=signature)
    

    