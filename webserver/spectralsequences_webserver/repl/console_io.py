# This file stolen in large part from ptpython/repl.py,
# with some other ideas from IPython and my own customizations 
# to exception handling.

from message_passing_tree.prelude import *
from message_passing_tree import ansi
from .executor import ExecResult, Executor

from ast import PyCF_ALLOW_TOP_LEVEL_AWAIT
from typing import Any, Dict, List
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
            await self.execute_and_print_a(line)

            if self.insert_blank_line_after_output:
                self.app.output.write("\n")

            self.current_statement_index += 1
            self.signatures = []
    
    def exit(self):
        self.app.output.flush()
        self.app.exit()

    def get_compiler_flags(self):
        """ Make sure that "await f_a()" is not reported as a syntax error."""
        return PyCF_ALLOW_TOP_LEVEL_AWAIT

    async def execute_and_print_a(self, input: str) -> None:
        output = self.app.output

        # WORKAROUND: Due to a bug in Jedi, the current directory is removed
        # from sys.path. See: https://github.com/davidhalter/jedi/issues/1148
        if "" not in sys.path:
            sys.path.insert(0, "")

        if input.lstrip().startswith("\x1a"):
            # When the input starts with Ctrl-Z, quit the REPL.
            self.app.exit()
        elif input.lstrip().startswith("!"):
            # Run as shell command
            os.system(input[1:])
        else:
            result = await self.executor.exec_code_a(input)
            if type(result) is ExecResult.Ok:
                self.format_and_print_result(result.result)
            elif type(result) is ExecResult.Err:
                self.print_traceback_list(result.exception, buffered = False)
            elif type(result) is ExecResult.Interrupt:
                self._handle_keyboard_interrupt(result.exception)
            else:
                assert False

    def format_and_print_result(self, result):
        if result is None:
            return
        formatted_result = self.format_result(result)
        self.print_formatted_result(formatted_result)
        self.app.output.flush()

    def format_result(self, result):
        locals: Dict[str, Any] = self.get_locals()
        locals["_"] = locals["_%i" % self.current_statement_index] = result

        if result is None:
            return None
        else:
            out_prompt = self.get_output_prompt()

            result_str = f"{result}\n"
            line_width = 80
            truncation_lines = 15
            truncation_threshold = line_width * truncation_lines
            truncated = len(result_str) > truncation_threshold
            if truncated:
                head_result = result_str[:truncation_threshold] 
                tail_result = result_str[-line_width * 2 - 1:]
                result = [head_result, tail_result]
            else:
                result = [result_str]

            # Align every line to the first one.
            line_sep = "\n" + " " * fragment_list_width(out_prompt)
            result = [line_sep.join(result_str.splitlines()).strip("") for result_str in result]

            # Write output tokens.
            result_tokens = [PygmentsTokens(list(_lex_python_result(result_str))) for result_str in result]
            if truncated:
                # Remove first token of start text and last token of end text
                # in case they got weirdly truncated.
                result_tokens[0].token_list.pop(-2) 
                result_tokens[1].token_list.pop(0)
                result_tokens.insert(1, ANSI(ansi.highlight(" ... <Truncated Long Output> ...\n")))
            formatted_output = FormattedText(merge_formatted_text(
                [
                    out_prompt,
                    *result_tokens
                ]
            )())
            formatted_output.pop()
            return formatted_output

    def print_formatted_result(self, formatted_output):
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
            f"Error {ansi.ORANGE}{type}: {ansi.PINK}{msg}{ansi.NOCOLOR}\n"  +\
            additional_info
        ))

    def print_critical_error(self, type, msg, additional_info):
        self.print_formatted_text(ANSI(
            f"Critical Error {ansi.RED}{type}: {ansi.PINK}{msg}{ansi.NOCOLOR}\n"  +\
            additional_info
        ))

    def print_exception(self, exception, buffered = True):
        if type(exception) is list:
            tb_list = exception
        else:
            tb_list = Executor.exception_to_traceback_list(exception, "")
        self.print_traceback_list(tb_list)

    def print_traceback_list(self, tb_str_list : List[str], buffered = True):
        self.app.output.write("\n")
        self.print_traceback(tb_str_list[0], buffered)
        for tb_str in tb_str_list[1:]:
            self.print_formatted_text("During handling of the above exception, another exception occurred:\n", buffered)
            self.print_traceback(tb_str, buffered)


    def print_traceback(self, tb_str : str, buffered):
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

