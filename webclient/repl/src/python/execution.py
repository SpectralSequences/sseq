import ast
import sys

from textwrap import dedent
from .traceback import Traceback
from .send_message import send_message
from .write_stream import WriteStream
from js import console
from contextlib import redirect_stdout, redirect_stderr, contextmanager
import crappy_multitasking as crappy_multitasking_module


@contextmanager
def crappy_multitasking(callback, interval):
    crappy_multitasking_module.set_interval(interval)
    crappy_multitasking_module.start(callback)
    try:
        yield None
    finally:
        crappy_multitasking_module.end()

def firstlinelen(s):
    res = s.find("\n") 
    if res == -1:
        return len(s)
    return res

def dedent_code(code):
    """ handle mis-indented input from multi-line strings """
    dedent_code = dedent(code)
    offset = firstlinelen(code) - firstlinelen(dedent_code)
    code=dedent_code
    return [code, offset]



class Execution:
    def __init__(self, executor, code, uuid, interrupt_buffer):
        self.executor = executor
        self.uuid = uuid
        [code, dedent_offset] = dedent_code(code)
        self.code = code
        self.dedent_offset = dedent_offset
        self.read_interrupt_buffer = interrupt_buffer
        self.check_interrupt_interval = 10_000

    def send_message(self, subcmd, *, last_response, **kwargs):
        from .executor import PyodideExecutor
        send_message("execute", self.uuid, subcmd=subcmd, last_response=last_response, **kwargs)

    def check_interrupt(self):
        if self.read_interrupt_buffer() == 0:
            return
        raise KeyboardInterrupt


    @staticmethod
    def adjust_ast(mod, code):
        expr = None

        if code[-1] == ";":
            return [mod, None]

        if isinstance(mod.body[-1], ast.Expr):
            expr = ast.Expression(mod.body[-1].value)
            del mod.body[-1]  
        elif isinstance(mod.body[-1], ast.Assign):
            from copy import deepcopy
            target = mod.body[-1].targets[0] # unclear if targets ever has length greater than 1?
            expr = ast.Expression(deepcopy(target))
            for x in ast.walk(expr):
                if hasattr(x, "ctx"):
                    x.ctx = ast.Load()
        if expr:
            ast.fix_missing_locations(expr)
        ast.fix_missing_locations(mod)
        return [mod, expr]
        
    
    def run(self):
        """
        Runs a string of code, the last part of which may be an expression.
        """

        try:
            mod = ast.parse(self.code)
        except SyntaxError as e:
            import parso
            r = parso.parse(self.code)
            errors = []
            for error in parso.load_grammar().iter_errors(r):
                erobj = dict(
                    start_pos=error.start_pos,
                    end_pos=error.end_pos,
                    msg=error.message
                )
                errors.append(erobj)
            self.send_syntax_errors(erobj)
            return
        self.send_syntax_is_valid()

        if len(mod.body) == 0:
            self.send_result(None)
            return

        [mod, expr] = Execution.adjust_ast(mod, self.code)

        try:
            flags = self.executor.flags
            ns = self.executor.namespace
            with \
              redirect_stdout(WriteStream(self.send_stdout_write)),\
              redirect_stderr(WriteStream(self.send_stderr_write)),\
              crappy_multitasking(self.check_interrupt, self.check_interrupt_interval):
                if len(mod.body):
                    exec(compile(mod, '<exec>', mode='exec', flags=flags), ns, ns)
                if expr is not None:
                    result = eval(compile(expr, '<eval>', mode='eval', flags=flags), ns, ns)
                    if result is not None:
                        self.send_result(repr(result))
                    else:
                        self.send_result(None)
                else:
                    self.send_result(None)
        except Exception as e:
            self.send_exception(e)
        except KeyboardInterrupt as e:
            self.send_keyboard_interrupt(e)


    def send_syntax_is_valid(self):
        self.send_message("validate_syntax", last_response=False, valid=True)

    def send_syntax_error(self, errors):
        self.send_message("validate_syntax",  last_response=True, 
            valid=False,
            errors=errors
        )

    
    def send_stdout_write(self, data):
        self.send_message("stdout", last_response=False, data=data)
    
    def send_stderr_write(self, data):
        self.send_message("stderr", last_response=False, data=data)


    def send_result(self, result):
        self.send_message("result", last_response=True, result=result)

    def send_exception(self, e):
        self.send_message("exception", last_response=True, traceback=Traceback.format_exception(e))

    def send_keyboard_interrupt(self, e):
        self.send_message("keyboard_interrupt", last_response=True)

    def format_stack_trace(self, e):
        pygments.highlight("def temp(x):\n return x*x+1", pygments.lexers.PythonTracebackLexer)