from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
import asyncio
# import logging
import pathlib
from textwrap import dedent, indent
import traceback
import types
from typing import Any, Callable, ContextManager, Dict, Optional
import os
import sys


from .. import utils

from prompt_toolkit import HTML, ANSI, print_formatted_text
from prompt_toolkit.document import Document
from prompt_toolkit.formatted_text import (
    Template,
    FormattedText,
    PygmentsTokens,
    merge_formatted_text,
)
from prompt_toolkit.formatted_text.utils import fragment_list_width
from pygments.lexers import PythonLexer, PythonTracebackLexer
from prompt_toolkit.patch_stdout import patch_stdout as patch_stdout_context
from ptpython.repl import PythonRepl
from prompt_toolkit.shortcuts import print_formatted_text

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

async def make_repl_a(
    globals=None,
    locals=None,
    configure_a: Optional[Callable] = None,
    vi_mode: bool = False,
    history_filename: Optional[str] = None,
    title: Optional[str] = None
):
    def get_globals():
        return globals

    def get_locals():
        return locals

    # Create REPL.
    repl = MyPythonRepl(
        get_globals=get_globals,
        get_locals=get_locals,
        vi_mode=vi_mode,
        history_filename=history_filename,
    )        

    if title:
        repl.terminal_title = title

    if configure_a:
        await configure_a(repl)

    # Start repl.
    patch_context : ContextManager = patch_stdout_context()
    async def coroutine_a():
        with patch_context:
            await repl.run_async_a()

    await coroutine_a()


def asyncify(code: str) -> str:
    """wrap code in async def definition.
    And setup a bit of context to run it later.
    """
    res = asyncify.WRAPPER_TEMPLATE.format(ASYNCIFY_WRAPPER_NAME=asyncify.WRAPPER_NAME, usercode=indent(code, " " * 8)) 
    return res

asyncify.WRAPPER_NAME = '__async_def_wrapper_a__'
asyncify.WRAPPER_LINE_NUMBER_OFFSET = 2 # Number of lines before user_code in ASYNCIFY_TEMPLATE
asyncify.FIX_LINE_NUMBER_MARKER = "##FIX_LINE_NUMBER##"
# Do not mess with indentation of WRAPPER_TEMPLATE.
asyncify.WRAPPER_TEMPLATE = dedent(
        """
async def {ASYNCIFY_WRAPPER_NAME}():
{usercode}
        return locals() 
        """
    )



