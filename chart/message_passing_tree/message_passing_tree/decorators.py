import functools
import inspect
import sys
import traceback
from copy import copy

from . import ansi
from .agent import Agent


def reset_global_handlers():
    global HANDLERS
    HANDLERS = {
        "in": {},
        "out": {},
    }


reset_global_handlers()


def subscribe_to(subs):
    def helper(cls):
        if subs == "*":
            cls.subscriptions = {"*"}
        elif type(subs) is list:
            cls.subscriptions = set(subs)
        else:
            raise TypeError(
                f"""Subscribe decorator argument expected to be either "*" or a list, not "{subs}"."""
            )
        return cls

    return helper


def add_inherited_handlers(cls):
    outward_handlers = {}
    inward_handlers = {}
    for super in cls.__bases__:
        if hasattr(super, "outward_handlers") and super.outward_handlers is not None:
            outward_handlers.update(super.outward_handlers)
        if hasattr(super, "inward_handlers") and super.inward_handlers is not None:
            inward_handlers.update(super.inward_handlers)
    outward_handlers.update(cls.outward_handlers)
    inward_handlers.update(cls.inward_handlers)
    cls.outward_handlers = outward_handlers
    cls.inward_handlers = inward_handlers
    return cls


def collect_handlers(*, inherit):
    def helper(cls):
        cls.outward_handlers = HANDLERS["out"]
        cls.inward_handlers = HANDLERS["in"]
        reset_global_handlers()
        if inherit:
            add_inherited_handlers(cls)
        return cls

    return helper


def handle_inbound_messages(func):
    return handle("in")(func)


def handle_outbound_messages(func):
    return handle("out")(func)


handler_source_agent_argument_name = {
    "in": "source_agent_path",
    "out": "source_agent_id",
}


def declared_at(func):
    filename = inspect.getsourcefile(func)
    lineno = inspect.getsourcelines(func)[1]

    ctx = inspect.getframeinfo(inspect.stack()[3][0])
    try:
        cls = ctx.function
    finally:
        del ctx
    return (
        f""""{ansi.info(func.__name__)}" was declared:\n"""
        + f"""   in file "{ansi.info(filename)}"\n"""
        + f"""   in class "{ansi.info(cls)}"\n"""
        + f"""   on line {ansi.info(lineno)}"""
    )


def handle(in_or_out: str):
    if in_or_out not in HANDLERS:
        raise ValueError(
            f"""Second argument "in_or_out" should be "in" or "out" not "{in_or_out}"."""
        )

    def helper(func):
        colored_func_name = f"{ansi.info(func.__name__)}"
        func_args = inspect.getargspec(func).args
        second_argument_name = handler_source_agent_argument_name[in_or_out]

        def get_sample_declaration(colored_positions):
            subs = [ansi.INFO] * 6
            for i, pos in enumerate(["async", "self", "envelope"]):
                if pos in colored_positions:
                    subs[2 * i] = ansi.CORRECTION
            return (
                f""""{colored_func_name}" should be declared as"""
                + f""" "{ansi.INFO}%sasync%s def {func.__name__}(%sself%s, %senvelope%s, ...){ansi.NOCOLOR}"."""
                % tuple(subs)
            )

        if not inspect.iscoroutinefunction(func):
            raise TypeError(
                f"""Handler method "{colored_func_name}" """
                + f"""should be defined with the "{ansi.correction("async")}" keyword.\n"""
                + get_sample_declaration(["async"])
                + "\n"
                + declared_at(func)
                + "\n"
                + declared_at(func)
            )

        prefix = "handle__"
        suffix = "__a"
        if not func.__name__.startswith(prefix):
            raise TypeError(
                f"""Handler method name "{ansi.mistake(func.__name__)}" """
                + f"""should start with "{ansi.correction(prefix)}"."""
                + "\n"
                + declared_at(func)
            )

        if not func.__name__.endswith(suffix):
            raise TypeError(
                f"""Handler method name "{ansi.mistake(func.__name__)}" """
                + f"""should end with "{ansi.correction(suffix)}"."""
                + "\n"
                + declared_at(func)
            )

        if len(func_args) < 2:
            raise TypeError(
                f"""Handler method "{colored_func_name}" """
                + f"""should have at least two positional arguments.\n"""
                + get_sample_declaration(["self", "envelope"])
                + "\n"
                + declared_at(func)
            )
        if func_args[0] != "self":
            raise TypeError(
                f"""The first argument of handler method "{colored_func_name}" """
                + f"""should be named "{ansi.correction("self")}" not "{ansi.mistake(func_args[0])}".\n"""
                + get_sample_declaration(["self"])
                + "\n"
                + declared_at(func)
            )

        if func_args[1] != "envelope":
            raise TypeError(
                f"""The second argument of handler function "{colored_func_name}" """
                + f"""should be named "{ansi.correction("envelope")}" not "{ansi.mistake(func_args[1])}".\n"""
                + get_sample_declaration(["envelope"])
                + "\n"
                + declared_at(func)
            )

        handler_cmd = get_handler_cmd(func)
        wrapper = get_handler_wrapper(in_or_out, func)
        HANDLERS[in_or_out][handler_cmd] = wrapper
        return wrapper

    return helper


# Given a function named "handle__cmd__sub_cmd__a" return "cmd.sub_cmd"
def get_handler_cmd(func):
    prefix = "handle__"
    if not func.__name__.startswith(prefix):
        raise ValueError(
            f"""Method name {func.__name__} should start with "{prefix}"."""
        )
    suffix = "__a"
    if not func.__name__.endswith(suffix):
        raise ValueError(f"""Method name {func.__name__} should end with "{suffix}".""")
    result = func.__name__[len(prefix) : -len(suffix)].replace("__", ".")

    if result == "all":
        return "*"
    return result


def get_handler_wrapper(in_or_out, func_a):
    async def handler_wrapper_a(self, envelope):
        self.log_envelope_task(f"handle_{in_or_out}bound_method", envelope)
        try:
            await func_a(self, envelope, *envelope.msg.args, **envelope.msg.kwargs)
        except TypeError as e:
            add_wrapped_func_to_stack_trace_if_necessary(e, handler_wrapper_a, func_a)
            raise
        if in_or_out == "out":
            msg = envelope.msg
            new_msg = copy(msg)
            new_msg.cmd = copy(msg.cmd)
            envelope.msg = new_msg

    return handler_wrapper_a


class MockTraceback:
    def __init__(self, tb_frame, tb_lineno):
        self.tb_frame = tb_frame
        self.tb_lineno = tb_lineno
        self.tb_next = None


class MockFrame:
    def __init__(self, code):
        self.f_code = code
        self.f_globals = globals()


def add_wrapped_func_to_stack_trace_if_necessary(exception, wrapper, func):
    """If either the message is wrong or the argspec of the handler function is wrong,
    then we might get a TypeError reporting that the wrapped function has incorrect arguments.
    By default, the resulting stacktrace only mentions "func" leaving the identity of the wrapped
    function completely unclear.
    If there is an error
    """
    if traceback.extract_tb(exception.__traceback__)[-1].name != wrapper.__name__:
        return
    # exc_type, exc_instance, exc_traceback = exc_info
    filename = inspect.getsourcefile(func)
    lineno = inspect.getsourcelines(func)[1]
    exception.extra_traceback = traceback.extract_tb(
        MockTraceback(tb_lineno=lineno, tb_frame=MockFrame(func.__code__))
    )
