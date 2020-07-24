import ast
import sys
from textwrap import dedent
import pyodide
from js import (
    sendMessage as js_send_message, 
    message_lookup,
    Atomics,
    console
    # doio
)

from .traceback import Traceback
from .handler_decorator import *



# import micropip
# micropip.install("jedi>=0.17")
# micropip.install("spectralsequence_chart")

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def firstlinelen(s):
    res = s.find("\\n")
    if res == -1:
        return len(s)
    return res

def dedent_code(code):
    dedent_code = dedent(code)
    offset = firstlinelen(code) - firstlinelen(dedent_code)
    code=dedent_code
    return [code, offset]


class WriteStream:
    def __init__(self, write_handler):
        self.write_handler = write_handler
    
    def write(self, text):
        self.write_handler(text)

@collect_handlers("message_handlers")
class PyodideExecutor:
    def __init__(self, namespace=None, flags=0):
        self.namespace = namespace or {}
        self.flags = flags
        self.completers = {}

    @staticmethod
    def send_message(cmd, uuid, **kwargs):
        kwargs.update(cmd=cmd, uuid=uuid)
        js_send_message(kwargs)

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


class Execution:
    def __init__(self, executor, code, uuid, interrupt_buffer):
        self.executor = executor
        self.uuid = uuid
        [code, dedent_offset] = dedent_code(code)
        self.code = code
        self.dedent_offset = dedent_offset
        self.interrupt_buffer = interrupt_buffer

    def send_message(self, subcmd, *, last_response, **kwargs):
        PyodideExecutor.send_message("execute", self.uuid, subcmd=subcmd, last_response=last_response, **kwargs)



    def run(self):
        """
        Runs a string of code, the last part of which may be an expression.
        """

        sys.stdout = WriteStream(self.send_stdout_write)
        sys.stderr = WriteStream(self.send_stderr_write)
        # handle mis-indented input from multi-line strings

        try:
            mod = ast.parse(self.code)
        except SyntaxError as e:
            self.send_syntax_error(e)
            return
        self.send_syntax_is_valid()

        if len(mod.body) == 0:
            self.send_result(None)
            return

        if isinstance(mod.body[-1], ast.Expr):
            expr = ast.Expression(mod.body[-1].value)
            del mod.body[-1]
        else:
            expr = None

        flags = self.executor.flags
        ns = self.executor.namespace
        try:
            if len(mod.body):
                exec(compile(mod, '<exec>', mode='exec', flags=flags), ns, ns)
            if expr is not None:
                result = eval(compile(expr, '<eval>', mode='eval', flags=flags), ns, ns)
                self.send_result(repr(result))
            else:
                self.send_result(None)
        except Exception as e:
            self.send_exception(e)
            # raise
        except KeyboardInterrupt as e:
            self.send_keyboard_interrupt(e)
        # interrupt = self.interrupt_buffer()
        # if interrupt:
        #     console.log("Interrupt:", interrupt)
        # else:
        #     console.log("No interrupt")

            

    def send_syntax_is_valid(self):
        self.send_message("validate_syntax", last_response=False, valid=True)

    def send_syntax_error(self, error):
        self.send_message("validate_syntax",  last_response=True, 
            valid=False,
            error=dict(
                type= type(error).__name__,
                msg= error.msg, 
                lineno= error.lineno, 
                offset= error.offset + self.dedent_offset
            )
        )

    
    def send_stdout_write(self, data):
        self.send_message("stdout", last_response=False, data=data)
    
    def send_stderr_write(self, data):
        self.send_message("stderr", last_response=False, data=data)


    def send_result(self, result):
        # print("got result:", result)
        self.send_message("result", last_response=True, result=result)

    def send_exception(self, e):
        self.send_message("exception", last_response=True, traceback=Traceback.format_exception(e))

    def send_keyboard_interrupt(self, e):
        self.send_message("keyboard_interrupt", last_response=True)

    def format_stack_trace(self, e):
        pygments.highlight("def temp(x):\n return x*x+1", pygments.lexers.PythonTracebackLexer)


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
        PyodideExecutor.send_message("complete", self.uuid, subcmd=subcmd, subuuid=subuuid, **kwargs)


    @handle("set-code")
    def set_code(self, code):
        import jedi
        self.code = code
        self.interpreter = jedi.Interpreter(code, [self.executor.namespace])
        print("")
    
    @handle("completions")
    def get_completions(self, subuuid):
        if self.interpreter is None:
            raise Exception("No completions available, must call set_code first.")
        self.send_message("completions", subuuid, completions=[x.name for x in self.interpreter.completions()])

