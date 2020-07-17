import ast
import sys
from textwrap import dedent
import traceback

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def eval_code(code, ns, flags=0):
    """
    Runs a string of code, the last part of which may be an expression.
    """
    # handle mis-indented input from multi-line strings
    code = dedent(code)

    mod = ast.parse(code)
    if len(mod.body) == 0:
        return [None, None]

    if isinstance(mod.body[-1], ast.Expr):
        expr = ast.Expression(mod.body[-1].value)
        del mod.body[-1]
    else:
        expr = None

    if len(mod.body):
        exec(compile(mod, '<exec>', mode='exec', flags=flags), ns, ns)
    if expr is not None:
        result = eval(compile(expr, '<eval>', mode='eval', flags=flags), ns, ns)
        return [result, repr(result)]
    else:
        return [None, None]

def validate_code(code, flags=0):
    code = dedent(code)
    try:
        compile(code, '<exec>', mode='exec', flags=flags)
        return None
    except SyntaxError as e:
        error = e
    except Exception as e:
        eprint("validate_code failed to catch exception of type", error.__class__.__name__)
        raise
    # traceback.clear_frames(error.__traceback__)
    # tb = traceback.format_tb(error.__traceback__)
    return error
