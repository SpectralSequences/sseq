from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
import asyncio
# import logging
import pathlib
from signal import SIGTERM
from textwrap import dedent, indent
import types
from typing import overload
import os
import sys


from ..decorators import monkey_patch
from .. import utils


from prompt_toolkit.formatted_text import (
    FormattedText,
    PygmentsTokens,
    merge_formatted_text,
)
from prompt_toolkit.formatted_text.utils import fragment_list_width
from prompt_toolkit.shortcuts import print_formatted_text
from ptpython.repl import embed, PythonRepl
from ptpython.python_input import PythonInput
from pygments.lexers import PythonLexer, PythonTracebackLexer

# logger = logging.getLogger("hi")

def _lex_python_traceback(tb):
    " Return token list for traceback string. "
    lexer = PythonTracebackLexer()
    return lexer.get_tokens(tb)

def _lex_python_result(tb):
    " Return token list for Python string. "
    lexer = PythonLexer()
    return lexer.get_tokens(tb)

def shutdown():
    pass

@monkey_patch(PythonInput)
def get_compiler_flags(self):
    return PyCF_ALLOW_TOP_LEVEL_AWAIT

async def make_repl(globals, locals, **kwargs):
    try:
        await embed(globals, locals, return_asyncio_coroutine=True, patch_stdout=True, **kwargs)
    except EOFError:
        sys.exit(0)
        
@monkey_patch(PythonRepl)
async def run_async(self) -> None:
    while True:
        try:
            text = await self.app.run_async()
            await self._process_text(text)
        except KeyboardInterrupt:
            pass

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
async def eval_code(self, line):
    code = self.compile_with_flags(line, "eval")
    result = eval(code, self.get_globals(), self.get_locals())
    if asyncio.iscoroutine(result):
        return await result
    else:
        return result

@monkey_patch(PythonRepl)
async def exec_file(self, path : pathlib.Path, working_directory=None):
    await self.exec_code(path.read_text(), working_directory)

@monkey_patch(PythonRepl)
async def exec_code(self, lines, working_directory=None):
    mod = _asyncify(lines)
    async_wrapper_code = self.compile_with_flags(mod, 'exec')
    exec(async_wrapper_code, self.get_globals(), self.get_locals()) 
    do_the_thing = self.compile_with_flags("await __async_def_wrapper__()", "eval")
    save_working_dir = os.getcwd()
    if working_directory is not None:
        os.chdir(working_directory)
    try:
        result = await eval(do_the_thing, self.get_globals(), self.get_locals())
    except:
        raise sys.exc_info()[1].with_traceback(sys.exc_info()[2].tb_next.tb_next)
        # raise
        # raise Exception from (exc_info[0], exc_info[1], exc_info[2].tb_next.tb_next)
    os.chdir(save_working_dir)
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