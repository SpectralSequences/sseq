from lark import Lark
_name_parser = Lark("""
    ?start : name
    name : var*

    var : gen _extras

    _extras : (sub _extras_sub) | (sup _extras_sup) | (prime _extras) | ()
    _extras_sub : (sup [prime]) | (prime _extras_sub) | ()
    _extras_sup : (sub [prime]) | (prime _extras_sup) | ()

    prime : "'" ((prime) | ())
    sub : "_" arg

    sup : "^" arg

    ?gen : gen_singleton | gen_macro | ( "(" " "* gen_singleton " "* ")" ) | ( "(" " "* gen_macro " "* ")" )| gen_parens 
    gen_singleton : /[A-Za-z]/
    gen_macro : /\\\\[A-Za-z]+/
    gen_parens : "(" /((\\\\[A-Za-z]+)|[\w^ ])+/ ")"

    ?arg : ("{" /[0-9]+/ "}") | /[0-9]+/

    %ignore " "
""")

from lark import Tree, Transformer
class EvalName(Transformer):
    def gen_parens(self, args):
        import re
        result = re.sub("\\\\[A-Za-z]*", "\g<0>!!!!", args[0])\
                   .replace(" ", "")\
                   .replace("!!!!", " ")\
                   .strip()
        return "(" + result + ")"

    def gen_singleton(self, args):
        return str(args[0]).strip()

    def gen_macro(self, args):
        return str(args[0]).strip()

    def sub(self, args):
        return ["add_to_var", f"_{{{args[0]}}}"]

    def sup(self,args):
        return ["sup", str(args[0])]

    def var(self, args):
        name = args[0]
        result = []
        for arg in sorted(args[1:]):
           if arg[0] == "add_to_var":
              name += arg[1]
           else:
              result.append(int(arg[1]))
        result = [name] + result
        if len(result) == 1:
            result.append(1)
        return result

    def prime(self, args):
        return ["add_to_var", "'"]

    def name(self, args):
        return args

_name_evaluator = EvalName()

def parse_name(name):
    t = _name_parser.parse(name)
    return reduce_monomial(_name_evaluator.transform(t))

def reduce_monomial(mono):
    result = {}
    for [k, v] in mono:
        result[k] = result.get(k, 0) + v
    return list(result.items())

def validate_name(name):
    from lark import LarkError
    err = None
    try:
        parse_name(name)
    except LarkError as e:
        err = {"name": type(e).__name__, "column" : getattr(e, "column", None)}
    return [err is None, err]

# Write x^n but handle special cases x^0 ==> 1 and x^1 ==> x
def power_name(var, n, zeroth_power=""):
    if n == 0:
        return zeroth_power
    elif n==1:
        return var
    else:
        # if var.find("'") > -1:
        #     var = f"({var})"
        return f"{var}^{{{n}}}"

def monomial_name(*exponents):
    result = " ".join(power_name(var, e) for [var,e] in exponents)
    if result == "":
        result = "1"
    return result

def add_monomials(mono1, mono2):
    d = dict(mono1)
    for [k, v] in mono2:
        d[k] = d.get(k, 0)
        d[k] += v
    return list(d.items())