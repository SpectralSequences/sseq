import ast
from asyncio import iscoroutine
from copy import deepcopy
import sys


from textwrap import dedent
from .async_js import wrap_promise
from .send_message import send_message
from .traceback import Traceback
from .write_stream import WriteStream

from js import console, is_promise
from contextlib import redirect_stdout, redirect_stderr, contextmanager
import crappy_multitasking as crappy_multitasking_module


@contextmanager
def crappy_multitasking(callback, interval):
    crappy_multitasking_module.set_interval(interval)
    # crappy_multitasking_module.start(callback)
    try:
        yield
    finally:
        pass
        # crappy_multitasking_module.end()

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
        raise KeyboardInterrupt()

    @contextmanager
    def execution_context(self):
        from .executor import PyodideExecutor
        saved_executor = PyodideExecutor.executor
        with  redirect_stdout(WriteStream(self.send_stdout_write)),\
              redirect_stderr(WriteStream(self.send_stderr_write)),\
              crappy_multitasking(self.check_interrupt, self.check_interrupt_interval):
            try:
                PyodideExecutor.executor = self.executor
                yield
            finally:
                PyodideExecutor.executor = saved_executor


    @staticmethod
    def adjust_ast(tree, code):
        target = [ast.Name("EXEC-LAST-EXPRESSION", ctx = ast.Store())]
        [tree, result] = Execution.get_ast_result(tree, code)
        tree.body.append(ast.Assign(target, result))
        ast.fix_missing_locations(tree)
        return tree
    
    @staticmethod
    def get_ast_result(tree, code):
        if code[-1] == ";":
            return [tree, ast.Constant(None, None)]

        last_node = tree.body[-1]
        if isinstance(last_node, (ast.Expr, ast.Await)):
            tree.body.pop()
            return [tree, last_node.value]

        if isinstance(last_node, ast.Assign):
            target = last_node.targets[0] # unclear if targets ever has length greater than 1?
            expr = deepcopy(target)
            for x in ast.walk(expr):
                if hasattr(x, "ctx"):
                    x.ctx = ast.Load()
            return [tree, expr]

        return [tree, ast.Constant(None, None)]



        
    
    async def run(self):
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

        mod = Execution.adjust_ast(mod, self.code)

        try:
            flags = self.executor.flags
            ns = self.executor.namespace
            with self.execution_context():
                res = eval(compile(mod, '<exec>', mode='exec', flags=flags), ns, ns)
                if iscoroutine(res):
                    await res
                result = ns.pop("EXEC-LAST-EXPRESSION")
                if result is not None:
                    self.send_result(repr(result))
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