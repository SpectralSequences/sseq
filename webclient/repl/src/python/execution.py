import ast
from asyncio import iscoroutine
from copy import deepcopy
import sys

from textwrap import dedent
from .traceback import Traceback
from .write_stream import WriteStream

from contextlib import redirect_stdout, redirect_stderr, contextmanager

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


    @staticmethod
    def adjust_ast_to_store_result(target_name, tree, code):
        """ Add instruction to store result of expression into a variable with name "target_name"  """
        target = [ast.Name(target_name, ctx = ast.Store())] 
        [tree, result] = Execution.adjust_ast_to_store_result_helper(tree, code)
        tree.body.append(ast.Assign(target, result))
        ast.fix_missing_locations(tree)
        return tree
    
    @staticmethod
    def adjust_ast_to_store_result_helper(tree, code):
        # If the raw source ends in a semicolon, supress the result.
        if code[-1] == ";":
            return [tree, ast.Constant(None, None)]

        # We directly wrap Expr or Await node in an Assign node.
        last_node = tree.body[-1]
        if isinstance(last_node, (ast.Expr, ast.Await)):
            tree.body.pop()
            return [tree, last_node.value]

        # If node is already an Assign, deep copy the lvalue of the Assign and store that structure
        # into our result.
        # This has the consequence that "[a, b] = (1,2)" returns "[1, 2]", while "a, b = (1,2)" returns "(1,2)".
        # This could be mildly unexpected behavior but it seems entirely harmless.
        if isinstance(last_node, ast.Assign):
            target = last_node.targets[0] # unclear if targets ever has length greater than 1?
            expr = deepcopy(target)
            # The deep copied expression was an lvalue but we are trying to use it as an rvalue.
            # Need to replace all the "Store" lvalue context markers with "Load" rvalue context markers.
            for x in ast.walk(expr):
                if hasattr(x, "ctx"):
                    x.ctx = ast.Load()
            return [tree, expr]
        # Remaining ast Nodes have no return value (not sure what other possibilities there are actually...)
        return [tree, ast.Constant(None, None)]

        
    
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

        if len(mod.body) == 0:
            await self.send_result_a(None)
            return

        # The string chosen is not a valid identifier to minimize chances of accidental collision with a user's variables.
        # Collision can only happen if they explicitly write to globals(), definitely not accidental...
        result_target = "EXEC-LAST-EXPRESSION"
        # we need to hand in the source string (self.code) just to check if it ends in a semicolon
        mod = Execution.adjust_ast_to_store_result(result_target, mod, self.code)
        file = '<exec>'
        # If everything is reasonable then sys.exc_info() should be (None, None, None) here.
        # Sometimes there is a wasm stack overflow which leaves sys.exc_info() set when it should have been cleared.
        # Surprisingly these stack overflows don't seem to cause other harm.
        # Store exc_info ahead of time and don't report these stale trash exceptions as part of our stack trace.
        trash_exception = sys.exc_info()[1]
        try:
            flags = self.executor.flags
            ns = self.executor.namespace
            with self.execution_context():
                res = eval(compile(mod, file, mode='exec', flags=flags), ns, ns)
                if iscoroutine(res):
                    await res
                result = ns.pop(result_target)
                if result is not None:
                    await self.send_result_a(repr(result))
                else:
                    await self.send_result_a(None)
        except Exception as e:
            await self.send_exception_a(e, file, trash_exception)
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
        self.executor.loop.call_soon(coroutine)
    
    def send_stderr_write(self, data):
        coroutine = self.send_message_a("stderr", last_response=False, data=data)
        self.executor.loop.call_soon(coroutine)


    async def send_result_a(self, result):
        await self.send_message_a("result", last_response=True, result=result)

    async def send_exception_a(self, e, file, trash_exception):
        await self.send_message_a("exception", last_response=True, traceback=Traceback.format_exception(e, file, trash_exception))

    async def send_keyboard_interrupt_a(self, e):
        await self.send_message_a("keyboard_interrupt", last_response=True)

    # def format_stack_trace(self, e):
    #     # TODO...
    #     pygments.highlight("def temp(x):\n return x*x+1", pygments.lexers.PythonTracebackLexer)