from contextlib import redirect_stdout, redirect_stderr, contextmanager
from .write_stream import WriteStream
from .traceback import Traceback

from .util import contextmanager, set_interrupt_buffer, to_js

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from pyodide import CodeRunner
from js import releaseComlinkProxy


class Execution:
    def __init__(self, namespace, code, *, interrupt_buffer, stdout, stderr):
        self.namespace = namespace
        self.code = code
        self.interrupt_buffer = interrupt_buffer
        self.stdout = stdout
        self.stderr = stderr

    def dispose(self):
        if self.stdout:
            releaseComlinkProxy(self.stdout)
            self.stdout = None
        if self.stderr:
            releaseComlinkProxy(self.stderr)
            self.stderr = None

    def __del__(self):
        self.dispose()

    def validate_syntax(self):
        try:
            self.code_runner = CodeRunner(
                self.code, flags=PyCF_ALLOW_TOP_LEVEL_AWAIT
            ).compile()
        except SyntaxError:
            import parso

            r = parso.parse(self.code)
            errors = []
            for error in parso.load_grammar().iter_errors(r):
                error_dict = dict(
                    start_pos=error.start_pos, end_pos=error.end_pos, msg=error.message
                )
                errors.append(error_dict)
            self.dispose()
            return to_js(dict(valid=False, errors=errors))
        return to_js(dict(valid=True))

    @contextmanager
    def execution_context(self):
        with redirect_stdout(WriteStream(self.stdout)), redirect_stderr(
            WriteStream(self.stderr)
        ), set_interrupt_buffer(self.interrupt_buffer):
            yield

    async def run(self):
        """
        Runs a string of code, the last part of which may be an expression.
        """
        file = "<exec>"
        try:
            with self.execution_context():
                result = await self.code_runner.run_async(globals=self.namespace)
            result = repr(result) if result is not None else None
            return to_js(["success", result])
        except Exception as e:
            return to_js(["exception", Traceback.format_exception(e, file)])
        except KeyboardInterrupt:
            return to_js(["keyboard_interrupt", None])
        finally:
            self.dispose()

    # def format_stack_trace(self, e):
    #     # TODO...
    #     pygments.highlight("def temp(x):\n return x*x+1", pygments.lexers.PythonTracebackLexer)
