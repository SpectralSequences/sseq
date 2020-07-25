from .send_message import send_message
from .handler_decorator import *

@collect_handlers("message_handlers")
class Completer:
    def __init__(self, executor, *, uuid):
        self.executor = executor
        self.uuid = uuid
        self.code = None
        self.interpreter = None

    def handle_message(self, subcmd, **kwargs):
        if subcmd not in self.message_handlers:
            raise Exception(f'Message with unrecognized subcommand "{subcmd}"')
        handler = self.message_handlers[subcmd]
        handler(self, **kwargs)

    def send_message(self, subcmd, subuuid, **kwargs):
        send_message("complete", self.uuid, subcmd=subcmd, subuuid=subuuid, **kwargs)

    

    @handle("set-code")
    def set_code(self, code):
        import jedi
        self.code = code
        self.interpreter = jedi.Interpreter(code, [self.executor.namespace])
    
    @handle("completions")
    def get_completions(self, subuuid):
        if self.interpreter is None:
            raise Exception("No completions available, must call set_code first.")
        completions = []
        for comp in self.interpreter.completions():
            # docstring = comp.docstring(raw=True)
            completions.append(dict(
                name=comp.name, 
                kind=comp.type
            ))
        self.send_message("completions", subuuid, completions=completions)

    @handle("completion_detail")
    def get_completion_info(self, subuuid, idx):
        completion = self.interpreter.completions()[idx]
        docstring = completion.docstring(raw=True)
        signature = completion._get_docstring_signature()
        self.send_message("completion_detail", subuuid, docstring=docstring, signature=signature)
    

    