from typing import List

class Traceback:
    @staticmethod
    def format_exception(exception, file : str):
        return Traceback.format_traceback_list(Traceback.exception_to_traceback_list(exception, file))

    @staticmethod
    def exception_to_traceback_list(exception, file : str) -> List[str]:
        exception_chain = [exception]
        while (exception := exception.__context__):
            exception_chain.append(exception)
        return [Traceback.exception_to_traceback(e, file) for e in reversed(exception_chain)]

    @staticmethod
    def format_traceback_list(tb_str_list : List[str]):
        result = ["", Traceback.format_traceback(tb_str_list[0])]
        for tb_str in tb_str_list[1:]:
            result.append("During handling of the above exception, another exception occurred:\n")
            result.append(Traceback.format_traceback(tb_str))
        return "\n".join(result)


    @staticmethod
    def format_traceback(tb_str : str):
        # import pygments
        # from pygments.lexers import PythonTracebackLexer
        # from pygments.formatters import HtmlFormatter
        # from pygments import highlight
        # return highlight(tb_str, PythonTracebackLexer(), HtmlFormatter())
        return tb_str

    @staticmethod
    def exception_to_traceback(exception, file : str) -> str:
        import traceback
        traceback.clear_frames(exception.__traceback__)
        tb_summary_list = list(traceback.extract_tb(exception.__traceback__))
        # Remove stack frames from the stack trace that come from the repl implementation.
        for line_number, tb_summary in enumerate(tb_summary_list):
            if tb_summary.filename == file:
                tb_summary_list = tb_summary_list[line_number:]
                break
        else:
            # Leave traceback unaltered.
            pass
            # raise Exception("What should happen here??")
        

        if hasattr(exception, "extra_traceback"):
            tb_summary_list.extend(exception.extra_traceback)

        l = traceback.format_list(tb_summary_list)
        if l:
            l.insert(0, "Traceback (most recent call last):\n")
        l.extend(traceback.format_exception_only(type(exception), exception))

        return "".join(l)   