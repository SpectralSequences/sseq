import ast
import sys

from contextlib import redirect_stdout, redirect_stderr, contextmanager
from .write_stream import WriteStream
from asyncio import ensure_future

from textwrap import dedent
from .traceback import Traceback


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
    def __init__(self, executor, code, uuid):
        self.executor = executor
        self.uuid = uuid
        [code, dedent_offset] = dedent_code(code)
        self.code = code
        self.dedent_offset = dedent_offset


    async def send_message_a(self, subcmd, *, last_response, **kwargs):
        await self.executor.send_message_a("execute", self.uuid, subcmd=subcmd, last_response=last_response, **kwargs)

    def check_interrupt(self):
        if self.read_interrupt_buffer() == 0:
            return
        raise KeyboardInterrupt()
        
    @contextmanager
    def execution_context(self):
        from .executor import Executor
        saved_executor = Executor.executor
        with  redirect_stdout(WriteStream(self.send_stdout_write)),\
              redirect_stderr(WriteStream(self.send_stderr_write)):
            try:
                Executor.executor = self.executor
                yield
            finally:
                Executor.executor = saved_executor


    async def run_a(self):
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
                error_dict = dict(
                    start_pos=error.start_pos,
                    end_pos=error.end_pos,
                    msg=error.message
                )
                errors.append(error_dict)
            await self.send_syntax_errors_a(errors)
            return
        await self.send_syntax_is_valid_a()

        # If everything is reasonable then sys.exc_info() should be (None, None, None) here.
        # Sometimes there is a wasm stack overflow which leaves sys.exc_info() set when it should have been cleared.
        # Surprisingly these stack overflows don't seem to cause other harm.
        # Store exc_info ahead of time and don't report these stale trash exceptions as part of our stack trace.
        file = '<exec>'
        try:
            with self.execution_context():
                result = await self.executor.run_ast_a(self.code, mod, file)
            result = repr(result) if result is not None else None
            await self.send_result_a(result)
        except Exception as e:
            await self.send_exception_a(e, file)
        except KeyboardInterrupt as e:
            await self.send_keyboard_interrupt_a(e)


    async def send_syntax_is_valid_a(self):
        await self.send_message_a("validate_syntax", last_response=False, valid=True)

    async def send_syntax_errors_a(self, errors):
        await self.send_message_a("validate_syntax",  last_response=True, 
            valid=False,
            errors=errors
        )
    
    def send_stdout_write(self, data):
        coroutine = self.send_message_a("stdout", last_response=False, data=data)
        ensure_future(coroutine)
    
    def send_stderr_write(self, data):
        coroutine = self.send_message_a("stderr", last_response=False, data=data)
        ensure_future(coroutine)


    async def send_result_a(self, result):
        await self.send_message_a("result", last_response=True, result=result)

    async def send_exception_a(self, e, file):
        await self.send_message_a("exception", last_response=True, traceback=Traceback.format_exception(e, file))

    async def send_keyboard_interrupt_a(self, e):
        await self.send_message_a("keyboard_interrupt", last_response=True)

    # def format_stack_trace(self, e):
    #     # TODO...
    #     pygments.highlight("def temp(x):\n return x*x+1", pygments.lexers.PythonTracebackLexer)