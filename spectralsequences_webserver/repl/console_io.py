# This file stolen in large part from ptpython/repl.py,
# with some other ideas from IPython and my own customizations 
# to exception handling.

from message_passing_tree.prelude import *

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
import traceback
from typing import Any, Dict
import os
import sys

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


class ConsoleIO(PythonRepl):
    def __init__(self, *a, **kw) -> None:
        self.BUFFER_STDOUT = True
        self.executor = None
        super().__init__(*a, **kw)

    async def run_a(self) -> None:
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

    async def _process_text_a(self, line: str) -> None:
        if line and not line.isspace():
            try:
                # Eval and print.
                await self.execute_a(line)
            except KeyboardInterrupt as e:  # KeyboardInterrupt doesn't inherit from Exception.
                self._handle_keyboard_interrupt(e)
            except Exception as e:
                self.app.output.write("\n")
                self._handle_exception(e)

            if self.insert_blank_line_after_output:
                self.app.output.write("\n")

            self.current_statement_index += 1
            self.signatures = []

    def get_compiler_flags(self):
        """ Make sure that "await f_a()" is not reported as a syntax error."""
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    async def execute_a(self, line: str) -> None:
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
                # Don't exec_code here because otherwise if another error occurs later,
                # the SyntaxError above would be printed in the stack trace.
                pass 
            await self.exec_code_a(line)
            output.flush()


    async def eval_code_a(self, *args, **kwargs):
        return await self.executor.eval_code_a(*args, **kwargs)

    async def exec_file_a(self, *args, **kwargs):
        return await self.executor.exec_file_a(*args, **kwargs)

    async def exec_code_a(self, *args, **kwargs):
        return await self.executor.exec_code_a(*args, **kwargs)

    def format_output(self, result):
        locals: Dict[str, Any] = self.get_locals()
        locals["_"] = locals["_%i" % self.current_statement_index] = result

        if result is None:
            return None
        else:
            out_prompt = self.get_output_prompt()

            try:
                result_str = "%r\n" % (repr(result),)
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

    def turn_on_buffered_stdout(self):
        self.BUFFER_STDOUT=True

    def turn_off_buffered_stdout(self):
        self.BUFFER_STDOUT=False

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
        # self.get_globals()["exception"] = exception
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

        self.executor.adjust_traceback(tb_summary_list)

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


    def _handle_keyboard_interrupt(self, e: KeyboardInterrupt) -> None:
        output = self.app.output
        output.write("\rKeyboardInterrupt\n\n")
        output.flush()


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

