import asyncio
from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
import importlib
from inspect import iscoroutine, getsourcefile
import os
import pathlib

import traceback
import types
from typing import Any, List
from weakref import WeakSet

from message_passing_tree import Agent 
from message_passing_tree.decorators import collect_handlers, subscribe_to

from .. import config


class ExecResult:
    pass

def temp():
    class Ok(ExecResult):
        def __init__(self, result : Any):
            self.result = result

    class Err(ExecResult):
        def __init__(self, exc : List[str]):
            self.exception = exc

    class Interrupt(ExecResult):
        def __init__(self, exc : List[str]):
            self.exception = exc
    
    ExecResult.Ok = Ok
    ExecResult.Err = Err
    ExecResult.Interrupt = Interrupt

temp()
del temp


@subscribe_to("*")
@collect_handlers(inherit = False)
class Executor(Agent):
    namespace = None
    executors = WeakSet()

    def __init__(self, repl, globs=None, locs=None):
        Executor.executors.add(self)
        super().__init__()
        self.repl = repl
        self._initialize_scopes(globs, locs)
        self._initialize_namespace()
        self.globals["REPL"] = repl

    @staticmethod
    def ensure_global_namespace_is_initialized():
        if Executor.namespace is None:
            # Import this in here to avoid circular imports
            from .namespace import get_default_namespace
            # try:
            default_namespace = get_default_namespace()
            Executor.namespace = [ [value.__name__.split(".")[-1], value] for value in default_namespace ]
            # except Exception as e:
            #     from .console_io import ConsoleIO
            #     ConsoleIO.print_exception(None, e, buffered=False)


    @staticmethod
    def add_to_global_namespace(name, value=None):
        Executor.ensure_global_namespace_is_initialized()
        if value is None:
            value = name
            name = value.__name__.split(".")[-1]
        Executor.namespace.append([name, value])
    
    @staticmethod
    async def reload_channel_a(cls):
        from message_passing_tree.socket_close_codes import SERVICE_RESTART
        from ..server import serve
        await cls.close_all_channels_a(SERVICE_RESTART)
        cls = Executor.reload_class(cls)
        serve(cls)
        for executor in Executor.executors:
            executor.get_globals()[cls.__name__] = cls

    @staticmethod
    def reload_class(cls):
        from itertools import zip_longest
        module_path = cls.__module__.split(".")
        module = __import__(module_path[0])
        for name in module_path[1:]:
            module = getattr(module, name)
        reloaded_module = importlib.reload(module)
        return getattr(reloaded_module , cls.__name__)
    
    def _initialize_scopes(self, globs, locs):
        if globs is None:
            globs = {} 
        if locs is None:
            locs = globs
        self.globals = globs
        self.locals = locs

        def get_globals(): return self.globals
        def get_locals(): return self.locals

        self.get_globals = get_globals
        self.get_locals = get_locals

    def _initialize_namespace(self):
        Executor.ensure_global_namespace_is_initialized()
        globals = self.get_globals()
        for [name, value] in Executor.namespace:
            globals[name] = value
    
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
            self.repl.console_io.print_exception(result.exception, buffered = False)
        elif type(result) is ExecResult.Interrupt:
            print("KeyboardInterrupt 171571") # This doesn't happen very often...
        else:
            assert False

    async def exec_code_a(self, code : str, working_directory=None, file="<stdin>"):
        try:
            return ExecResult.Ok(await self.exec_code_unhandled_a(code, working_directory, file))
        except KeyboardInterrupt as e:  # KeyboardInterrupt doesn't inherit from Exception.
            return ExecResult.Interrupt(e)
        except Exception as e:
            e = self.exception_to_traceback_list(e, file)
            return ExecResult.Err(e)

    async def exec_code_unhandled_a(self, code : str, working_directory, file : str):
        tree = Executor.ast_get_last_expression(code) # Executor.asyncify(lines)
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
    def ast_get_last_expression(code : str):
        """ Modify code so that if the last statement is an "Expr" or "Await" statement, we return that into "EXEC-LAST-EXPRESSION" """
        from ast import (
            fix_missing_locations, parse, 
            Assign, Await, Constant, Expr, Name, Store
        )
        tree = parse(code)
        targets = [Name("EXEC-LAST-EXPRESSION", ctx = Store())]
        if isinstance(tree.body[-1], (Expr, Await)):
            tree.body[-1] = Assign(targets, tree.body[-1].value)
        else:
            tree.body.append(Assign(targets, Constant(None, None)))
        fix_missing_locations(tree)
        return tree

    @staticmethod
    def exception_to_traceback_list(exception, file : str) -> List[str]:
        exception_chain = [exception]
        while (exception := exception.__context__) is not None:
            exception_chain.append(exception)
        return [Executor.exception_to_traceback(e, file) for e in reversed(exception_chain)]
        
    @staticmethod
    def exception_to_traceback(exception, file : str) -> str:
        traceback.clear_frames(exception.__traceback__)
        tb_summary_list = list(traceback.extract_tb(exception.__traceback__))

        for line_number, tb_summary in enumerate(tb_summary_list):
            if tb_summary.filename == file:
                tb_summary_list = tb_summary_list[line_number:]
                break

        if hasattr(exception, "extra_traceback"):
            tb_summary_list.extend(exception.extra_traceback)

        l = traceback.format_list(tb_summary_list)
        if l:
            l.insert(0, "Traceback (most recent call last):\n")
        l.extend(traceback.format_exception_only(type(exception), exception))

        return "".join(l)     