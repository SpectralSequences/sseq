import asyncio
from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from inspect import iscoroutine
import os
import pathlib
from textwrap import dedent, indent
import types

from message_passing_tree import Agent 
from message_passing_tree.decorators import collect_transforms, subscribe_to

from .. import config

class ExecResult:
    pass

def temp():
    class Ok(ExecResult):
        def __init__(self, result):
            self.result = result

    class Err(ExecResult):
        def __init__(self, exc):
            self.exception = exc

    class Interrupt(ExecResult):
        def __init__(self, exc):
            self.exception = exc
    
    ExecResult.Ok = Ok
    ExecResult.Err = Err
    ExecResult.Interrupt = Interrupt

temp()
del temp


@subscribe_to("*")
@collect_transforms(inherit = False)
class Executor(Agent):
    def __init__(self, console_io, globs=None, locs=None):
        super().__init__()
        self.console_io = console_io
        if globs is None:
            globs = {} 
        if locs is None:
            locs = globs
        self.globals = globs
        self.locals = locs

        def get_globals():
            return self.globals

        def get_locals():
            return self.locals

        self.get_globals = get_globals
        self.get_locals = get_locals
    
    def get_compiler_flags(self):
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    def compile_with_flags(self, code: str, mode: str, file = "<stdin>"):
        " Compile code with the right compiler flags. "
        return compile(
            code,
            file,
            mode,
            flags=self.get_compiler_flags(),
            # dont_inherit=True,
        )

    async def load_repl_init_file_if_it_exists_a(self):
        await self.exec_file_if_exists_a(config.REPL_INIT_FILE, working_directory=config.USER_DIR)

    async def exec_file_if_exists_a(self, path : pathlib.Path, working_directory=None):
        if path.is_file():
            await self.exec_file_a(path, working_directory)

    async def exec_file_a(self, path : pathlib.Path, working_directory=None):
        result = await self.exec_code_a(
            path.read_text(),
            working_directory,
            str(path)
        )
        if type(result) is ExecResult.Ok:
            pass
        elif type(result) is ExecResult.Err:
            self.console_io.print_exception(result.exception, buffered = False)
        elif type(result) is ExecResult.Interrupt:
            print("KeyboardInterrupt 171571")
        else:
            assert False

    async def exec_code_a(self, code_str, working_directory=None, file="<stdin>"):
        try:
            return ExecResult.Ok(await self.exec_code_unhandled_a(code_str))
        except KeyboardInterrupt as e:  # KeyboardInterrupt doesn't inherit from Exception.
            return ExecResult.Interrupt(e)
        except Exception as e:
            return ExecResult.Err(e)

    async def exec_code_unhandled_a(self, code_str, working_directory=None, file="<stdin>"):
        tree = Executor.ast_get_last_expression(code_str) # Executor.asyncify(lines)
        do_the_thing = self.compile_with_flags(tree, 'exec', file)
        save_working_dir = os.getcwd()
        if working_directory is not None:
            os.chdir(working_directory)
        try:
            res = eval(do_the_thing, self.get_globals(), self.get_locals())
            if asyncio.iscoroutine(res):
                await res
        finally:
            os.chdir(save_working_dir)
        return self.get_locals().pop("EXEC-LAST-EXPRESSION")
    
    @staticmethod
    def ast_get_last_expression(code_str):
        """ Modify code so that if the last statement is an "Expr" or "Await" statement, we return that into "EXEC-LAST-EXPRESSION" """
        from ast import (
            fix_missing_locations, parse, 
            Assign, Await, Constant, Expr, Name, Store
        )
        tree = parse(code_str)
        targets = [Name("EXEC-LAST-EXPRESSION", ctx = Store())]
        if isinstance(tree.body[-1], (Expr, Await)):
            tree.body[-1] = Assign(targets, tree.body[-1].value)
        else:
            tree.body.append(Assign(targets, Constant(None, None)))
        fix_missing_locations(tree)
        return tree