class MyPythonRepl(PythonRepl):
    def __init__(self, *a, **kw) -> None:
        self.BUFFER_STDOUT = True        
        super().__init__(*a, **kw)

    def turn_on_buffered_stdout(self):
        self.BUFFER_STDOUT=True

    def turn_off_buffered_stdout(self):
        self.BUFFER_STDOUT=False

    def get_compiler_flags(self):
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    async def run_async_a(self) -> None:
        while True:
            try:
                text = await self.app.run_async()
            except EOFError:
                sys.exit(0)
            except KeyboardInterrupt:
                # Abort - try again.
                self.default_buffer.document = Document()
            else:
                await self._process_text_a(text)
            # except KeyboardInterrupt:
            #     pass

    async def _process_text_a(self, line: str) -> None:
        if line and not line.isspace():
            try:
                # Eval and print.
                await self._execute_a(line)
            except KeyboardInterrupt as e:  # KeyboardInterrupt doesn't inherit from Exception.
                self._handle_keyboard_interrupt(e)
            except Exception as e:
                self.app.output.write("\n")
                self._handle_exception(e)

            if self.insert_blank_line_after_output:
                self.app.output.write("\n")

            self.current_statement_index += 1
            self.signatures = []

    async def _execute_a(self, line: str) -> None:
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
                result = await self.eval_code_a(line)
                formatted_output = self.format_output(result)
                self.print_formatted_output(formatted_output)
                output.flush()
                return
            # If not a valid `eval` expression, run using `exec` instead.
            except SyntaxError:
                # Don't exec_codehere because otherwise if another error occurs later,
                # the SyntaxError above would be printed in the stack trace.
                pass 
            await self.exec_code_a(line)
            output.flush()

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
        return result

    async def exec_file_a(self, path : pathlib.Path, working_directory=None):
        await self.exec_code_a(path.read_text(), working_directory, asyncify.FIX_LINE_NUMBER_MARKER + str(path))

    async def exec_code_a(self, lines, working_directory=None, file="<stdin>"):
        mod = asyncify(lines)
        async_wrapper_code = self.compile_with_flags(mod, 'exec', file)
        exec(async_wrapper_code, self.get_globals(), self.get_locals()) 
        do_the_thing_a = self.compile_with_flags(f"await {asyncify.WRAPPER_NAME}()", "eval")
        save_working_dir = os.getcwd()
        if working_directory is not None:
            os.chdir(working_directory)
        try:
            result = await eval(do_the_thing_a, self.get_globals(), self.get_locals())
        except:
            raise
        os.chdir(save_working_dir)
        self.get_globals().update(result)

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

    def print_formatted_output(self, formatted_output):
        print_formatted_text(
            formatted_output,
            style=self._current_style,
            style_transformation=self.style_transformation,
            include_default_pygments_style=False,
        )

    def print_formatted_text(self, text, buffered=None, **kwargs):
        if ("file" in kwargs) or (buffered is False) or (buffered is None and not self.BUFFER_STDOUT):
            print_formatted_text(text, **kwargs)
        else:
            print_formatted_text(text, file=WrappedStdout(sys.stdout), **kwargs)


    def format_and_print_text(self, text):
        self.print_formatted_text(HTML(text))

    def print_info(self, type, msg):
        self.format_and_print_text("<green>" + str(msg) + "</green>")

    def print_warning(self, type, msg):
        self.format_and_print_text("<orange>" + str(msg) + "</orange>")

    def print_error(self, type, msg, additional_info):
        self.print_formatted_text(ANSI(
            f"Error {ANSI_ORANGE}{type}: {ANSI_PINK}{msg}{ANSI_NOCOLOR}\n"  +\
            additional_info
        ))

    def print_exception(self, exception):
        self.get_globals()["exception"] = exception
        self._handle_exception(exception, True)

    def _handle_exception(self, exception: Exception, buffered = False) -> None:
        exception_chain = [exception]
        while (exception := exception.__context__) is not None:
            exception_chain.append(exception)
        orig_exception = exception_chain.pop()
        self._handle_one_exception(orig_exception, buffered)
        for e in reversed(exception_chain):
            self.print_formatted_text("During handling of the above exception, another exception occurred:\n", buffered)
            self._handle_one_exception(e, buffered)
        


    def _handle_one_exception(self, exception: Exception, buffered = False) -> None:
        traceback.clear_frames(exception.__traceback__)
        tb_summary_list = list(traceback.extract_tb(exception.__traceback__))

        for line_number, tb_summary in enumerate(tb_summary_list):
            if tb_summary.filename == "<stdin>":
                tb_summary_list = tb_summary_list[line_number:]
                break

        for (i, tb_summary) in enumerate(tb_summary_list):
            if tb_summary.filename.startswith(asyncify.FIX_LINE_NUMBER_MARKER):
                tb_summary.filename = tb_summary.filename[len(asyncify.FIX_LINE_NUMBER_MARKER):]
                tb_summary.lineno -= asyncify.WRAPPER_LINE_NUMBER_OFFSET
                # We need to clear the cached "_line" so when we print the exception, 
                # Python will grab the line from the file again using the updated line number.
                tb_summary._line = None 

        if len(tb_summary_list) > 1 and tb_summary_list[1].name == asyncify.WRAPPER_NAME:
            tb_summary_list[1].name = tb_summary_list[0].name


        if hasattr(exception, "extra_traceback"):
            tb_summary_list.extend(exception.extra_traceback)


        self.get_globals()["exc"] = tb_summary_list
        # for tb_tuple in enumerate(tblist):


        l = traceback.format_list(tb_summary_list)
        if l:
            l.insert(0, "Traceback (most recent call last):\n")
        l.extend(traceback.format_exception_only(type(exception), exception))

        tb_str = "".join(l)

        # Format exception and write to output.
        # (We use the default style. Most other styles result
        # in unreadable colors for the traceback.)
        if self.enable_syntax_highlighting:
            tokens = list(_lex_python_traceback(tb_str))
        else:
            tokens = [(Token, tb_str)]

        self.print_formatted_text(
            PygmentsTokens(tokens),
            buffered,
            style=self._current_style,
            style_transformation=self.style_transformation,
            include_default_pygments_style=False,
        )



ANSI_RED = "\x1b[31m"
# ANSI_ORANGE = "\033[48:2:255:165:0m%s\033[m"

def ansi_color(r, g, b):
    return "\033[38;2;" +";".join([str(r), str(g), str(b)]) + "m"

ANSI_ORANGE = ansi_color(255, 0, 60)
ANSI_NOCOLOR = "\033[m"

ANSI_PINK = "\033[38;5;206m"

class WrappedStdout:
    """
    Proxy object for stdout which captures everything and prints output above
    the current application.
    """

    def __init__(
        self, inner
    ) -> None:
        self.inner = inner
        self.errors = self.inner.errors
        self.encoding = self.inner.encoding        

    def write(self, data: str) -> int:
        return self.inner.write(data.decode())

    def flush(self) -> None:
        return self.inner.flush()

    def fileno(self) -> int:
        return self.inner.fileno()

    def isatty(self) -> bool:
        return self.inner.isatty()