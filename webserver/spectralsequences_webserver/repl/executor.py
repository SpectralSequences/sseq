import asyncio
from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from inspect import iscoroutine
import os
import pathlib
from textwrap import dedent, indent

from message_passing_tree import Agent 
from message_passing_tree.decorators import collect_transforms, subscribe_to

from .. import config

@subscribe_to("*")
@collect_transforms(inherit = False)
class Executor(Agent):
    def __init__(self, globs=None, locs=None):
        super().__init__()
        if globs is None:
            globs = globals()
        if locs is None:
            locs = locals()
        self.globals = globs
        self.locals = locs

        def get_globals():
            return self.globals

        def get_locals():
            return self.locals

        self.get_globals = get_globals
        self.get_locals = get_locals
    
    @staticmethod
    def asyncify(code: str) -> str:
        """wrap code in async def definition.
        And set up a bit of context to run it later.
        """
        INDENT_SIZE = 4
        NUM_INDENTS = 3
        return Executor.asyncify_WRAPPER_TEMPLATE.format(
            ASYNCIFY_WRAPPER_NAME=Executor.asyncify_WRAPPER_NAME, 
            usercode=indent(code, " " * (INDENT_SIZE * NUM_INDENTS) )
        ) 

    asyncify_WRAPPER_NAME = '__async_def_wrapper_a__'
    asyncify_WRAPPER_LINE_NUMBER_OFFSET = 2 # Number of lines before user_code in ASYNCIFY_TEMPLATE
    asyncify_FIX_LINE_NUMBER_MARKER = "##FIX_LINE_NUMBER##"
    # Do not mess with indentation of WRAPPER_TEMPLATE.
    asyncify_WRAPPER_TEMPLATE = dedent(
            """
    async def {ASYNCIFY_WRAPPER_NAME}(result):
            try:
    {usercode}
            finally:
                result["locals"] = locals()
            """
        )

    def adjust_traceback(self, tb_summary_list):
        for (i, tb_summary) in enumerate(tb_summary_list):
            if tb_summary.filename.startswith(Executor.asyncify_FIX_LINE_NUMBER_MARKER):
                tb_summary.filename = tb_summary.filename[len(Executor.asyncify_FIX_LINE_NUMBER_MARKER):]
                tb_summary.lineno -= Executor.asyncify_WRAPPER_LINE_NUMBER_OFFSET
                # We need to clear the cached "_line" so when we print the exception, 
                # Python will grab the line from the file again using the updated line number.
                tb_summary._line = None 

        if len(tb_summary_list) > 1 and tb_summary_list[1].name == Executor.asyncify_WRAPPER_NAME:
            tb_summary_list[1].name = tb_summary_list[0].name


    def get_compiler_flags(self):
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    async def _execute_a(self, line: str) -> None:
        """
        Evaluate the line and print the result.
        """
        # Try eval first
        try:
            result = await self.eval_code_a(line)
            # Do something with result!
            return
            # If not a valid `eval` expression, run using `exec` instead.
        except SyntaxError:
                # Don't exec_codehere because otherwise if another error occurs later,
                # the SyntaxError above would be printed in the stack trace.
            pass 
        await self.exec_code(line)

    def compile_with_flags(self, code: str, mode: str, file = "<stdin>"):
        " Compile code with the right compiler flags. "
        return compile(
            code,
            file,
            mode,
            flags=self.get_compiler_flags(),
            dont_inherit=True,
        )

    async def eval_code_a(self, line):
        code = self.compile_with_flags(line, "eval")
        result = eval(code, self.get_globals(), self.get_locals())
        if iscoroutine(result):
            result = await result
        return result

    async def exec_file_if_exists_a(self, path : pathlib.Path, working_directory=None):
        if path.is_file():
            await self.exec_file_a(path, working_directory)

    async def load_repl_init_file_if_it_exists_a(self):
        await self.exec_file_if_exists_a(config.REPL_INIT_FILE, working_directory=config.USER_DIR)


    async def exec_file_a(self, path : pathlib.Path, working_directory=None):
        await self.exec_code_a(
            path.read_text(), 
            working_directory, 
            Executor.asyncify_FIX_LINE_NUMBER_MARKER + str(path)
        )

    async def exec_code_a(self, lines, working_directory=None, file="<stdin>"):
        mod = Executor.asyncify(lines)
        async_wrapper_code = self.compile_with_flags(mod, 'exec', file)
        exec(async_wrapper_code, self.get_globals(), self.get_locals()) 
        do_the_thing = self.compile_with_flags(f"await {Executor.asyncify_WRAPPER_NAME}(exec_result)", "eval")
        save_working_dir = os.getcwd()
        if working_directory is not None:
            os.chdir(working_directory)
        self.get_locals()["exec_result"] = {}
        try:
            await eval(do_the_thing, self.get_globals(), self.get_locals())
        finally:
            result = self.get_locals().pop("exec_result")
            self.get_globals().update(result["locals"])
            os.chdir(save_working_dir)

    

