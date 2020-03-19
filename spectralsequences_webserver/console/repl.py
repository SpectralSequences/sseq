import os
import sys

import ast
import asyncio
import logging
import types
from typing import overload
from textwrap import dedent, indent

from ptpython.repl import embed, PythonRepl
from ptpython.python_input import PythonInput

from prompt_toolkit.formatted_text import (
    FormattedText,
    PygmentsTokens,
    merge_formatted_text,
)
from prompt_toolkit.formatted_text.utils import fragment_list_width
from prompt_toolkit.shortcuts import print_formatted_text
from pygments.lexers import PythonLexer, PythonTracebackLexer

# from uvicorn.lifespan.on import shutdown

from decorators import monkey_patch
import utils

logger = logging.getLogger("hi")

def _lex_python_traceback(tb):
    " Return token list for traceback string. "
    lexer = PythonTracebackLexer()
    return lexer.get_tokens(tb)

def _lex_python_result(tb):
    " Return token list for Python string. "
    lexer = PythonLexer()
    return lexer.get_tokens(tb)


@monkey_patch(PythonInput)
def get_compiler_flags(self):
    return ast.PyCF_ALLOW_TOP_LEVEL_AWAIT
 
repl = None
def configure_repl(r):
    global repl
    repl = r 
    # utils.bind(repl.app, exit) 


def exit(
    self,
    result = None,
    exception = None,
    style: str = "", 
) -> None:
    assert result is None or exception is None

    if self.future is None:
        raise Exception("Application is not running. Application.exit() failed.")

    if self.future.done():
        raise Exception("Return value already set. Application.exit() failed.")

    
    self.exit_style = style

    if exception is not None:
        utils.print_error("We cannot really quit right now...")
        # shutdown()
        # self.future.set_exception(exception)
    else:
        self.future.set_result(result)

def shutdown():
    pass
    # os.system("""pgrep uvicorn | xargs kill -9""")
    # sys.exec()
    # for t in asyncio.Task.all_tasks():
    #     if str(t).find("Server") >= 0:
    #         task = t
    # task.cancel()

async def make_repl(globals, locals, **kwargs):
    try:
        await embed(globals, locals, return_asyncio_coroutine=True, patch_stdout=True, configure=configure_repl, **kwargs)
    except (EOFError, KeyboardInterrupt):
        print("Press ^C again...")
        shutdown()


@monkey_patch(PythonRepl)
async def run_async(self) -> None:
    while True:
        text = await self.app.run_async()
        await self._process_text(text)

@monkey_patch(PythonRepl)
async def _process_text(self, line: str) -> None:
    if line and not line.isspace():
        try:
            # Eval and print.
            await self._execute(line)
        except KeyboardInterrupt as e:  # KeyboardInterrupt doesn't inherit from Exception.
            self._handle_keyboard_interrupt(e)
        except Exception as e:
            self._handle_exception(e)

        if self.insert_blank_line_after_output:
            self.app.output.write("\n")

        self.current_statement_index += 1
        self.signatures = []

@monkey_patch(PythonRepl)
async def _execute(self, line: str) -> None:
    """
    Evaluate the line and print the result.
    """
    output = self.app.output

    # WORKAROUND: Due to a bug in Jedi, the current directory is removed
    # from sys.path. See: https://github.com/davidhalter/jedi/issues/1148
    if "" not in sys.path:
        sys.path.insert(0, "")

    if line.lstrip().startswith("\x1a"):
        # When the input starts with Ctrl-Z, quit the REPL.
        self.app.exit()
    elif line.lstrip().startswith("!"):
        # Run as shell command
        os.system(line[1:])
    else:
        # Try eval first
        try:
            result = await self.eval_code(line)
            formatted_output = self.format_output(result)
            self.print_formatted_output(formatted_output)
        # If not a valid `eval` expression, run using `exec` instead.
        except SyntaxError:
            await self.exec_code(line)
        output.flush()

@monkey_patch(PythonRepl)
def compile_with_flags(self, code: str, mode: str):
    " Compile code with the right compiler flags. "
    return compile(
        code,
        "<stdin>",
        mode,
        flags=self.get_compiler_flags(),
        dont_inherit=True,
    )

@monkey_patch(PythonRepl)
def compile_with_flags(self, code: str, mode: str):
    " Compile code with the right compiler flags. "
    return compile(
        code,
        "<stdin>",
        mode,
        flags=self.get_compiler_flags(),
        dont_inherit=True,
    )

@monkey_patch(PythonRepl)
async def eval_code(self, line):
    code = self.compile_with_flags(line, "eval")
    result = eval(code, self.get_globals(), self.get_locals())
    if asyncio.iscoroutine(result):
        return await result
    else:
        return result
 
@monkey_patch(PythonRepl)
async def exec_code(self, lines):
    mod = _asyncify(lines)
    async_wrapper_code = self.compile_with_flags(mod, 'exec')
    exec(async_wrapper_code, self.get_globals(), self.get_locals()) 
    do_the_thing = self.compile_with_flags("await __async_def_wrapper__()", "eval")
    try:
        result = await eval(do_the_thing, self.get_globals(), self.get_locals())
    except:
        print(sys.exc_info()[2].tb_frame)
        raise sys.exc_info()[1].with_traceback(sys.exc_info()[2].tb_next.tb_next)
        # raise
        # raise Exception from (exc_info[0], exc_info[1], exc_info[2].tb_next.tb_next)
    self.get_globals().update(result)


def _asyncify(code: str) -> str:
    """wrap code in async def definition.
    And setup a bit of context to run it later.
    """
    # Hood: do not mess with the indentation of this string. It will break.
    res = dedent(
        """
async def __async_def_wrapper__():
{usercode}
        return locals() 
        """
    ).format(usercode=indent(code, " " * 8)) 
    return res



@monkey_patch(PythonRepl)
def format_output(self, result):
    locals: Dict[str, Any] = self.get_locals()
    locals["_"] = locals["_%i" % self.current_statement_index] = result

    if result is None:
        return None
    else:
        out_prompt = self.get_output_prompt()

        try:
            result_str = "%r\n" % (result,)
        except UnicodeDecodeError:
            # In Python 2: `__repr__` should return a bytestring,
            # so to put it in a unicode context could raise an
            # exception that the 'ascii' codec can't decode certain
            # characters. Decode as utf-8 in that case.
            result_str = "%s\n" % repr(result).decode(  # type: ignore
                "utf-8"
            )

        # Align every line to the first one.
        line_sep = "\n" + " " * fragment_list_width(out_prompt)
        result_str = line_sep.join(result_str.splitlines()).strip("")

        # Write output tokens.
        if self.enable_syntax_highlighting:
            formatted_output = FormattedText(merge_formatted_text(
                [
                    out_prompt,
                    PygmentsTokens(list(_lex_python_result(result_str))),
                ]
            )())
            formatted_output.pop()
        else:
            formatted_output = FormattedText(
                out_prompt + [("", result_str)]
            )
        return formatted_output

@monkey_patch(PythonRepl)
def print_formatted_output(self, formatted_output):
    print_formatted_text(
        formatted_output,
        style=self._current_style,
        style_transformation=self.style_transformation,
        include_default_pygments_style=False,
    )

if __name__ == "__main__":
    pass