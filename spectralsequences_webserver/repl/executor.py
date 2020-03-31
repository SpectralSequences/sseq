from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
import os
import pathlib
from textwrap import dedent, indent

from message_passing_tree import Agent 
from message_passing_tree.decorators import collect_transforms, subscribe_to

@subscribe_to("*")
@collect_transforms(inherit = False)
class Executor(Agent):
    def __init__(self, globs=None, locs=None):
        super().__init__()
        if globs is None:
            globs = globals()
        if locals is None:
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
        return Executor.asyncify_WRAPPER_TEMPLATE.format(
            ASYNCIFY_WRAPPER_NAME=Executor.asyncify_WRAPPER_NAME, 
            usercode=indent(code, " " * 8)
        ) 

    asyncify_WRAPPER_NAME = '__async_def_wrapper__'
    asyncify_WRAPPER_LINE_NUMBER_OFFSET = 2 # Number of lines before user_code in ASYNCIFY_TEMPLATE
    asyncify_FIX_LINE_NUMBER_MARKER = "##FIX_LINE_NUMBER##"
    # Do not mess with indentation of WRAPPER_TEMPLATE.
    asyncify_WRAPPER_TEMPLATE = dedent(
            """
    async def {ASYNCIFY_WRAPPER_NAME}():
    {usercode}
            return locals() 
            """
        )

    def get_compiler_flags(self):
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    async def _execute(self, line: str) -> None:
        """
        Evaluate the line and print the result.
        """
        # Try eval first
        try:
            result = await self.eval_code(line)
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

    async def eval_code(self, line):
        code = self.compile_with_flags(line, "eval")
        result = eval(code, self.get_globals(), self.get_locals())
        return result

    async def exec_file(self, path : pathlib.Path, working_directory=None):
        await self.exec_code(
            path.read_text(), 
            working_directory, 
            Executor.asyncify_FIX_LINE_NUMBER_MARKER + str(path)
        )

    async def exec_code(self, lines, working_directory=None, file="<stdin>"):
        mod = Executor.asyncify(lines)
        async_wrapper_code = self.compile_with_flags(mod, 'exec', file)
        exec(async_wrapper_code, self.get_globals(), self.get_locals()) 
        do_the_thing = self.compile_with_flags(f"await {Executor.asyncify_WRAPPER_NAME}()", "eval")
        save_working_dir = os.getcwd()
        if working_directory is not None:
            os.chdir(working_directory)
        try:
            result = await eval(do_the_thing, self.get_globals(), self.get_locals())
        except:
            raise
        os.chdir(save_working_dir)
        self.get_globals().update(result)

    

